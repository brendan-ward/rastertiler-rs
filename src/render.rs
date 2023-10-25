#![allow(clippy::too_many_arguments)]

use std::error::Error;
// use std::fs;
use std::path::PathBuf;

use anyhow::{anyhow, Result};
use crossbeam::channel;
// use gdal::spatial_ref::SpatialRef;
use gdal::raster::GdalDataType;
use indicatif::{ProgressBar, ProgressStyle};

// use crate::affine::Affine;
// use crate::dataset::{write_raster, Dataset};
use crate::dataset::Dataset;
use crate::mbtiles::MBTiles;
use crate::png::{ColormapEncoder, Encode, GrayscaleEncoder, RGBEncoder, Rgb8};
use crate::tileid::{TileID, TileRange};

pub fn render_tiles(
    tiff: &PathBuf,
    mbtiles: &PathBuf,
    minzoom: u8,
    maxzoom: u8,
    tilesize: u16,
    name: &Option<String>,
    description: &Option<String>,
    attribution: &Option<String>,
    workers: u8,
    colormap: &Option<String>,
    disable_overviews: bool,
) -> Result<()> {
    let dataset = Dataset::open(tiff, false).unwrap();
    let band = dataset.band(1).unwrap();
    let dtype = band.band_type();
    let geo_bounds = dataset.geo_bounds().unwrap();
    let mercator_bounds = dataset.mercator_bounds().unwrap();

    if colormap.is_some() && dtype != GdalDataType::UInt8 {
        return Err(anyhow!("colormap can only be provided for uint8 data"));
    }

    if !matches!(dtype, GdalDataType::UInt8 | GdalDataType::UInt32) {
        return Err(anyhow!(format!(
            "data type is not supported: {:}",
            dtype.name()
        )));
    }

    let mut metadata = Vec::<(&str, &str)>::new();

    // default tileset name to output filename
    let name = match name {
        Some(n) => n.to_owned(),
        None => String::from(mbtiles.file_stem().unwrap().to_str().unwrap()),
    };
    metadata.push(("name", &name));

    if description.is_some() {
        metadata.push(("description", description.as_ref().unwrap()));
    }

    if attribution.is_some() {
        metadata.push(("attribution", attribution.as_ref().unwrap()));
    }

    let minzoom_str = format!("{}", minzoom);
    let maxzoom_str = format!("{}", maxzoom);

    metadata.push(("minzoom", &minzoom_str));
    metadata.push(("maxzoom", &maxzoom_str));

    let bounds_str = format!(
        "{:.5},{:.5},{:.5},{:.5}",
        geo_bounds.xmin, geo_bounds.ymin, geo_bounds.xmax, geo_bounds.ymax
    );
    metadata.push(("bounds", &bounds_str));

    let center_str = format!(
        "{:.5},{:.5},{}",
        (geo_bounds.xmax + geo_bounds.xmin) / 2.,
        (geo_bounds.ymax + geo_bounds.ymin) / 2.,
        minzoom
    );
    metadata.push(("center", &center_str));

    metadata.push(("type", "overlay"));
    metadata.push(("format", "png"));
    metadata.push(("version", "1.0.0"));

    // close dataset; will be opened in each thread
    drop(dataset);

    // in a block so that connections are dropped to force flush / close
    {
        let db = MBTiles::new(mbtiles, workers).unwrap();
        db.set_metadata(&metadata).unwrap();

        let (snd, rcv) = channel::bounded(1);

        crossbeam::scope(|s| {
            // add tiles to queue
            s.spawn(|_| {
                let mut tiles: TileRange;
                for zoom in minzoom..(maxzoom + 1) {
                    tiles = TileRange::new(zoom, &mercator_bounds);
                    let bar = ProgressBar::new(tiles.count() as u64)
                        .with_style(ProgressStyle::default_bar().template(
                            "{prefix:<8} {bar:50} {pos}/{len} {msg} [elapsed: {elapsed_precise}]]",
                        ).unwrap())
                        .with_prefix(format!("zoom: {}", zoom));

                    for tile_id in tiles.iter() {
                        snd.send(tile_id).unwrap();
                        bar.inc(1);
                    }

                    bar.finish();
                }

                drop(snd);
            });

            let tiff = &tiff;
            let db = &db;
            let colormap = &colormap;
            for _ in 0..workers {
                let rcv = rcv.clone();

                s.spawn(move |_| {
                    match dtype {
                        GdalDataType::UInt8 => {
                            worker_u8(rcv, tiff, db, tilesize, colormap, disable_overviews)
                                .unwrap();
                        }
                        GdalDataType::UInt32 => {
                            worker_u32(rcv, tiff, db, tilesize, disable_overviews).unwrap();
                        }
                        // supported data types validated above
                        _ => {
                            unreachable!("data type not supported");
                        }
                    }
                });
            }
        })
        .unwrap();

        db.update_index().unwrap();
    }

    // change the database back to non-WAL mode
    MBTiles::flush(mbtiles).unwrap();

    Ok(())
}

fn worker_u8(
    tiles: channel::Receiver<TileID>,
    tiff_filename: &PathBuf,
    db: &MBTiles,
    tilesize: u16,
    colormap_str: &Option<String>,
    disable_overviews: bool,
) -> Result<(), Box<dyn Error>> {
    let dataset = Dataset::open(tiff_filename, disable_overviews)?;
    let vrt = dataset.mercator_vrt()?;
    let band = vrt.band(1)?;
    let nodata = band.no_data_value().unwrap() as u8;

    let conn = db.get_connection()?;

    let width: u32 = tilesize as u32;
    let height: u32 = width;

    let (has_colormap, encoder): (bool, Box<dyn Encode<u8>>) = match colormap_str {
        Some(c) => (
            true,
            Box::new(ColormapEncoder::<u8>::from_str(width, height, c, nodata).unwrap()),
        ),
        _ => (
            false,
            Box::new(GrayscaleEncoder::new(width, height, nodata)),
        ),
    };

    // create buffers to receive data; these are automatically filled with
    // the appropriate nodata value before reading from the raster
    let mut buffer = vec![0u8; tilesize as usize * tilesize as usize];

    let mut png_data: Vec<u8>;

    for tile_id in tiles.iter() {
        if vrt.read_tile(&band, tile_id, tilesize, &mut buffer, nodata)? {
            if has_colormap {
                png_data = encoder.encode(&buffer)?;
            } else {
                png_data = encoder.encode_8bit(&buffer)?;
            }
            db.write_tile(&conn, &tile_id, &png_data)?;
        }
    }

    Ok(())
}

fn worker_u32(
    tiles: channel::Receiver<TileID>,
    tiff_filename: &PathBuf,
    db: &MBTiles,
    tilesize: u16,
    disable_overviews: bool,
) -> Result<(), Box<dyn Error>> {
    let dataset = Dataset::open(tiff_filename, disable_overviews)?;
    let vrt = dataset.mercator_vrt()?;
    let band = vrt.band(1)?;
    let nodata = band.no_data_value().unwrap() as u32;

    let conn = db.get_connection()?;

    let width: u32 = tilesize as u32;
    let height: u32 = width;

    // get a function pointer for the RGBEncoder that defines specific type of data
    let rgb_encoder = RGBEncoder::new(width, height, nodata);
    let encode_rgb = <RGBEncoder as Encode<u32>>::encode_8bit;

    let mut colormap_encoder: ColormapEncoder<u32> =
        ColormapEncoder::new(width, height, nodata, 256)?;

    let buffer_size = tilesize as usize * tilesize as usize;
    let mut buffer = vec![nodata; buffer_size];
    let mut rgb_buffer: Vec<u8> = vec![0u8; buffer_size * 3];
    let mut color: Rgb8;
    let mut png_data: Vec<u8>;
    let mut use_palette: bool;

    for tile_id in tiles.iter() {
        if vrt.read_tile(&band, tile_id, tilesize, &mut buffer, nodata)? {
            // // DEBUG: write raw data to TIFF for inspection
            // let tile_bounds = tile_id.mercator_bounds();
            // let xres = (tile_bounds.xmax - tile_bounds.xmin) as f64 / tilesize as f64;
            // let yres = (tile_bounds.ymax - tile_bounds.ymin) as f64 / tilesize as f64;
            // let transform = Affine::new(xres, 0., tile_bounds.xmin, 0., -yres, tile_bounds.ymax);

            // write_raster(
            //     format!("/tmp/test_{}_{}_{}.tif", tile_id.zoom, tile_id.x, tile_id.y),
            //     tilesize as usize,
            //     tilesize as usize,
            //     &transform,
            //     &SpatialRef::from_epsg(3857)?,
            //     buffer.to_vec(),
            //     nodata as f64,
            // )
            // .unwrap();

            colormap_encoder.colormap.clear();
            use_palette = true;

            // convert value buffer to 8-bit RGB buffer, ignoring alpha
            // also build up palette of unique values
            for (i, &value) in buffer.iter().enumerate() {
                color = Rgb8::from_u32(value);
                rgb_buffer[i * 3] = color.r;
                rgb_buffer[i * 3 + 1] = color.g;
                rgb_buffer[i * 3 + 2] = color.b;

                if colormap_encoder.colormap.len() < 256 {
                    colormap_encoder.colormap.add_color(value, color);
                } else {
                    use_palette = false;
                }
            }

            if use_palette {
                png_data = colormap_encoder.encode(&buffer)?;
            } else {
                png_data = encode_rgb(&rgb_encoder, &rgb_buffer)?;
            }

            db.write_tile(&conn, &tile_id, &png_data)?;

            // DEBUG: write rendered PNG to file
            // fs::write(
            //     format!("/tmp/test_{}_{}_{}.png", tile_id.zoom, tile_id.x, tile_id.y),
            //     png_data,
            // )
            // .unwrap();
        }
    }

    Ok(())
}

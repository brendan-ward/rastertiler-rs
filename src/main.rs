use std::error::Error;
use std::fs;
use std::path::PathBuf;

use clap::{CommandFactory, ErrorKind, Parser};
use crossbeam::channel;
use gdal::raster::GDALDataType;
use gdal::spatial_ref::SpatialRef;
use indicatif::{ProgressBar, ProgressStyle};

mod affine;
mod array;
mod bounds;
mod dataset;
mod mbtiles;
mod png;
mod tileid;
mod window;

use crate::affine::Affine;
use crate::dataset::{write_raster, Dataset};
use crate::mbtiles::MBTiles;
use crate::png::{ColormapEncoder, Encode, GrayscaleEncoder, RGBEncoder, Rgb8};
use crate::tileid::{TileID, TileRange};

#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Cli {
    #[clap(parse(try_from_str=file_exists))]
    /// Input GeoTIFF filename
    tiff: PathBuf,

    /// Output MBTiles filename
    mbtiles: PathBuf,

    /// Minimum zoom level
    #[clap(short = 'Z', long, default_value_t = 0, parse(try_from_str=parse_zoom))]
    minzoom: u8,

    /// Maximum zoom level
    #[clap(short = 'z', long, default_value_t = 0, parse(try_from_str=parse_zoom))]
    maxzoom: u8,

    /// Tile size in pixels per side
    #[clap(short = 's', long, default_value_t = 512)]
    tilesize: u16,

    /// Tileset name
    #[clap(short = 'n', long)]
    name: Option<String>,

    /// Tileset description
    #[clap(short = 'd', long)]
    description: Option<String>,

    /// Minimum zoom level
    #[clap(short = 'a', long)]
    attribution: Option<String>,

    /// Number of workers to create tiles
    #[clap(short = 'w', long, default_value_t = 4)]
    workers: u8,

    /// Colormap as comma-delmited value:hex color pairs, e.g., "<value>:<hex>,<value:hex>"
    /// can only be provided for uint8 data
    #[clap(short = 'c', long)]
    colormap: Option<String>,

    /// Disable use of overviews in source GeoTIFF. This will yield more precise
    /// results at the expense of slower performance
    #[clap(long, action)]
    disable_overviews: bool,
}

fn main() {
    let args = Cli::parse();

    println!(
        "Call: {:?} ({}) {:?}, name:{:?}, zooms: {}-{}",
        args.tiff,
        args.tiff.exists(),
        args.mbtiles,
        args.name,
        args.minzoom,
        args.maxzoom
    );

    if args.minzoom > args.maxzoom {
        let mut cmd = Cli::command();
        cmd.error(
            ErrorKind::ArgumentConflict,
            "minzoom must be less than maxzoom",
        )
        .exit();
    }

    // default tileset name to output filename
    let name = args.name.unwrap_or(String::from(
        args.mbtiles.file_stem().unwrap().to_str().unwrap(),
    ));

    let dataset = Dataset::open(&args.tiff, false).unwrap();
    let band = dataset.band(1).unwrap();
    let dtype = band.band_type();
    let geo_bounds = dataset.geo_bounds().unwrap();
    let mercator_bounds = dataset.mercator_bounds().unwrap();

    // colormap is only allowed for u8 data
    if args.colormap.is_some() && dtype != GDALDataType::GDT_Byte {
        let mut cmd = Cli::command();
        cmd.error(
            ErrorKind::ArgumentConflict,
            "colormap can only be provided for uint8 data",
        )
        .exit();
    }

    let allowed_dtype: bool = match dtype {
        GDALDataType::GDT_Byte => true,
        GDALDataType::GDT_UInt32 => true,
        _ => false,
    };

    if !allowed_dtype {
        let mut cmd = Cli::command();
        cmd.error(ErrorKind::ArgumentConflict, "data type is not supported")
            .exit();
    }

    let mut metadata = Vec::<(&str, &str)>::new();
    metadata.push(("name", &name));

    if args.description.is_some() {
        metadata.push(("description", args.description.as_ref().unwrap()));
    }

    if args.attribution.is_some() {
        metadata.push(("attribution", args.attribution.as_ref().unwrap()));
    }

    let minzoom_str = format!("{}", args.minzoom);
    let maxzoom_str = format!("{}", args.maxzoom);

    metadata.push(("minzoom", &minzoom_str));
    metadata.push(("maxzoom", &maxzoom_str));

    let bounds_str = format!(
        "{:.5},{:.5},{:.5},{:.5}",
        geo_bounds.xmin, geo_bounds.ymin, geo_bounds.xmax, geo_bounds.ymax
    );
    metadata.push(("bounds", &bounds_str));

    let center_str = format!(
        "{:.5},{:.5},{}",
        (geo_bounds.xmax - geo_bounds.xmin) / 2.,
        (geo_bounds.ymax - geo_bounds.ymin) / 2.,
        args.minzoom
    );
    metadata.push(("center", &center_str));

    metadata.push(("type", "overlay"));
    metadata.push(("format", "png"));
    metadata.push(("version", "1.0.0"));

    // close dataset; will be opened in each thread
    drop(dataset);

    // in a block so that connections are dropped to force flush / close
    {
        let db = MBTiles::new(&args.mbtiles, args.workers).unwrap();
        db.set_metadata(&metadata).unwrap();

        let (snd, rcv) = channel::bounded(1);

        crossbeam::scope(|s| {
            // add tiles to queue
            s.spawn(|_| {
                let mut tiles: TileRange;
                for zoom in args.minzoom..(args.maxzoom + 1) {
                    tiles = TileRange::new(zoom, &mercator_bounds);
                    let bar = ProgressBar::new(tiles.count() as u64)
                        .with_style(ProgressStyle::default_bar().template(
                            "{prefix:<8} {bar:50} {pos}/{len} {msg} [elapsed: {elapsed_precise}]]",
                        ))
                        .with_prefix(format!("zoom: {}", zoom));

                    for tile_id in tiles.iter() {
                        snd.send(tile_id).unwrap();
                        bar.inc(1);
                    }

                    bar.finish();
                }

                drop(snd);
            });

            let tiff = &args.tiff;
            let db = &db;
            let colormap = &args.colormap;
            for _ in 0..args.workers {
                let rcv = rcv.clone();

                s.spawn(move |_| {
                    match dtype {
                        GDALDataType::GDT_Byte => {
                            worker_u8(
                                rcv,
                                tiff,
                                db,
                                args.tilesize,
                                colormap,
                                args.disable_overviews,
                            )
                            .unwrap();
                        }
                        GDALDataType::GDT_UInt32 => {
                            worker_u32(rcv, tiff, db, args.tilesize, args.disable_overviews)
                                .unwrap();
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

        db.close().unwrap();
    }

    // change the database back to non-WAL mode
    MBTiles::flush(&args.mbtiles).unwrap();
}

fn file_exists(s: &str) -> Result<PathBuf, String> {
    let mut path = PathBuf::new();
    path.push(s);

    if !path.exists() {
        return Err(String::from("path does not exist"));
    }
    Ok(path)
}

fn parse_zoom(s: &str) -> Result<u8, String> {
    let zoom = s
        .parse()
        .map_err(|_| format!("`{}` isn't a valid number", s))?;
    if zoom > 24 {
        return Err(String::from("must be no greater than 24"));
    }
    return Ok(zoom);
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
            Box::new(ColormapEncoder::<u8>::from_str(width, height, &c, nodata).unwrap()),
        ),
        _ => (
            false,
            Box::new(GrayscaleEncoder::new(width, height, nodata)),
        ),
    };

    // create buffers to receive data; these are automatically filled with
    // the appropriate nodata value before reading from the raster
    let mut buffer = vec![0u8; (tilesize as usize * tilesize as usize) as usize];

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

    let buffer_size = (tilesize as usize * tilesize as usize) as usize;
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

#[cfg(test)]
mod test {
    use crate::affine::Affine;
    use crate::bounds::Bounds;

    pub fn approx_eq(l: f64, r: f64, precision: f64) -> bool {
        (l - r).abs() < precision
    }

    pub fn approx_equal_affine(l: &Affine, r: &Affine, precision: f64) -> bool {
        approx_eq(l.a, r.a, precision)
            && approx_eq(l.b, r.b, precision)
            && approx_eq(l.c, r.c, precision)
            && approx_eq(l.d, r.d, precision)
            && approx_eq(l.e, r.e, precision)
            && approx_eq(l.f, r.f, precision)
    }

    pub fn approx_eq_bounds(l: &Bounds, r: &Bounds, precision: f64) -> bool {
        approx_eq(l.xmin, r.xmin, precision)
            && approx_eq(l.ymin, r.ymin, precision)
            && approx_eq(l.xmax, r.xmax, precision)
            && approx_eq(l.ymax, r.ymax, precision)
    }
}

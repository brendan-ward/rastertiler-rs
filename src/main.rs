use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;

use clap::{CommandFactory, ErrorKind, Parser};
use crossbeam::channel;
use gdal::raster::GDALDataType;
use indicatif::{ProgressBar, ProgressStyle};

mod affine;
mod array;
mod bounds;
mod color;
mod dataset;
mod mbtiles;
mod png;
mod tileid;
mod window;

use crate::dataset::Dataset;
use crate::mbtiles::MBTiles;
use crate::png::{ColormapEncoder, Encode, GrayscaleEncoder};
use crate::tileid::{TileID, TileRange};

#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Cli {
    #[clap(parse(try_from_str=file_exists))]
    tiff: PathBuf,
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
    #[clap(short = 'c', long)]
    colormap: Option<String>,
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

    let dataset = Dataset::open(&args.tiff).unwrap();
    let band = dataset.band(1).unwrap();
    let dtype = band.band_type();
    let geo_bounds = dataset.geo_bounds().unwrap();
    let mercator_bounds = dataset.mercator_bounds().unwrap();

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

    let encoder: Arc<dyn Encode + Send + Sync> = match dtype {
        GDALDataType::GDT_Byte => match args.colormap {
            Some(c) => Arc::new(
                ColormapEncoder::new(args.tilesize as u32, args.tilesize as u32, &c).unwrap(),
            ),
            _ => Arc::new(GrayscaleEncoder::new(
                args.tilesize as u32,
                args.tilesize as u32,
                band.no_data_value().unwrap() as u8,
            )),
        },
        _ => panic!("Data type not  supported: {:?}", dtype),
    };

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

            for _ in 0..args.workers {
                let rcv = rcv.clone();
                let tiff = &args.tiff;
                let db = &db;
                let encoder = encoder.clone();
                s.spawn(move |_| {
                    worker(rcv, tiff, db, args.tilesize, &encoder).unwrap();
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

fn worker(
    tiles: channel::Receiver<TileID>,
    tiff_filename: &PathBuf,
    db: &MBTiles,
    tilesize: u16,
    encoder: &Arc<dyn Encode + Send + Sync>,
) -> Result<(), Box<dyn Error>> {
    let dataset = Dataset::open(tiff_filename)?;
    let vrt = dataset.mercator_vrt()?;
    let band = vrt.band(1)?;

    let conn = db.get_connection()?;

    // TODO: figure out how to make this dynamic with respect to dtype
    let nodata = band.no_data_value().unwrap() as u8;

    let mut buffer = match band.band_type() {
        GDALDataType::GDT_Byte => {
            vec![nodata as u8; (tilesize as usize * tilesize as usize) as usize]
        }
        // GDALDataType::GDT_UInt16 => {
        //     vec![nodata as u16; (args.tilesize * args.tilesize) as usize]
        // }
        _ => panic!("Data type not  supported: {:?}", band.band_type()),
    };

    let mut has_data: bool;
    for tile_id in tiles.iter() {
        has_data = vrt.read_tile(&band, tile_id, tilesize, &mut buffer, nodata)?;

        if has_data {
            let png_data = encoder.encode(&buffer)?;
            db.write_tile(&conn, &tile_id, &png_data)?;

            std::fs::write(
                format!("/tmp/test_{}_{}_{}.png", tile_id.zoom, tile_id.x, tile_id.y),
                png_data,
            )
            .unwrap();
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

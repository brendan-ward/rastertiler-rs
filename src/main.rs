use clap::{CommandFactory, ErrorKind, Parser};
use std::path::PathBuf;

mod mbtiles;
use crate::mbtiles::MBTiles;

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

    // TODO: parse / validate colormap

    let db = MBTiles::new(&args.mbtiles, args.workers).unwrap();

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

    // TODO: calculate bounds and set in metadata

    db.set_metadata(&metadata).unwrap();

    // TODO: lots of processing

    db.close().unwrap();
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

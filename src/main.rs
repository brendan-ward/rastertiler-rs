use std::path::PathBuf;

use anyhow::Result;
use clap::error::ErrorKind;
use clap::{CommandFactory, Parser, Subcommand};

mod affine;
mod array;
mod bounds;
mod dataset;
mod mbtiles;
mod png;
mod render;
mod tileid;
mod window;

use crate::mbtiles::merge;
use crate::render::render_tiles;

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[command(about = "merge two MBTiles files into a single MBTiles file")]
    Merge {
        #[arg(name="left MBTiles file", value_parser=file_exists)]
        left: PathBuf,

        #[arg(name="right MBTiles file", value_parser=file_exists)]
        right: PathBuf,

        #[arg(name = "output MBTiles file")]
        out: PathBuf,
    },
    #[command(about = "render a single-band GeoTIFF to a MBTiles file")]
    Render {
        #[arg(value_parser=file_exists)]
        /// Input GeoTIFF filename
        tiff: PathBuf,

        /// Output MBTiles filename
        mbtiles: PathBuf,

        /// Minimum zoom level
        #[clap(short = 'Z', long, default_value_t = 0, value_parser=parse_zoom)]
        minzoom: u8,

        /// Maximum zoom level
        #[clap(short = 'z', long, default_value_t = 0, value_parser=parse_zoom)]
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
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match &cli.command {
        Commands::Merge { left, right, out } => {
            merge(left, right, out)?;
        }
        Commands::Render {
            tiff,
            mbtiles,
            minzoom,
            maxzoom,
            tilesize,
            name,
            description,
            attribution,
            workers,
            colormap,
            disable_overviews,
        } => {
            if minzoom > maxzoom {
                let mut cmd = Cli::command();
                cmd.error(
                    ErrorKind::ArgumentConflict,
                    "minzoom must be less than maxzoom",
                )
                .exit();
            }

            render_tiles(
                tiff,
                mbtiles,
                *minzoom,
                *maxzoom,
                *tilesize,
                name,
                description,
                attribution,
                *workers,
                colormap,
                *disable_overviews,
            )?;
        }
    }

    Ok(())
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
    Ok(zoom)
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

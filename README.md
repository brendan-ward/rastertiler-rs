# rastertiler-rs

A tool to create a PNG MBtiles tileset from a single-band GeoTIFF.

Requires GDAL >= 3.4 to be installed on the system.

## WARNING

This is still under early development and a lot of validation is not yet in place.

## Installation

### Development

From a local clone of this repository

Build and run this in development mode:

```bash
cargo run -- render <tif filename> <mbtiles filename>
```

Or build a release version:

```bash
cargo build --release
```

## Usage

```bash
Usage: rastertiler <COMMAND>

Commands:
  merge   merge two MBTiles files into a single MBTiles file
  render  render a single-band GeoTIFF to a MBTiles file
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

### Render tiles

```bash
Usage: rastertiler render [OPTIONS] <TIFF> <MBTILES>

Arguments:
  <TIFF>     Input GeoTIFF filename
  <MBTILES>  Output MBTiles filename

Options:
  -Z, --minzoom <MINZOOM>          Minimum zoom level [default: 0]
  -z, --maxzoom <MAXZOOM>          Maximum zoom level [default: 0]
  -s, --tilesize <TILESIZE>        Tile size in pixels per side [default: 512]
  -n, --name <NAME>                Tileset name
  -d, --description <DESCRIPTION>  Tileset description
  -a, --attribution <ATTRIBUTION>  Minimum zoom level
  -w, --workers <WORKERS>          Number of workers to create tiles [default: 4]
  -c, --colormap <COLORMAP>        Colormap as comma-delmited value:hex color pairs, e.g., "<value>:<hex>,<value:hex>" can only be provided for uint8 data
      --disable-overviews          Disable use of overviews in source GeoTIFF. This will yield more precise results at the expense of slower performance
  -h, --help                       Print help
```

To create MBtiles from a single-band `uint8` GeoTIFF:

```bash
rastertiler render example.tif example.mbtiles --minzoom 0 --maxzoom 2
```

By default, this will render grayscale PNG tiles.

To use a colormap to render the `uint8` data to paletted PNG

```bash
rastertiler render example.tif example.mbtiles --minzoom 0 --maxzoom 2 --colormap "1:#686868,2:#fbb4b9,3:#c51b8a,4:#49006a"
```

Any values in the GeoTIFF that are not present in the colormap are converted to
transparent pixels.

The colormap renderer will automatically select the smallest bit depth that can
hold all values of the colormap plus a transparency value:

-   a colormap with 1 value will be output as a 1-bit PNG
-   a colormap with 3 values will be output as a 2-bit PNG
-   a colormap with 14 values will be output as a 4-bit PNG
-   otherwise will be output as an 8-bit PNG

### Merge tilesets

You may need to render a given dataset at different zoom levels, such as using
internal overviews for low zooms and no overviews for higher zooms. You can use
the `merge` subcommand to merge these into a single tileset.

```bash
Usage: rastertiler merge <left MBTiles file> <right MBTiles file> <output MBTiles file>

Arguments:
  <left MBTiles file>
  <right MBTiles file>
  <output MBTiles file>

Options:
  -h, --help  Print help
```

## Credits

This started as a Rust port of
[rastertiler](https://github.com/brendan-ward/rastertiler) built in Go,
specifically to avoid the performance penalty of CGO bindings to GDAL and more
flexible PNG encoding.

See also [raster-tilecutter](https://github.com/brendan-ward/raster-tilecutter)
which does much the same thing, in Python, using `rasterio`.

This project was developed with the support of the
[U.S. Fish and Wildlife Service](https://www.fws.gov/)
[Southeast Conservation Adaptation Strategy](https://secassoutheast.org/) for
use in the
[Southeast Conservation Blueprint Viewer](https://blueprint.geoplatform.gov/southeast/).

## License

Licensed under either of

-   [Apache License, Version 2.0](http://www.apache.org/licenses/LICENSE-2.0)
-   [MIT license](http://opensource.org/licenses/MIT)

at your option.

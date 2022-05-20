use itertools::Itertools;
use std::f64::consts::PI;

use crate::bounds::Bounds;

const RE: f64 = 6378137.0;
const ORIGIN: f64 = RE * PI;
const CE: f64 = 2.0 * ORIGIN;
const RAD2DEG: f64 = 180.0 / PI;
const DEG2RAD: f64 = PI / 180.0;

/// Calculate Mercator coordinates for geographic coordinates.
/// Coordinates are clipped to -180 to 180 and -85.051129 to 85.051129.
///
/// # Arguments
/// * `lon` - longitude
/// * `lat` - latitude
///
/// # Returns
/// (x, y)
fn geo_to_mercator(lon: f64, lat: f64) -> (f64, f64) {
    // clamp x to -180 to 180 range
    let lon = lon.max(-180.0).min(180.0);

    // clamp y to -85.051129 to 85.051129 range
    let lat = lat.max(-85.051129).min(85.051129);

    let x = lon * (ORIGIN / 180.0);
    let y = RE * ((PI * 0.25) + (0.5 * DEG2RAD * lat)).tan().ln();

    (x, y)
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct TileID {
    pub zoom: u8,
    pub x: u32,
    pub y: u32,
}

impl TileID {
    /// Constructs a new TileID
    ///
    /// # Arguments
    /// * `zoom` - zoom level
    /// * `x` - tile column (X)
    /// * `y` - tile row (Y)
    pub fn new(zoom: u8, x: u32, y: u32) -> TileID {
        TileID { zoom, x, y }
    }

    pub fn geo_bounds(&self) -> Bounds {
        let z = (1 << self.zoom) as f64;
        let x = self.x as f64;
        let y = self.y as f64;

        Bounds {
            xmin: x / z * 360.0 - 180.0,
            ymin: (PI * (1.0 - 2.0 * ((y + 1.0) / z))).sinh().atan() * RAD2DEG,
            xmax: (x + 1.0) / z * 360.0 - 180.0,
            ymax: (PI * (1.0 - 2.0 * y / z)).sinh().atan() * RAD2DEG,
        }
    }
    pub fn mercator_bounds(&self) -> Bounds {
        let z = (1 << self.zoom) as f64;
        let x = self.x as f64;
        let y = self.y as f64;
        let tile_size = CE / z;

        let xmin = x * tile_size - CE / 2.0;
        let ymax = CE / 2.0 - y * tile_size;

        Bounds {
            xmin,
            ymin: ymax - tile_size,
            xmax: xmin + tile_size,
            ymax,
        }
    }
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct TileRange {
    pub zoom: u8,
    pub xmin: u32,
    pub ymin: u32,
    pub xmax: u32,
    pub ymax: u32,
}

impl TileRange {
    /// Creates a TileRange that covers the bounds at the zoom level
    ///
    /// # Arguments
    /// * `zoom` - zoom level to cover
    /// * `bounds` - Bounds object containing Mercator coordinates
    pub fn new(zoom: u8, bounds: &Bounds) -> TileRange {
        let z = (1 << zoom) as f64;
        let origin = -ORIGIN;
        let eps = 1e-11;

        let xmin = (((bounds.xmin - origin) / CE) * z)
            .floor()
            .max(0.0)
            .min(z - 1.0) as u32;
        let ymin = ((1.0 - ((bounds.ymax - origin) / CE)) * z)
            .floor()
            .max(0.0)
            .min(z - 1.0) as u32;

        let xmax = ((((bounds.xmax - origin) / CE) - eps) * z)
            .floor()
            .max(0.0)
            .min(z - 1.0) as u32;

        let ymax = ((1.0 - (((bounds.ymin - origin) / CE) + eps)) * z)
            .floor()
            .max(0.0)
            .min(z - 1.0) as u32;

        TileRange {
            zoom,
            xmin,
            ymin,
            xmax,
            ymax,
        }
    }

    pub fn count(&self) -> usize {
        (self.xmax as usize - self.xmin as usize + 1)
            * (self.ymax as usize - self.ymin as usize + 1) as usize
    }

    pub fn iter(&self) -> impl Iterator<Item = TileID> {
        let zoom = self.zoom;

        //  return iterator over tiles
        (self.xmin..self.xmax + 1)
            .cartesian_product(self.ymin..self.ymax + 1)
            .map(move |(x, y)| TileID { zoom, x, y })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::{approx_eq, approx_eq_bounds};
    use rstest::rstest;

    #[rstest]
    #[case(0., 0., 0., 0.)]
    #[case(-180., 0., -ORIGIN, 0.)]
    #[case(-180., -90., -ORIGIN, -20037508.6269291)]
    #[case(-180., -85.051129, -ORIGIN, -20037508.6269291)]
    fn geo_to_mercator(#[case] lon: f64, #[case] lat: f64, #[case] x: f64, #[case] y: f64) {
        let eps = 1e-6;
        let (actual_x, actual_y) = super::geo_to_mercator(lon, lat);
        assert!(
            approx_eq(actual_x, x, eps) && approx_eq(actual_y, y, eps),
            "({},{})!=({},{})",
            actual_x,
            actual_y,
            x,
            y
        );
    }

    #[test]
    fn new_tileid() {
        let t = TileID::new(4, 0, 1);
        assert_eq!(
            t,
            TileID {
                zoom: 4,
                x: 0,
                y: 1
            }
        )
    }

    #[rstest]
    #[case(TileID{zoom: 0, x: 0, y: 0}, Bounds{xmin: -180.0, ymin: -85.051129, xmax: 180.0, ymax: 85.051129})]
    #[case(TileID{zoom: 1, x: 1, y: 1}, Bounds{xmin: 0.0, ymin: -85.051129, xmax: 180.0, ymax: 0.0})]
    #[case(TileID{zoom: 10, x: 20, y: 30}, Bounds{xmin: -172.968750, ymin: 84.016022, xmax: -172.617188, ymax: 84.052561})]
    fn geo_bounds(#[case] tile: TileID, #[case] bounds: Bounds) {
        let actual = tile.geo_bounds();
        assert!(
            approx_eq_bounds(&actual, &bounds, 1e-6),
            "\n{:?}\n!=\n{:?}",
            actual,
            bounds
        );
    }

    #[rstest]
    #[case(TileID{zoom: 0, x: 0, y: 0}, Bounds{xmin: -20037508.342789, ymin: -20037508.342789, xmax: 20037508.342789, ymax: 20037508.342789})]
    #[case(TileID{zoom: 1, x: 1, y: 1}, Bounds{xmin: 0.0, ymin: -20037508.342789, xmax: 20037508.342789, ymax: 0.0})]
    #[case(TileID{zoom: 10, x: 20, y: 30}, Bounds{xmin: -19254793.173149, ymin: 18824299.829847, xmax: -19215657.414667, ymax: 18863435.588329})]
    fn mercator_bounds(#[case] tile: TileID, #[case] bounds: Bounds) {
        let actual = tile.mercator_bounds();
        assert!(
            approx_eq_bounds(&actual, &bounds, 1e-6),
            "\n{:?}\n!=\n{:?}",
            actual,
            bounds
        );
    }

    #[rstest]
    #[case(0, Bounds{xmin: -180.0, ymin: -90.0, xmax: 180.0, ymax: 90.0}, TileRange{zoom: 0, xmin: 0, ymin: 0, xmax: 0, ymax: 0})]
    #[case(1, Bounds{xmin: -180.0, ymin: -90.0, xmax: 90.0, ymax: 90.0}, TileRange{zoom: 1, xmin: 0, ymin: 0, xmax: 1, ymax: 1})]
    #[case(1, Bounds{xmin: -180.0, ymin: -90.0, xmax: 0.0, ymax: 90.0}, TileRange{zoom: 1, xmin: 0, ymin: 0, xmax: 0, ymax: 1})]
    #[case(4, Bounds{xmin: -100.0, ymin: -20.0, xmax: -20.0, ymax: 20.0}, TileRange{zoom: 4, xmin: 3, ymin: 7, xmax: 7, ymax: 8})]
    #[case(4, Bounds{xmin: -1e-6, ymin: -1e-6, xmax: 1e-6, ymax: 1e-6}, TileRange{zoom: 4, xmin: 7, ymin: 7, xmax: 8, ymax: 8})]
    fn tile_range(#[case] zoom: u8, #[case] bounds: Bounds, #[case] expected: TileRange) {
        // convert bounds to Mercator bounds
        let (xmin, ymin) = super::geo_to_mercator(bounds.xmin, bounds.ymin);
        let (xmax, ymax) = super::geo_to_mercator(bounds.xmax, bounds.ymax);
        let mercator_bounds = Bounds {
            xmin,
            ymin,
            xmax,
            ymax,
        };

        assert_eq!(TileRange::new(zoom, &mercator_bounds), expected);
    }

    #[rstest]
    #[case(0, Bounds{xmin: -180.0, ymin: -90.0, xmax: 180.0, ymax: 90.0}, 1)]
    #[case(1, Bounds{xmin: -180.0, ymin: -90.0, xmax: 90.0, ymax: 90.0}, 4)]
    #[case(1, Bounds{xmin: -180.0, ymin: -90.0, xmax: 0.0, ymax: 90.0}, 2)]
    #[case(4, Bounds{xmin: -100.0, ymin: -20.0, xmax: -20.0, ymax: 20.0}, 10)]
    #[case(4, Bounds{xmin: -1e-6, ymin: -1e-6, xmax: 1e-6, ymax: 1e-6}, 4)]
    fn tile_range_count(#[case] zoom: u8, #[case] bounds: Bounds, #[case] expected: usize) {
        // convert bounds to Mercator bounds
        let (xmin, ymin) = super::geo_to_mercator(bounds.xmin, bounds.ymin);
        let (xmax, ymax) = super::geo_to_mercator(bounds.xmax, bounds.ymax);
        let mercator_bounds = Bounds {
            xmin,
            ymin,
            xmax,
            ymax,
        };

        assert_eq!(TileRange::new(zoom, &mercator_bounds).count(), expected);
    }

    #[rstest]
    #[case(0, Bounds{xmin: -180.0, ymin: -90.0, xmax: 180.0, ymax: 90.0}, TileID{zoom: 0, x: 0, y: 0}, TileID{zoom: 0, x: 0, y: 0})]
    #[case(1, Bounds{xmin: -180.0, ymin: -90.0, xmax: 180.0, ymax: 90.0}, TileID{zoom: 1, x: 0, y: 0}, TileID { zoom: 1, x: 1, y: 1 })]
    #[case(1, Bounds{xmin: -180.0, ymin: -90.0, xmax: 0.0, ymax: 90.0}, TileID{zoom: 1, x: 0, y: 0}, TileID { zoom: 1, x: 0, y: 1 })]
    #[case(4, Bounds{xmin: -100.0, ymin: -20.0, xmax: -20.0, ymax: 20.0}, TileID{zoom: 4, x: 3, y: 7}, TileID { zoom: 4, x: 7, y: 8 })]
    #[case(4, Bounds{xmin: -1e-6, ymin: -1e-6, xmax: 1e-6, ymax: 1e-6}, TileID{zoom: 4, x: 7, y: 7}, TileID { zoom: 4, x: 8, y: 8 })]
    fn tile_range_iter(
        #[case] zoom: u8,
        #[case] bounds: Bounds,
        #[case] first: TileID,
        #[case] last: TileID,
    ) {
        // convert bounds to Mercator bounds
        let (xmin, ymin) = super::geo_to_mercator(bounds.xmin, bounds.ymin);
        let (xmax, ymax) = super::geo_to_mercator(bounds.xmax, bounds.ymax);
        let mercator_bounds = Bounds {
            xmin,
            ymin,
            xmax,
            ymax,
        };

        let range = TileRange::new(zoom, &mercator_bounds);
        let actual = range.iter().collect::<Vec<TileID>>();

        assert_eq!(*actual.first().unwrap(), first);
        assert_eq!(*actual.last().unwrap(), last);
    }
}

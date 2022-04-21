use std::f64::consts::PI;

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
    let x = lon.max(-180.0).min(180.0) * (ORIGIN / 180.0);

    // clamp y to -85.051129 to 85.051129 range
    let y = RE
        * ((PI * 0.25) + (0.5 * DEG2RAD * lat.max(-85.051129).min(85.051129)))
            .tan()
            .ln();

    return (x, y);
}

#[derive(Debug, Eq, PartialEq)]
pub struct TileID {
    zoom: u8,
    x: u32,
    y: u32,
}

#[derive(Debug)]
pub struct Bounds {
    xmin: f64,
    ymin: f64,
    xmax: f64,
    ymax: f64,
}

impl TileID {
    /// Constructs a new TileID
    ///
    /// # Arguments
    /// * `zoom` - zoom level
    /// * `x` - tile column (X)
    /// * `y` - tile row (Y)
    pub fn new(zoom: u8, x: u32, y: u32) -> TileID {
        return TileID { zoom, x, y };
    }

    /// Calculates the min and max TileIDs that would cover the input
    /// Mercator bounds.
    ///
    /// # Arguments
    /// * `zoom` - zoom level to cover
    /// * `bounds` - Bounds object containing Mercator coordinates
    pub fn tile_range(zoom: u8, bounds: &Bounds) -> (TileID, TileID) {
        let z = (1 << zoom) as f64;
        let origin = -ORIGIN;
        let eps = 1e-11;

        let xmin = (((bounds.xmin - origin) / CE) * z)
            .floor()
            .max(0.0)
            .min(z - 1.0) as u32;
        let ymin = ((1.0 - (((bounds.ymin - origin) / CE) + eps)) * z)
            .floor()
            .max(0.0)
            .min(z - 1.0) as u32;
        let xmax = ((((bounds.xmax - origin) / CE) - eps) * z)
            .floor()
            .max(0.0)
            .min(z - 1.0) as u32;
        let ymax = ((1.0 - ((bounds.ymax - origin) / CE)) * z)
            .floor()
            .max(0.0)
            .min(z - 1.0) as u32;

        return (
            TileID {
                zoom,
                x: xmin,
                y: ymax,
            },
            TileID {
                zoom,
                x: xmax,
                y: ymin,
            },
        );
    }
    pub fn geo_bounds(&self) -> Bounds {
        let z = (1 << self.zoom) as f64;
        let x = self.x as f64;
        let y = self.y as f64;

        return Bounds {
            xmin: x / z * 360.0 - 180.0,
            ymin: (PI * (1.0 - 2.0 * ((y + 1.0) / z))).sinh().atan() * RAD2DEG,
            xmax: (x + 1.0) / z * 360.0 - 180.0,
            ymax: (PI * (1.0 - 2.0 * y / z)).sinh().atan() * RAD2DEG,
        };
    }
    pub fn mercator_bounds(&self) -> Bounds {
        let z = (1 << self.zoom) as f64;
        let x = self.x as f64;
        let y = self.y as f64;
        let tile_size = CE / z;

        let xmin = x * tile_size - CE / 2.0;
        let ymax = CE / 2.0 - y * tile_size;

        return Bounds {
            xmin,
            ymin: ymax - tile_size,
            xmax: xmin + tile_size,
            ymax,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    fn approx_eq(l: f64, r: f64, precision: f64) -> bool {
        return (l - r).abs() < precision;
    }

    fn approx_eq_bounds(l: &Bounds, r: &Bounds, precision: f64) -> bool {
        approx_eq(l.xmin, r.xmin, precision)
            && approx_eq(l.ymin, r.ymin, precision)
            && approx_eq(l.xmax, r.xmax, precision)
            && approx_eq(l.ymax, r.ymax, precision)
    }

    #[rstest]
    #[case(0., 0., 0., 0.)]
    #[case(-180., 0., -ORIGIN, 0.)]
    #[case(-180., -90., -ORIGIN, -20037508.6269291)]
    #[case(-180., -85.051129, -ORIGIN, -20037508.6269291)]
    fn test_geo_to_mercator(#[case] lon: f64, #[case] lat: f64, #[case] x: f64, #[case] y: f64) {
        let eps = 1e-6;
        let (actual_x, actual_y) = geo_to_mercator(lon, lat);
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
    fn test_new_tileid() {
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
    fn test_tile_geo_bounds(#[case] tile: TileID, #[case] bounds: Bounds) {
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
    fn test_tile_mercator_bounds(#[case] tile: TileID, #[case] bounds: Bounds) {
        let actual = tile.mercator_bounds();
        assert!(
            approx_eq_bounds(&actual, &bounds, 1e-6),
            "\n{:?}\n!=\n{:?}",
            actual,
            bounds
        );
    }

    #[rstest]
    // #[case(0, Bounds{xmin: -180.0, ymin: -90.0, xmax: 180.0, ymax: 90.0}, TileID{zoom: 0, x: 0, y: 0}, TileID{zoom: 0, x: 0, y: 0})]
    // #[case(1, Bounds{xmin: -180.0, ymin: -90.0, xmax: 180.0, ymax: 90.0}, TileID{zoom: 1, x: 0, y: 0}, TileID{zoom: 1, x: 1, y: 1})]
    // #[case(1, Bounds{xmin: -180.0, ymin: -90.0, xmax: 0.0, ymax: 90.0}, TileID{zoom: 1, x: 0, y: 0}, TileID{zoom: 1, x: 0, y: 1})]
    #[case(4, Bounds{xmin: -100.0, ymin: -20.0, xmax: -20.0, ymax: 20.0}, TileID{zoom: 4, x: 3, y: 7}, TileID{zoom: 4, x: 7, y: 8})]
    // #[case(4, Bounds{xmin: -1e-6, ymin: -1e-6, xmax: 1e-6, ymax: 1e-6}, TileID{zoom: 4, x: 7, y: 7}, TileID{zoom: 4, x: 8, y: 8})]
    fn test_tile_range(
        #[case] zoom: u8,
        #[case] bounds: Bounds,
        #[case] min_tile: TileID,
        #[case] max_tile: TileID,
    ) {
        let expected = (min_tile, max_tile);

        // convert to Mercator bounds
        let (xmin, ymin) = geo_to_mercator(bounds.xmin, bounds.ymin);
        let (xmax, ymax) = geo_to_mercator(bounds.xmax, bounds.ymax);

        let mercator_bounds = Bounds {
            xmin,
            ymin,
            xmax,
            ymax,
        };
        let actual = TileID::tile_range(zoom, &mercator_bounds);

        assert_eq!(actual, expected);
    }
}

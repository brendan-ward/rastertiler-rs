use rstest::*;
use std::f64::consts::PI;

// const RE: f64 = 6378137.0;
// const ORIGIN: f64 = RE * PI;
// const CE: f64 = 2.0 * ORIGIN;
// const DEG2RAD: f64 = PI / 180.0;
const RAD2DEG: f64 = 180.0 / PI;

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
    pub fn new(zoom: u8, x: u32, y: u32) -> TileID {
        return TileID { zoom, x, y };
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
}

fn approx_eq(l: f64, r: f64, precision: f64) -> bool {
    return (l - r).abs() < precision;
}

#[test]
fn new() {
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
#[case(0,0,0, -180.0,-85.051129, 180.0, 85.051129)]
#[case(1,1,1, 0.0,-85.051129, 180.0, 0.0)]
#[case(10,20,30, -172.968750,84.016022, -172.617188, 84.052561)]
fn geo_bounds(
    #[case] zoom: u8,
    #[case] x: u32,
    #[case] y: u32,
    #[case] xmin: f64,
    #[case] ymin: f64,
    #[case] xmax: f64,
    #[case] ymax: f64,
) {
    let t = TileID::new(zoom, x, y);
    let expected = Bounds {
        xmin,
        ymin,
        xmax,
        ymax,
    };

    let actual = t.geo_bounds();

    let eps = 1e-6;

    assert!(
        approx_eq(actual.xmin, expected.xmin, eps)
            && approx_eq(actual.ymin, expected.ymin, eps)
            && approx_eq(actual.xmax, expected.xmax, eps)
            && approx_eq(actual.ymax, expected.ymax, eps),
        "{:?} != {:?}",
        actual,
        expected,
    );
}

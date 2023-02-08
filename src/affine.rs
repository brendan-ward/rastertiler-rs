#![allow(unused)]

use std::fmt;

#[derive(Debug)]
pub struct Affine {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub e: f64,
    pub f: f64,
}

impl fmt::Display for Affine {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Affine(a:{}, b:{}, c:{}, d: {}, e: {}, f:{})",
            self.a, self.b, self.c, self.d, self.e, self.f
        )
    }
}

impl Affine {
    pub fn new(a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) -> Affine {
        Affine { a, b, c, d, e, f }
    }

    pub fn from_gdal(transform: &[f64; 6]) -> Affine {
        Affine {
            a: transform[1],
            b: transform[2],
            c: transform[0],
            d: transform[4],
            e: transform[5],
            f: transform[3],
        }
    }

    pub fn to_gdal(&self) -> [f64; 6] {
        [self.c, self.a, self.b, self.f, self.d, self.e]
    }

    pub fn invert(&self) -> Affine {
        let inv_determinant = 1.0 / (self.a * self.e - self.b * self.d);
        let a = self.e * inv_determinant;
        let b = -self.b * inv_determinant;
        let d = -self.d * inv_determinant;
        let e = self.a * inv_determinant;

        Affine {
            a,
            b,
            c: -self.c * a - self.f * b,
            d,
            e,
            f: -self.c * d - self.f * e,
        }
    }

    pub fn multiply(&self, x: f64, y: f64) -> (f64, f64) {
        (
            x * self.a + y * self.b + self.c,
            x * self.d + y * self.e + self.f,
        )
    }

    pub fn scale(&self, x: f64, y: f64) -> Affine {
        Affine {
            a: self.a * x,
            b: self.b,
            c: self.c,
            d: self.d,
            e: self.e * y,
            f: self.f,
        }
    }

    pub fn resolution(&self) -> (f64, f64) {
        (self.a.abs(), self.e.abs())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::{approx_eq, approx_equal_affine};

    #[test]
    fn from_gdal() {
        let expected = Affine::new(1., 2., 0., 4., 5., 3.);
        let actual = Affine::from_gdal(&[0., 1., 2., 3., 4., 5.]);

        assert!(
            approx_equal_affine(&actual, &expected, 1e-6),
            "{} != {}",
            actual,
            expected
        );
    }

    #[test]
    fn to_gdal() {
        let expected = [0., 1., 2., 3., 4., 5.];
        let actual = Affine::from_gdal(&expected).to_gdal();

        let eps = 1e-6;

        assert!(
            approx_eq(actual[0], expected[0], eps)
                && approx_eq(actual[1], expected[1], eps)
                && approx_eq(actual[2], expected[2], eps)
                && approx_eq(actual[3], expected[3], eps)
                && approx_eq(actual[4], expected[4], eps)
                && approx_eq(actual[5], expected[5], eps),
            "{:?}!={:?}",
            actual,
            expected
        );
    }

    #[test]
    fn invert() {
        let expected = Affine::new(
            0.03333333333333333,
            0.0,
            -33.333333333333336,
            0.0,
            -0.03333333333333333,
            66.66666666666667,
        );

        let actual = Affine::new(30.0, 0.0, 1000.0, 0.0, -30.0, 2000.0).invert();

        assert!(
            approx_equal_affine(&actual, &expected, 1e-6),
            "{} != {}",
            actual,
            expected
        );
    }

    #[test]
    fn multiply() {
        let expected_x = 1060.0;
        let expected_y = 1910.0;

        let (actual_x, actual_y) =
            Affine::new(30.0, 0.0, 1000.0, 0.0, -30.0, 2000.0).multiply(2., 3.);

        let eps = 1e-6;
        assert!(
            approx_eq(actual_x, expected_x, eps) && approx_eq(actual_y, expected_y, eps),
            "{:?} != {:?}",
            (actual_x, actual_y),
            (expected_x, expected_y)
        );
    }

    #[test]
    fn scale() {
        let expected = Affine::new(60., 0., 1000., 0., -90., 2000.);
        let actual = Affine::new(30.0, 0.0, 1000.0, 0.0, -30.0, 2000.0).scale(2., 3.);

        assert!(
            approx_equal_affine(&actual, &expected, 1e-6),
            "{} != {}",
            actual,
            expected
        );
    }

    #[test]
    fn resolution() {
        let affine = Affine::new(30.0, 0.0, 1000.0, 0.0, -60.0, 2000.0);
        let expected_xres = 30.;
        let expected_yres = 60.;

        let (actual_xres, actual_yres) = affine.resolution();

        let eps = 1e-6;
        assert!(
            approx_eq(actual_xres, expected_xres, eps)
                && approx_eq(actual_yres, expected_yres, eps),
            "{:?} != {:?}",
            (actual_xres, actual_yres),
            (expected_xres, expected_yres)
        );
    }
}

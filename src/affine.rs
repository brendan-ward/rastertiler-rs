#[derive(Debug)]
pub struct Affine {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub e: f64,
    pub f: f64,
}

impl Affine {
    fn new(a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) -> Affine {
        return Affine { a, b, c, d, e, f };
    }

    fn from_gdal(transform: &[f64; 6]) -> Affine {
        return Affine {
            a: transform[1],
            b: transform[2],
            c: transform[0],
            d: transform[4],
            e: transform[5],
            f: transform[3],
        };
    }

    fn to_gdal(&self) -> [f64; 6] {
        return [self.c, self.a, self.b, self.f, self.d, self.e];
    }

    fn invert(&self) -> Affine {
        let inv_determinant = 1.0 / (self.a * self.e - self.b * self.d);
        let a = self.e * inv_determinant;
        let b = -self.b * inv_determinant;
        let d = -self.d * inv_determinant;
        let e = self.a * inv_determinant;

        return Affine {
            a,
            b,
            c: -self.c * a - self.f * b,
            d,
            e,
            f: -self.c * d - self.f * e,
        };
    }

    fn multiply(&self, x: f64, y: f64) -> (f64, f64) {
        return (
            x * self.a + y * self.b + self.c,
            x * self.d + y * self.e + self.f,
        );
    }

    fn scale(&self, x: f64, y: f64) -> Affine {
        return Affine {
            a: self.a * x,
            b: self.b,
            c: self.c,
            d: self.d,
            e: self.e * y,
            f: self.f,
        };
    }

    fn resolution(&self) -> (f64, f64) {
        return (self.a.abs(), self.e.abs());
    }

    fn to_string(&self) -> String {
        return format!(
            "Affine(a:{}, b:{}, c:{}, d: {}, e: {}, f:{})",
            self.a, self.b, self.c, self.d, self.e, self.f
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(l: f64, r: f64, precision: f64) -> bool {
        return (l - r).abs() < precision;
    }

    fn approx_equal_affine(l: &Affine, r: &Affine, precision: f64) -> bool {
        return approx_eq(l.a, r.a, precision)
            && approx_eq(l.b, r.b, precision)
            && approx_eq(l.c, r.c, precision)
            && approx_eq(l.d, r.d, precision)
            && approx_eq(l.e, r.e, precision)
            && approx_eq(l.f, r.f, precision);
    }

    #[test]
    fn test_from_gdal() {
        let expected = Affine::new(1., 2., 0., 4., 5., 3.);
        let actual = Affine::from_gdal(&[0., 1., 2., 3., 4., 5.]);

        assert!(
            approx_equal_affine(&actual, &expected, 1e-6),
            "{} != {}",
            actual.to_string(),
            expected.to_string()
        );
    }

    #[test]
    fn test_to_gdal() {
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
    fn test_invert() {
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
            actual.to_string(),
            expected.to_string()
        );
    }

    #[test]
    fn test_multiply() {
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
    fn test_scale() {
        let expected = Affine::new(60., 0., 1000., 0., -90., 2000.);
        let actual = Affine::new(30.0, 0.0, 1000.0, 0.0, -30.0, 2000.0).scale(2., 3.);

        assert!(
            approx_equal_affine(&actual, &expected, 1e-6),
            "{} != {}",
            actual.to_string(),
            expected.to_string()
        );
    }
}

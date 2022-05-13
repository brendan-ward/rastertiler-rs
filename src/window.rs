use crate::affine::Affine;
use crate::bounds::Bounds;

#[derive(Debug)]
pub struct Window {
    pub x_offset: f64,
    pub y_offset: f64,
    pub width: f64,
    pub height: f64,
}

impl Window {
    pub fn from_bounds(transform: &Affine, bounds: &Bounds) -> Window {
        let transform = transform.invert();

        let mut xs: [f64; 4] = [0., 0., 0., 0.];
        let mut ys: [f64; 4] = [0., 0., 0., 0.];

        // extract all pairs of coordinates
        let (x, y) = transform.multiply(bounds.xmin, bounds.ymin);
        xs[0] = x;
        ys[0] = y;

        let (x, y) = transform.multiply(bounds.xmin, bounds.ymax);
        xs[1] = x;
        ys[1] = y;

        let (x, y) = transform.multiply(bounds.xmax, bounds.ymin);
        xs[2] = x;
        ys[2] = y;

        let (x, y) = transform.multiply(bounds.xmax, bounds.ymax);
        xs[3] = x;
        ys[3] = y;

        let mut xmin = xs[0];
        let mut xmax = xs[0];
        let mut ymin = ys[0];
        let mut ymax = ys[0];

        for i in 1..4 {
            if xs[i] < xmin {
                xmin = xs[i];
            }
            if ys[i] < ymin {
                ymin = ys[i];
            }
            if xs[i] > xmax {
                xmax = xs[i];
            }
            if ys[i] > ymax {
                ymax = ys[i];
            }
        }

        Window {
            x_offset: xmin,
            y_offset: ymin,
            width: xmax - xmin,
            height: ymax - ymin,
        }
    }

    pub fn transform(&self, transform: &Affine) -> Affine {
        let (x, y) = transform.multiply(self.x_offset, self.y_offset);

        Affine {
            a: transform.a,
            b: transform.b,
            c: x,
            d: transform.d,
            e: transform.e,
            f: y,
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::test::{approx_eq, approx_equal_affine};

    fn approx_eq_window(l: &Window, r: &Window, precision: f64) -> bool {
        approx_eq(l.x_offset, r.x_offset, precision)
            && approx_eq(l.y_offset, r.y_offset, precision)
            && approx_eq(l.width, r.width, precision)
            && approx_eq(l.height, r.height, precision)
    }

    #[test]
    fn from_bounds() {
        let transform = Affine::new(30.0, 0.0, 1000.0, 0.0, -30.0, 2000.0);
        let bounds = Bounds {
            xmin: 0.,
            ymin: 10.,
            xmax: 100.,
            ymax: 200.,
        };

        let expected = Window {
            x_offset: -33.333333333333336,
            y_offset: 60.00000000000001,
            width: 3.333333333333332,
            height: 6.333333333333336,
        };
        let actual = Window::from_bounds(&transform, &bounds);

        assert!(
            approx_eq_window(&actual, &expected, 1e-6),
            "{:?}!={:?}",
            actual,
            expected
        );
    }

    #[rstest]
    #[case(Window{x_offset: 0., y_offset: 0., width: 10., height: 20.}, Affine{a:30., b:0., c:1000., d:0., e:-30., f: 2000.})]
    #[case(Window{x_offset: 10., y_offset: 20., width: 10., height: 20.}, Affine{a:30., b:0., c:1300., d:0., e:-30., f: 1400.})]
    #[case(Window{x_offset: -10., y_offset: -20., width: 10., height: 20.}, Affine{a:30., b:0., c:700., d:0., e:-30., f: 2600.})]
    fn transform(#[case] window: Window, #[case] expected: Affine) {
        let transform = Affine {
            a: 30.,
            b: 0.,
            c: 1000.,
            d: 0.,
            e: -30.,
            f: 2000.,
        };

        let actual = window.transform(&transform);

        assert!(
            approx_equal_affine(&actual, &expected, 1e-6),
            "\n{:?}\n!=\n{:?}",
            actual,
            expected
        );
    }
}

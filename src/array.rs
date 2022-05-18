use std::collections::BTreeMap;
use std::hash::Hash;

/// Return true if all values in the slice equal the passed in value
pub fn all_equals<T: PartialEq>(buffer: &[T], value: T) -> bool {
    buffer.iter().all(|x| *x == value)
}

pub fn equals<T: PartialEq>(left: &[T], right: &[T]) -> bool {
    left.len() == right.len() && left.iter().zip(right.iter()).all(|(l, r)| l == r)
}

/// Set all values in the slice to the passed in value
pub fn set_all<T: Copy>(buffer: &mut [T], value: T) {
    buffer.iter_mut().for_each(|x| *x = value);
}

pub fn histogram<T: Copy + Eq + Ord + Hash>(buffer: &[T]) -> BTreeMap<T, u64> {
    let mut map: BTreeMap<T, u64> = BTreeMap::new();
    let mut count: u64;
    for v in buffer.iter() {
        count = map.get(v).unwrap_or(&0) + 1u64;
        map.insert(*v, count);
    }

    map
}

/// Shift values that are stored at the head of the buffer based on size
/// and move these to positions within the buffer based on target size and offset,
/// backfilling the moved pixels with fill.
///
/// # Parameters
/// * buffer: full slice within which original values are stored and values are
///           to be written
/// * size: shape of the input region (width, height)
/// * target_size: shape of the full buffer (width, height)
/// * offset: (offset_x, offset_y)
/// * fill: fill value to backfill pixels after moving them
pub fn shift<T: Copy>(
    buffer: &mut [T],
    size: (usize, usize),
    target_size: (usize, usize),
    offset: (usize, usize),
    fill: T,
) {
    // start from last pixel in source part of buffer
    let mut src_index: usize;
    let mut dest_index: usize;
    for row in (0..size.1).rev() {
        for col in (0..size.0).rev() {
            src_index = row * size.0 + col;
            dest_index = (row + offset.1) * target_size.0 + (col + offset.0);
            buffer[dest_index] = buffer[src_index];
            buffer[src_index] = fill;
        }
    }
}

pub fn print_2d<T: PartialEq + Ord + std::fmt::Debug>(
    buffer: &[T],
    size: (usize, usize),
    nodata: T,
) {
    let max = buffer.iter().filter(|x| **x != nodata).max().unwrap();
    let padding = format!("{:?}", max).len() + 1;
    for row in 0..size.1 {
        if row > 0 {
            println!();
        }
        for col in 0..size.0 {
            if buffer[row * size.0 + col] == nodata {
                print!("{:<width$}", "-", width = padding);
            } else {
                print!("{:<width$?}", buffer[row * size.0 + col], width = padding);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    fn test_set_all_for_type<T: Copy + PartialEq>(init: T, value: T) {
        let mut array: [T; 2] = [init, init];
        set_all(&mut array, value);
        assert!(all_equals(&array, value));
    }

    // fn histogram_is_equal(l: BTreeMap<u8, u64>, r: BTreeMap<u8, u64>) -> bool {
    //     if l.len() != r.len() {
    //         return false;
    //     }

    //     // TODO: all values must be present

    //     return true;
    // }

    #[test]
    fn test_all_equals() {
        assert!(all_equals(&[0i8, 0i8], 0i8));
        assert!(all_equals(&[0u8, 0u8], 0u8));
        assert!(all_equals(&[0i16, 0i16], 0i16));
        assert!(all_equals(&[0u16, 0u16], 0u16));
        assert!(all_equals(&[0i32, 0i32], 0i32));
        assert!(all_equals(&[0u32, 0u32], 0u32));
        assert!(all_equals(&[0i64, 0i64], 0i64));
        assert!(all_equals(&[0u64, 0u64], 0u64));

        assert!(!all_equals(&[0i8, 1i8], 0i8));
        assert!(!all_equals(&[0u8, 1u8], 0u8));
        assert!(!all_equals(&[0i16, 1i16], 0i16));
        assert!(!all_equals(&[0u16, 1u16], 0u16));
        assert!(!all_equals(&[0i32, 1i32], 0i32));
        assert!(!all_equals(&[0u32, 1u32], 0u32));
        assert!(!all_equals(&[0i64, 1i64], 0i64));
        assert!(!all_equals(&[0u64, 1u64], 0u64));
    }

    #[test]
    fn test_equals() {
        assert!(equals(&[0i8, 1i8], &[0i8, 1i8]));
        assert!(equals(&[0u8, 1u8], &[0u8, 1u8]));
        assert!(equals(&[0i16, 1i16], &[0i16, 1i16]));
        assert!(equals(&[0u16, 1u16], &[0u16, 1u16]));
        assert!(equals(&[0i32, 1i32], &[0i32, 1i32]));
        assert!(equals(&[0u32, 1u32], &[0u32, 1u32]));
        assert!(equals(&[0i64, 1i64], &[0i64, 1i64]));
        assert!(equals(&[0u64, 1u64], &[0u64, 1u64]));

        assert!(!equals(&[0i8, 1i8], &[0i8, 0i8]));
        assert!(!equals(&[0u8, 1u8], &[0u8, 0u8]));
        assert!(!equals(&[0i16, 1i16], &[0i16, 0i16]));
        assert!(!equals(&[0u16, 1u16], &[0u16, 0u16]));
        assert!(!equals(&[0i32, 1i32], &[0i32, 0i32]));
        assert!(!equals(&[0u32, 1u32], &[0u32, 0u32]));
        assert!(!equals(&[0i64, 1i64], &[0i64, 0i64]));
        assert!(!equals(&[0u64, 1u64], &[0u64, 0u64]));

        assert!(!equals(&[0i8, 1i8], &[0i8]));
        assert!(!equals(&[0u8, 1u8], &[0u8]));
        assert!(!equals(&[0i16, 1i16], &[0i16]));
        assert!(!equals(&[0u16, 1u16], &[0u16]));
        assert!(!equals(&[0i32, 1i32], &[0i32]));
        assert!(!equals(&[0u32, 1u32], &[0u32]));
        assert!(!equals(&[0i64, 1i64], &[0i64]));
        assert!(!equals(&[0u64, 1u64], &[0u64]));
    }

    #[test]
    fn test_set_all() {
        test_set_all_for_type(0i8, 1i8);
        test_set_all_for_type(0u8, 1u8);
        test_set_all_for_type(0i16, 1i16);
        test_set_all_for_type(0u16, 1u16);
        test_set_all_for_type(0i32, 1i32);
        test_set_all_for_type(0u32, 1u32);
        test_set_all_for_type(0i64, 1i64);
        test_set_all_for_type(0u64, 1u64);
    }

    #[test]
    fn test_shift() {
        // first 2 pixels are filled based on a shape of 1 col x 2 rows (filled
        // into front of buffer)
        #[rustfmt::skip]
        let mut buffer: [u8; 12] = [
            1, 2, 0, 0,
            0, 0, 0, 0,
            0, 0, 0, 0
        ];
        let size: (usize, usize) = (1, 2);

        #[rustfmt::skip]
        let expected: [u8; 12] = [
            0, 0, 0, 0,
            0, 0, 1, 0,
            0, 0, 2, 0
        ];
        let target_size: (usize, usize) = (4, 3);
        let offset: (usize, usize) = (2, 1);

        shift(&mut buffer, size, target_size, offset, 0);
        assert!(equals(&buffer, &expected));
    }

    #[rstest]
    #[case([0u8,0u8,0u8,0u8], BTreeMap::from([(0u8, 4u64)]))]
    #[case([0u8,0u8,1u8,0u8], BTreeMap::from([(0u8, 3u64), (1u8, 1u64)]))]
    fn test_histogram(#[case] buffer: [u8; 4], #[case] expected: BTreeMap<u8, u64>) {
        assert_eq!(histogram(&buffer), expected);
    }
}

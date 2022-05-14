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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_set_all_for_type<T: Copy + PartialEq>(init: T, value: T) {
        let mut array: [T; 2] = [init, init];
        set_all(&mut array, value);
        assert!(all_equals(&array, value));
    }

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
}

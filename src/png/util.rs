// bits are packed so that first value is in highest bits and last value is in lowest bits

#![allow(clippy::too_many_arguments)]
#[inline]
pub fn pack_8u_1bit(v1: u8, v2: u8, v3: u8, v4: u8, v5: u8, v6: u8, v7: u8, v8: u8) -> u8 {
    v1 << 7u8 | v2 << 6u8 | v3 << 5u8 | v4 << 4u8 | v5 << 3u8 | v6 << 2u8 | v7 << 1u8 | v8
}

#[inline]
pub fn pack_8u_2bit(v1: u8, v2: u8, v3: u8, v4: u8) -> u8 {
    v1 << 6u8 | v2 << 4u8 | v3 << 2u8 | v4
}

#[inline]
pub fn pack_8u_4bit(v1: u8, v2: u8) -> u8 {
    v1 << 4u8 | v2
}

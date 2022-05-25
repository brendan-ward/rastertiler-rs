use std::error::Error;

pub use self::color::*;
pub use self::colormap::*;
pub use self::grayscale::*;
pub use self::rgb::*;
pub use self::util::*;

mod color;
mod colormap;
mod grayscale;
mod rgb;
mod util;

pub trait PixelValue: Ord + Copy + From<u8> {}

impl PixelValue for u8 {}
impl PixelValue for u16 {}
impl PixelValue for u32 {}

pub trait Encode<T: PixelValue> {
    fn encode(&self, buffer: &[T]) -> Result<Vec<u8>, Box<dyn Error>>;
    fn encode_8bit(&self, buffer: &[u8]) -> Result<Vec<u8>, Box<dyn Error>>;
}

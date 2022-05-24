use std::error::Error;

pub use self::color::*;
pub use self::colormap::*;
pub use self::grayscale::*;
pub use self::rgb::*;

mod color;
mod colormap;
mod grayscale;
mod rgb;

pub trait Encode {
    fn encode(&self, buffer: &[u8]) -> Result<Vec<u8>, Box<dyn Error>>;
}

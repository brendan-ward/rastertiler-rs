use std::error::Error;

pub use self::colormap::ColormapEncoder;
pub use self::grayscale::GrayscaleEncoder;

mod colormap;
mod grayscale;

pub trait Encode {
    fn encode(&self, buffer: &[u8]) -> Result<Vec<u8>, Box<dyn Error>>;
}

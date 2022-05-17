use std::error::Error;
use std::io::BufWriter;

use png::{BitDepth, ColorType, Encoder};

pub trait Encode {
    fn encode(&self, buffer: &[u8]) -> Result<Vec<u8>, Box<dyn Error>>;
}

#[derive(Debug)]
pub struct GrayscaleEncoder {
    width: u32,
    height: u32,
}

impl GrayscaleEncoder {
    pub fn new(width: u32, height: u32) -> GrayscaleEncoder {
        GrayscaleEncoder { width, height }
    }
}

impl Encode for GrayscaleEncoder {
    fn encode(&self, buffer: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
        // TODO: may want to pass in or internally provide png_buffer to reduce allocations
        let mut png_buffer: Vec<u8> = Vec::new();

        // in a block to force writer to finish writing on drop
        {
            let mut encoder = Encoder::new(
                BufWriter::new(&mut png_buffer),
                self.width as u32,
                self.height as u32,
            );

            encoder.set_color(ColorType::Grayscale);
            encoder.set_depth(BitDepth::Eight);

            let mut writer = encoder.write_header()?;
            writer.write_image_data(buffer)?;
        }

        Ok(png_buffer)
    }
}

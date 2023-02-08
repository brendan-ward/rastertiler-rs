use png::{BitDepth, ColorType, Compression, Encoder, FilterType};
use std::error::Error;
use std::io::BufWriter;

use crate::png::{Encode, PixelValue};

#[derive(Debug)]
pub struct GrayscaleEncoder {
    width: u32,
    height: u32,
    nodata: u8,
}

impl GrayscaleEncoder {
    pub fn new(width: u32, height: u32, nodata: u8) -> GrayscaleEncoder {
        GrayscaleEncoder {
            width,
            height,
            nodata,
        }
    }
}

impl<T: PixelValue> Encode<T> for GrayscaleEncoder {
    fn encode(&self, _buffer: &[T]) -> Result<Vec<u8>, Box<dyn Error>> {
        unimplemented!("encode() not implemented for GrayscaleEncoder, use encode_8bit() instead")
    }

    fn encode_8bit(&self, buffer: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
        // TODO: may want to pass in or internally provide png_buffer to reduce allocations
        let mut png_buffer: Vec<u8> = Vec::new();

        let mut encoder = Encoder::new(
            BufWriter::new(&mut png_buffer),
            self.width as u32,
            self.height as u32,
        );

        encoder.set_color(ColorType::Grayscale);
        encoder.set_depth(BitDepth::Eight);
        // turn off filter, according to PNG book
        encoder.set_filter(FilterType::NoFilter);
        encoder.set_compression(Compression::Best);

        // encode nodata as a 2 byte value per the spec, with value in high bits
        encoder.set_trns(vec![0, self.nodata]);

        let mut writer = encoder.write_header()?;
        writer.write_image_data(buffer)?;
        writer.finish()?;

        Ok(png_buffer)
    }
}

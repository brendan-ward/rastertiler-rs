use png::{BitDepth, ColorType, Compression, Encoder, FilterType};
use std::error::Error;
use std::io::BufWriter;

use crate::png::color::Rgb8;
use crate::png::{Encode, PixelValue};

#[derive(Debug)]
pub struct RGBEncoder {
    width: u32,
    height: u32,
    nodata_color: Rgb8,
}

impl RGBEncoder {
    pub fn new(width: u32, height: u32, nodata: u32) -> RGBEncoder {
        RGBEncoder {
            width,
            height,
            nodata_color: Rgb8::from_u32(nodata),
        }
    }
}

impl<T: PixelValue> Encode<T> for RGBEncoder {
    fn encode(&self, buffer: &[T]) -> Result<Vec<u8>, Box<dyn Error>> {
        unimplemented!("encode() not implemented for RGBEncoder, use encode_8bit() instead")
    }

    fn encode_8bit(&self, buffer: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
        let mut png_buffer: Vec<u8> = Vec::new();

        let mut encoder = Encoder::new(
            BufWriter::new(&mut png_buffer),
            self.width as u32,
            self.height as u32,
        );

        encoder.set_color(ColorType::Rgb);
        encoder.set_depth(BitDepth::Eight);
        encoder.set_compression(Compression::Best);
        // disabling filter appears to give smaller files for u32 data
        encoder.set_filter(FilterType::NoFilter);

        // encode nodata as a 2 byte RGB values per the spec, with value in high bits
        encoder.set_trns(vec![
            0,
            self.nodata_color.r,
            0,
            self.nodata_color.g,
            0,
            self.nodata_color.b,
        ]);

        let mut writer = encoder.write_header()?;
        writer.write_image_data(buffer)?;

        // force writer to finish writing on drop
        drop(writer);

        Ok(png_buffer)
    }
}

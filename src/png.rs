use crate::color::Colormap;
use png::{BitDepth, ColorType, Encoder};
use std::error::Error;
use std::io::BufWriter;

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

#[derive(Debug)]
pub struct ColormapEncoder {
    width: u32,
    height: u32,
    colormap: Colormap,
}

impl ColormapEncoder {
    pub fn new(width: u32, height: u32, colormap: &str) -> Result<ColormapEncoder, Box<dyn Error>> {
        Ok(ColormapEncoder {
            width,
            height,
            colormap: Colormap::new(colormap)?,
        })
    }
}

impl Encode for ColormapEncoder {
    fn encode(&self, buffer: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
        let mut png_buffer: Vec<u8> = Vec::new();

        {
            let mut encoder = Encoder::new(
                BufWriter::new(&mut png_buffer),
                self.width as u32,
                self.height as u32,
            );

            encoder.set_color(ColorType::Indexed);
            encoder.set_depth(BitDepth::Eight);
            encoder.set_palette(self.colormap.get_colors());
            encoder.set_trns(self.colormap.get_transparency());

            let mut writer = encoder.write_header()?;
            writer.write_image_data(
                &buffer
                    .iter()
                    .map(|v| self.colormap.get_index(v))
                    .collect::<Vec<u8>>(),
            )?;
        }

        Ok(png_buffer)
    }
}

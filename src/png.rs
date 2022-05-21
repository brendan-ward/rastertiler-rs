use crate::color::Colormap;
use png::{BitDepth, ColorType, Compression, Encoder, FilterType};
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
            // turn off filter, according to PNG book
            encoder.set_filter(FilterType::NoFilter);

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
    depth: BitDepth,
}

impl ColormapEncoder {
    pub fn new(width: u32, height: u32, colormap: &str) -> Result<ColormapEncoder, Box<dyn Error>> {
        let colormap = Colormap::new(colormap)?;

        let depth = match colormap.len() {
            l if l < 2 => BitDepth::One,
            l if l < 4 => BitDepth::Two,
            l if l < 16 => BitDepth::Four,
            _ => BitDepth::Eight,
        };

        // TODO: pre-allocate and reuse buffer for storing packed data

        Ok(ColormapEncoder {
            width,
            height,
            colormap,
            depth,
        })
    }

    fn pack_1bit(&self, buffer: &[u8]) -> Vec<u8> {
        let mut pixels: Vec<u8> = Vec::with_capacity(buffer.len() / 8);
        for i in (0..buffer.len()).step_by(8) {
            pixels.push(
                (self.colormap.get_index(buffer[i]) << 7u8
                    | self.colormap.get_index(buffer[i + 1]) << 6u8
                    | self.colormap.get_index(buffer[i + 2]) << 5u8
                    | self.colormap.get_index(buffer[i + 3]) << 4u8
                    | self.colormap.get_index(buffer[i + 4]) << 3u8
                    | self.colormap.get_index(buffer[i + 5]) << 2u8
                    | self.colormap.get_index(buffer[i + 6]) << 1u8
                    | self.colormap.get_index(buffer[i + 7])) as u8,
            );
        }

        pixels
    }

    fn pack_2bit(&self, buffer: &[u8]) -> Vec<u8> {
        let mut pixels: Vec<u8> = Vec::with_capacity(buffer.len() / 4);
        for i in (0..buffer.len()).step_by(4) {
            pixels.push(
                (self.colormap.get_index(buffer[i]) << 6u8
                    | self.colormap.get_index(buffer[i + 1]) << 4u8
                    | self.colormap.get_index(buffer[i + 2]) << 2u8
                    | self.colormap.get_index(buffer[i + 3])) as u8,
            );
        }

        pixels
    }

    fn pack_4bit(&self, buffer: &[u8]) -> Vec<u8> {
        // bits are packed so that first index is in high bits
        let mut pixels: Vec<u8> = Vec::with_capacity(buffer.len() / 2);
        for i in (0..buffer.len()).step_by(2) {
            pixels.push(
                (self.colormap.get_index(buffer[i]) << 4u8 | self.colormap.get_index(buffer[i + 1]))
                    as u8,
            );
        }

        pixels
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
            encoder.set_compression(Compression::Best);
            // turn off filter, not useful for paletted PNGs
            encoder.set_filter(FilterType::NoFilter);

            encoder.set_depth(self.depth);
            encoder.set_palette(self.colormap.get_colors());
            encoder.set_trns(self.colormap.get_transparency());

            let mut writer = encoder.write_header()?;

            let pixels: Vec<u8> = match self.depth {
                BitDepth::One => self.pack_1bit(buffer),
                BitDepth::Two => self.pack_2bit(buffer),
                BitDepth::Four => self.pack_4bit(buffer),
                BitDepth::Eight => buffer
                    .iter()
                    .map(|&v| self.colormap.get_index(v))
                    .collect::<Vec<u8>>(),
                _ => unreachable!(),
            };

            writer.write_image_data(&pixels)?;
        }

        Ok(png_buffer)
    }
}

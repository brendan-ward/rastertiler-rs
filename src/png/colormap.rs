use crate::png::color::ColormapRgb8;
use png::{BitDepth, ColorType, Compression, Encoder, FilterType};
use std::error::Error;
use std::io::BufWriter;

use crate::png::{pack_8u_1bit, pack_8u_2bit, pack_8u_4bit, Encode, PixelValue};

// TODO: pre-allocate and reuse buffer for storing packed data

#[derive(Debug)]
pub struct ColormapEncoder<T: PixelValue> {
    pub width: u32,
    pub height: u32,
    pub colormap: ColormapRgb8<T>,
}

impl<T: PixelValue> ColormapEncoder<T> {
    pub fn new(
        width: u32,
        height: u32,
        nodata: T,
        palette_size: usize,
    ) -> Result<ColormapEncoder<T>, Box<dyn Error>> {
        Ok(ColormapEncoder {
            width,
            height,
            colormap: ColormapRgb8::new(palette_size, nodata),
        })
    }

    pub fn from_str(
        width: u32,
        height: u32,
        colormap_str: &str,
        nodata: u8,
    ) -> Result<ColormapEncoder<u8>, Box<dyn Error>> {
        Ok(ColormapEncoder {
            width,
            height,
            colormap: ColormapRgb8::<u8>::parse(colormap_str, nodata)?,
        })
    }

    fn pack_1bit(&self, buffer: &[T]) -> Vec<u8> {
        let mut pixels: Vec<u8> = Vec::with_capacity(buffer.len() / 8);
        for i in (0..buffer.len()).step_by(8) {
            pixels.push(pack_8u_1bit(
                self.colormap.get_index(buffer[i].into()),
                self.colormap.get_index(buffer[i + 1].into()),
                self.colormap.get_index(buffer[i + 2].into()),
                self.colormap.get_index(buffer[i + 3].into()),
                self.colormap.get_index(buffer[i + 4].into()),
                self.colormap.get_index(buffer[i + 5].into()),
                self.colormap.get_index(buffer[i + 6].into()),
                self.colormap.get_index(buffer[i + 7].into()),
            ));
        }

        pixels
    }

    fn pack_2bit(&self, buffer: &[T]) -> Vec<u8> {
        let mut pixels: Vec<u8> = Vec::with_capacity(buffer.len() / 4);
        for i in (0..buffer.len()).step_by(4) {
            pixels.push(pack_8u_2bit(
                self.colormap.get_index(buffer[i].into()),
                self.colormap.get_index(buffer[i + 1].into()),
                self.colormap.get_index(buffer[i + 2].into()),
                self.colormap.get_index(buffer[i + 3].into()),
            ));
        }

        pixels
    }

    fn pack_4bit(&self, buffer: &[T]) -> Vec<u8> {
        let mut pixels: Vec<u8> = Vec::with_capacity(buffer.len() / 2);
        for i in (0..buffer.len()).step_by(2) {
            pixels.push(pack_8u_4bit(
                self.colormap.get_index(buffer[i].into()),
                self.colormap.get_index(buffer[i + 1].into()),
            ));
        }

        pixels
    }
}

impl<T: PixelValue> Encode<T> for ColormapEncoder<T> {
    fn encode_8bit(&self, _buffer: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
        // self.encode(buffer.into())
        unimplemented!("encode_8bit() not implemented for ColormapEncoder, use encode() instead")
    }

    fn encode(&self, buffer: &[T]) -> Result<Vec<u8>, Box<dyn Error>> {
        let depth = match self.colormap.len() {
            l if l <= 2 => BitDepth::One,
            l if l <= 4 => BitDepth::Two,
            l if l <= 16 => BitDepth::Four,
            _ => BitDepth::Eight,
        };

        let mut png_buffer: Vec<u8> = Vec::new();

        let mut encoder = Encoder::new(
            BufWriter::new(&mut png_buffer),
            self.width as u32,
            self.height as u32,
        );

        encoder.set_color(ColorType::Indexed);
        encoder.set_compression(Compression::Best);
        // turn off filter, not useful for paletted PNGs
        encoder.set_filter(FilterType::NoFilter);

        encoder.set_depth(depth);
        encoder.set_palette(self.colormap.get_colors());
        encoder.set_trns(self.colormap.get_transparency());

        let mut writer = encoder.write_header()?;

        let pixels: Vec<u8> = match depth {
            BitDepth::One => self.pack_1bit(buffer),
            BitDepth::Two => self.pack_2bit(buffer),
            BitDepth::Four => self.pack_4bit(buffer),
            BitDepth::Eight => buffer
                .iter()
                .map(|&v| self.colormap.get_index(v.into()))
                .collect::<Vec<u8>>(),
            _ => unreachable!(),
        };

        writer.write_image_data(&pixels)?;
        writer.finish()?;

        Ok(png_buffer)
    }
}

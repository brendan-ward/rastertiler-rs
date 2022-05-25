use std::collections::BTreeMap;
use std::error::Error;

use hex;

use crate::png::PixelValue;

#[derive(Debug, Eq, PartialEq)]
pub struct Rgb8 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb8 {
    pub fn from_hex(hex_str: &str) -> Result<Rgb8, Box<dyn Error>> {
        if hex_str.len() != 7 {
            return Err("unsupported hex format")?;
        }

        let decoded = hex::decode(&hex_str[1..])?;

        Ok(Rgb8 {
            r: decoded[0],
            g: decoded[1],
            b: decoded[2],
        })
    }

    pub fn from_u32(value: u32) -> Rgb8 {
        Rgb8 {
            r: (value >> 16u32) as u8,
            g: (value >> 8u32) as u8,
            b: (value & 0xFF) as u8,
        }
    }
}

#[derive(Debug)]
pub struct ColormapRgb8<T: PixelValue> {
    values: BTreeMap<T, u8>,
    colors: Vec<u8>,
}

impl<T: PixelValue> ColormapRgb8<T> {
    pub fn new(capacity: usize, nodata: T) -> ColormapRgb8<T> {
        let mut colormap = ColormapRgb8 {
            values: BTreeMap::new(),
            colors: Vec::with_capacity((capacity + 1) * 3),
        };

        // NODATA is always associated with first index
        colormap.values.insert(nodata, 0u8);
        colormap.colors.push(0u8);
        colormap.colors.push(0u8);
        colormap.colors.push(0u8);

        colormap
    }

    pub fn clear(&mut self) {
        self.values.clear();
        self.colors.clear();
    }

    pub fn add_color(&mut self, value: T, color: Rgb8) {
        // only add unique entries
        if !self.values.contains_key(&value) {
            self.values.insert(value, self.values.len() as u8);
            self.colors.push(color.r);
            self.colors.push(color.g);
            self.colors.push(color.b);
        }
    }

    pub fn parse(colormap_str: &str, nodata: u8) -> Result<ColormapRgb8<u8>, Box<dyn Error>> {
        let num_colors = colormap_str.matches(",").count() + 1;
        let mut colormap = ColormapRgb8::<u8>::new(num_colors, nodata);

        let mut value: u8;
        let mut color: Rgb8;
        for entry in colormap_str.split(",") {
            let parts: Vec<&str> = entry.split(":").collect();
            value = parts[0].parse()?;
            color = Rgb8::from_hex(parts[1])?;
            colormap.add_color(value, color);
        }

        Ok(colormap)
    }

    /// Return index value for input value, returning index 0
    /// if not found (corresponds to transparent)
    pub fn get_index(&self, value: T) -> u8 {
        match self.values.get(&value) {
            Some(v) => *v,
            _ => 0u8,
        }
    }

    pub fn get_colors(&self) -> &Vec<u8> {
        &self.colors
    }

    pub fn get_transparency(&self) -> &[u8] {
        // transparency is always stored in lowest index
        &[0u8][..]
    }

    pub fn len(&self) -> usize {
        self.colors.len() / 3
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case("#FF00FF", Rgb8{r: 255u8, g: 0u8, b: 255u8})]
    fn test_color_from_hex(#[case] hex_str: &str, #[case] expected: Rgb8) {
        let actual = Rgb8::from_hex(hex_str).expect("color not parsed correctly");
        assert_eq!(actual, expected);
    }
}

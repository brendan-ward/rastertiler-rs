use hex;
use std::collections::HashMap;
use std::error::Error;

#[derive(Debug, Eq, PartialEq)]
pub struct Color {
    r: u8,
    g: u8,
    b: u8,
}

impl Color {
    pub fn from_hex(hex_str: &str) -> Result<Color, Box<dyn Error>> {
        if hex_str.len() != 7 {
            return Err("unsupported hex format")?;
        }

        let decoded = hex::decode(&hex_str[1..])?;

        Ok(Color {
            r: decoded[0],
            g: decoded[1],
            b: decoded[2],
        })
    }
}

#[derive(Debug)]
pub struct Colormap {
    values: HashMap<u8, u8>,
    colors: Vec<u8>,
    transparency: Vec<u8>,
}

impl Colormap {
    // TODO: transparency for values outside range
    pub fn new(colormap: &str) -> Result<Colormap, Box<dyn Error>> {
        let mut values: HashMap<u8, u8> = HashMap::new();
        let num_colors = colormap.matches(",").count() + 2;
        let mut colors: Vec<u8> = Vec::with_capacity(num_colors * 3);
        let mut transparency: Vec<u8> = Vec::with_capacity(num_colors);
        let mut value: u8;
        let mut color: Color;
        for (index, entry) in colormap.split(",").enumerate() {
            let parts: Vec<&str> = entry.split(":").collect();
            value = parts[0].parse()?;
            values.insert(value, index as u8);
            color = Color::from_hex(parts[1])?;
            colors.push(color.r);
            colors.push(color.g);
            colors.push(color.b);
            transparency.push(255);
        }
        // add transparent color at end
        colors.push(0);
        colors.push(0);
        colors.push(0);
        transparency.push(0);

        // TODO: push transparent

        Ok(Colormap {
            values,
            colors,
            transparency,
        })
    }

    /// Return index value for input value, returning index beyond length of
    /// indexes if not found (corresponds to transparent)
    pub fn get_index(&self, value: &u8) -> u8 {
        match self.values.get(value) {
            Some(v) => *v,
            _ => self.values.len() as u8,
        }
    }

    pub fn get_colors(&self) -> &Vec<u8> {
        &self.colors
    }

    pub fn get_transparency(&self) -> &Vec<u8> {
        &self.transparency
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case("#FF00FF", Color{r: 255, g: 0, b: 255})]
    fn test_color_from_hex(#[case] hex_str: &str, #[case] expected: Color) {
        let actual = Color::from_hex(hex_str).expect("color not parsed correctly");
        assert_eq!(actual, expected);
    }
}

use crate::{color::Rgb, image::{CycleImage, IndexedImage}, palette::Palette, palette::Cycle};

use std::convert::TryInto;
use serde::{de::{Error, IgnoredAny, Visitor}, Deserializer};

struct CycleImageVisitor;

impl<'de> Visitor<'de> for CycleImageVisitor {
    type Value = CycleImage;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a canvas cycle JSON file")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where A: serde::de::MapAccess<'de>, {
        let mut width = None;
        let mut height = None;
        let mut palette = None;
        let mut cycles = None;
        let mut image = None;

        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "width" => {
                    width = Some(map.next_value()?);
                }
                "height" => {
                    height = Some(map.next_value()?);
                }
                "colors" => {
                    palette = Some(map.next_value()?)
                }
                "cycles" => {
                    cycles = Some(map.next_value()?)
                }
                "pixels" => {
                    image = Some(map.next_value()?)
                }
                _ => {
                    map.next_value::<IgnoredAny>()?;
                }
            }
        }

        if width.is_none() {
            return Err(Error::missing_field("width"));
        }

        if height.is_none() {
            return Err(Error::missing_field("height"));
        }

        if palette.is_none() {
            return Err(Error::missing_field("colors"));
        }

        if cycles.is_none() {
            return Err(Error::missing_field("cycles"));
        }

        if image.is_none() {
            return Err(Error::missing_field("pixels"));
        }

        let (Some(width), Some(height), Some(palette), Some(cycles), Some(image)) = (width, height, palette, cycles, image) else {
            return Err(Error::custom("internal error (some field is missing)"));
        };

        let Some(indexed_image) = IndexedImage::from_buffer(width, height, image, palette) else {
            return Err(Error::custom("image buffer is too small for given width/height"));
        };

        Ok(CycleImage::new(indexed_image, cycles))
    }
}

impl<'de> serde::de::Deserialize<'de> for CycleImage {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        deserializer.deserialize_map(CycleImageVisitor)
    }
}

struct RgbVisitor;

impl<'de> Visitor<'de> for RgbVisitor {
    type Value = Rgb;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("RGB value as list of 3 numbers, each in the range of 0 to 255")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where A: serde::de::SeqAccess<'de>, {
        let Some(r) = seq.next_element()? else {
            return Err(Error::missing_field("r"));
        };

        let Some(g) = seq.next_element()? else {
            return Err(Error::missing_field("g"));
        };

        let Some(b) = seq.next_element()? else {
            return Err(Error::missing_field("b"));
        };

        if seq.next_element::<IgnoredAny>()?.is_some() {
            return Err(Error::custom("superfluous elements in RGB value"));
        };

        Ok(Rgb([r, g, b]))
    }
}

impl<'de> serde::de::Deserialize<'de> for Rgb {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        deserializer.deserialize_seq(RgbVisitor)
    }
}

struct PaletteVisitor;

impl<'de> Visitor<'de> for PaletteVisitor {
    type Value = Palette;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a list of 256 RGB values")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where A: serde::de::SeqAccess<'de>, {
        let mut colors = Vec::with_capacity(256);

        while let Some(rgb) = seq.next_element()? {
            colors.push(rgb);
        }

        let colors: Box<[Rgb; 256]> = match colors.try_into() {
            Ok(colors) => colors,
            Err(_) => return Err(Error::custom("the color palette needs to have exactly 256 color values"))
        };

        Ok(Palette(colors))
    }
}

impl<'de> serde::de::Deserialize<'de> for Palette {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        deserializer.deserialize_seq(PaletteVisitor)
    }
}

struct CycleVisitor;

impl<'de> Visitor<'de> for CycleVisitor {
    type Value = Cycle;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a color cycle definition")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where A: serde::de::MapAccess<'de>, {
        let mut reverse = false;
        let mut rate = 0;
        let mut low = None;
        let mut high = None;

        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "reverse" => {
                    let value: i32 = map.next_value()?;
                    if value == 0 {
                        reverse = false;
                    } else if value == 2 {
                        reverse = true;
                    } else {
                        return Err(Error::invalid_value(
                            serde::de::Unexpected::Signed(value as i64),
                            &"0 or 2"));
                    }
                }
                "rate" => {
                    rate = map.next_value()?;
                }
                "low" => {
                    low = Some(map.next_value()?);
                }
                "high" => {
                    high = Some(map.next_value()?);
                }
                _ => {
                    let _ = map.next_value::<IgnoredAny>()?;
                }
            }
        }

        if low.is_none() {
            return Err(Error::missing_field("low"));
        }

        if high.is_none() {
            return Err(Error::missing_field("high"));
        }

        let (Some(low), Some(high)) = (low, high) else {
            return Err(Error::custom("internal error"));
        };

        Ok(Cycle::new(low, high, rate, reverse))
    }
}

impl<'de> serde::de::Deserialize<'de> for Cycle {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        deserializer.deserialize_map(CycleVisitor)
    }
}

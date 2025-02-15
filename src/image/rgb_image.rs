// color-cycle - render color cycle images on the terminal
// Copyright (C) 2025  Mathias Panzenböck
// 
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
// 
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
// 
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use crate::color::Rgb;
use super::IndexedImage;
use crate::palette::Palette;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RgbImage {
    width: u32,
    height: u32,
    data: Box<[u8]>,
}

impl RgbImage {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            data: vec![0; width as usize * height as usize * 3].into(),
        }
    }

    pub fn from_color(width: u32, height: u32, color: Rgb) -> Self {
        let Rgb([r, g, b]) = color;
        Self {
            width,
            height,
            data: [r, g, b].repeat(width as usize * height as usize).into(),
        }
    }

    pub fn from_buffer(width: u32, height: u32, image: &[u8]) -> Option<Self> {
        let size = width as usize * height as usize * 3;
        if image.len() < size {
            return None;
        }

        Some(Self {
            width,
            height,
            data: image[..size].into(),
        })
    }

    pub fn from_indexed_image(indexed_image: &IndexedImage) -> Self {
        let mut data = Vec::with_capacity(indexed_image.data().len() * 3);
        let palette = indexed_image.palette();

        for index in indexed_image.data().iter().cloned() {
            let Rgb([r, g, b]) = palette[index];
            data.push(r);
            data.push(g);
            data.push(b);
        }

        Self {
            width: indexed_image.width(),
            height: indexed_image.height(),
            data: data.into(),
        }
    }

    #[inline]
    pub fn width(&self) -> u32 {
        self.width
    }

    #[inline]
    pub fn height(&self) -> u32 {
        self.height
    }

    #[inline]
    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    #[inline]
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    #[inline]
    pub fn get_pixel(&self, x: u32, y: u32) -> Rgb {
        let offset = (self.width as usize * y as usize + x as usize) * 3;
        let r = self.data[offset];
        let g = self.data[offset + 1];
        let b = self.data[offset + 2];
        Rgb([r, g, b])
    }

    #[inline]
    pub fn set_pixel(&mut self, x: u32, y: u32, color: Rgb) {
        let offset = (self.width as usize * y as usize + x as usize) * 3;
        let Rgb([r, g, b]) = color;
        self.data[offset] = r;
        self.data[offset + 1] = g;
        self.data[offset + 2] = b;
    }

    pub fn draw_indexed_image(&mut self, indexed_image: &IndexedImage) {
        let palette = indexed_image.palette();

        let mut pixel_iter = self.data.iter_mut();
        for index in indexed_image.data().iter().cloned() {
            let Rgb([r, g, b]) = palette[index];
            *pixel_iter.next().unwrap() = r;
            *pixel_iter.next().unwrap() = g;
            *pixel_iter.next().unwrap() = b;
        }
    }

    pub fn draw_indexed_image_with_palette(&mut self, indexed_image: &IndexedImage, palette: &Palette) {
        let mut pixel_iter = self.data.iter_mut();
        for index in indexed_image.data().iter().cloned() {
            let Rgb([r, g, b]) = palette[index];
            *pixel_iter.next().unwrap() = r;
            *pixel_iter.next().unwrap() = g;
            *pixel_iter.next().unwrap() = b;
        }
    }
}

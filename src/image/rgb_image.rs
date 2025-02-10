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
    data: Box<[Rgb]>,
}

impl From<RgbImage> for Box<[Rgb]> {
    #[inline]
    fn from(value: RgbImage) -> Self {
        value.data
    }
}

impl RgbImage {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            data: vec![Rgb::default(); width as usize * height as usize].into(),
        }
    }

    pub fn from_color(width: u32, height: u32, color: Rgb) -> Self {
        Self {
            width,
            height,
            data: vec![color; width as usize * height as usize].into(),
        }
    }

    pub fn from_buffer(width: u32, height: u32, image: &[Rgb]) -> Option<Self> {
        let size = width as usize * height as usize;
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
        let mut data = unsafe { Box::new_uninit_slice(indexed_image.data().len()).assume_init() };
        let palette = indexed_image.palette();

        for (index, pixel) in indexed_image.data().iter().cloned().zip(data.iter_mut()) {
            *pixel = palette[index];
        }

        Self {
            width: indexed_image.width(),
            height: indexed_image.height(),
            data,
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
    pub fn get_pixel(&self, x: u32, y: u32) -> Rgb {
        let offset = self.width as usize * y as usize + x as usize;
        self.data[offset]
    }

    #[inline]
    pub fn set_pixel(&mut self, x: u32, y: u32, color: Rgb) {
        let offset = self.width as usize * y as usize + x as usize;
        self.data[offset] = color;
    }

    #[inline]
    pub fn fill(&mut self, color: Rgb) {
        self.data.fill(color);
    }

    pub fn draw_indexed_image(&mut self, indexed_image: &IndexedImage) {
        let palette = indexed_image.palette();
        for (index, pixel) in indexed_image.data().iter().cloned().zip(self.data.iter_mut()) {
            *pixel = palette[index];
        }
    }

    pub fn draw_indexed_image_with_palette(&mut self, indexed_image: &IndexedImage, palette: &Palette) {
        for (index, pixel) in indexed_image.data().iter().cloned().zip(self.data.iter_mut()) {
            *pixel = palette[index];
        }
    }

    pub fn get_rect_data(&self, x: u32, y: u32, width: u32, height: u32) -> Box<[Rgb]> {
        if x >= self.width || y >= self.height {
            return Box::new([]);
        }

        let width = width.min(self.width - x);
        let height = height.min(self.height - y);
        let size = width as usize * height as usize;

        let mut data = unsafe { Box::new_uninit_slice(size).assume_init() };

        for new_y in 0..height {
            let old_offset = (y + new_y) as usize * self.width as usize + x as usize;
            let new_offset = new_y as usize * width as usize;
            data[new_offset..new_offset + width as usize].copy_from_slice(&self.data[old_offset..old_offset + width as usize]);
        }

        data
    }

    #[inline]
    pub fn get_rect(&self, x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            data: self.get_rect_data(x, y, width, height),
        }
    }

    #[inline]
    pub fn get_rect_from(&mut self, x: u32, y: u32, width: u32, height: u32, other: &RgbImage) {
        let width = width.min(other.width - x);
        let height = height.min(other.height - y);
        self.width = width;
        self.height = height;
        self.data = other.get_rect_data(x, y, width, height);
    }

    pub fn resize(&mut self, width: u32, height: u32, color: Rgb) {
        if width == self.width && height == self.height {
            return;
        }

        let size = width as usize * height as usize;
        let mut data: Box<[Rgb]> = vec![color; size].into();

        for new_y in 0..height.min(self.height) {
            let old_offset = new_y as usize * self.width as usize;
            let new_offset = new_y as usize * width as usize;
            let copy_width = width.min(self.width);
            data[new_offset..new_offset + copy_width as usize].copy_from_slice(&self.data[old_offset..old_offset + copy_width as usize]);
        }

        self.width = width;
        self.height = height;
        self.data = data;
    }
}

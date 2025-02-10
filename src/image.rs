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

use crate::{color::Rgb, palette::{Cycle, Palette}};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexedImage {
    width: u32,
    height: u32,
    data: Box<[u8]>,
    palette: Palette,
}

impl From<IndexedImage> for Box<[u8]> {
    #[inline]
    fn from(value: IndexedImage) -> Self {
        value.data
    }
}

impl IndexedImage {
    pub fn new(width: u32, height: u32, palette: Palette) -> Self {
        Self {
            width,
            height,
            data: vec![0; width as usize * height as usize].into(),
            palette,
        }
    }

    pub fn from_index(width: u32, height: u32, index: u8, palette: Palette) -> Self {
        Self {
            width,
            height,
            data: vec![index; width as usize * height as usize].into(),
            palette,
        }
    }

    pub fn from_buffer(width: u32, height: u32, image: Box<[u8]>, palette: Palette) -> Option<Self> {
        let size = width as usize * height as usize;
        if image.len() < size {
            return None;
        }

        Some(Self {
            width,
            height,
            data: if image.len() > size { image[..size].into() } else { image },
            palette,
        })
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
    pub fn palette(&self) -> &Palette {
        &self.palette
    }

    #[inline]
    pub fn palette_mut(&mut self) -> &mut Palette {
        &mut self.palette
    }

    #[inline]
    pub fn get_index(&self, x: u32, y: u32) -> u8 {
        let offset = self.width as usize * y as usize + x as usize;
        self.data[offset]
    }

    #[inline]
    pub fn set_index(&mut self, x: u32, y: u32, index: u8) {
        let offset = self.width as usize * y as usize + x as usize;
        self.data[offset] = index;
    }

    #[inline]
    pub fn fill(&mut self, index: u8) {
        self.data.fill(index);
    }

    pub fn get_rect_data(&self, x: u32, y: u32, width: u32, height: u32) -> Box<[u8]> {
        if x >= self.width || y >= self.height {
            return Box::new([]);
        }

        let width = width.min(self.width - x);
        let height = height.min(self.height - y);
        let size = width as usize * height as usize;

        let mut data = unsafe { Box::new_uninit_slice(size).assume_init() };

        for new_y in 0..height.min(self.height) {
            let old_offset = (y + new_y) as usize * self.width as usize + x as usize;
            let new_offset = new_y as usize * width as usize;
            let copy_width = width.min(self.width);
            data[new_offset..new_offset + copy_width as usize].copy_from_slice(&self.data[old_offset..old_offset + copy_width as usize]);
        }

        data
    }

    #[inline]
    pub fn get_rect(&self, x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            data: self.get_rect_data(x, y, width, height),
            palette: self.palette.clone(),
        }
    }

    #[inline]
    pub fn get_rect_from(&mut self, x: u32, y: u32, width: u32, height: u32, other: &IndexedImage) {
        let width = width.min(other.width - x);
        let height = height.min(other.height - y);
        self.width = width;
        self.height = height;
        self.data = other.get_rect_data(x, y, width, height);
    }

    pub fn apply_with_palette(&self, image: &mut RgbImage, palette: &Palette) {
        for (index, pixel) in self.data.iter().cloned().zip(image.data.iter_mut()) {
            *pixel = palette[index];
        }
    }

    pub fn apply(&self, image: &mut RgbImage) {
        for (index, pixel) in self.data.iter().cloned().zip(image.data.iter_mut()) {
            *pixel = self.palette[index];
        }
    }

    pub fn to_rgb_image(&self) -> RgbImage {
        let mut data = unsafe { Box::new_uninit_slice(self.data.len()).assume_init() };

        for (index, pixel) in self.data.iter().cloned().zip(data.iter_mut()) {
            *pixel = self.palette[index];
        }

        RgbImage {
            width: self.width,
            height: self.height,
            data,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32, index: u8) {
        if width == self.width && height == self.height {
            return;
        }

        let size = width as usize * height as usize;
        let mut data: Box<[u8]> = vec![index; size].into();

        for new_y in 0..height {
            let old_offset = new_y as usize * self.width as usize;
            let new_offset = new_y as usize * width as usize;
            data[new_offset..new_offset + width as usize].copy_from_slice(&self.data[old_offset..old_offset + width as usize]);
        }

        self.width = width;
        self.height = height;
        self.data = data;
    }
}

impl From<IndexedImage> for RgbImage {
    #[inline]
    fn from(value: IndexedImage) -> Self {
        value.to_rgb_image()
    }
}

#[derive(Debug, Clone)]
pub struct CycleImage {
    frame_palette: Palette,
    indexed_image: IndexedImage,
    cycles: Box<[Cycle]>,
}

impl CycleImage {
    #[inline]
    pub fn new(indexed_image: IndexedImage, cycles: Box<[Cycle]>) -> Self {
        Self {
            frame_palette: indexed_image.palette.clone(),
            indexed_image,
            cycles,
        }
    }

    #[inline]
    pub fn indexed_image(&self) -> &IndexedImage {
        &self.indexed_image
    }

    #[inline]
    pub fn cycles(&self) -> &[Cycle] {
        &self.cycles
    }

    #[inline]
    pub fn width(&self) -> u32 {
        self.indexed_image.width()
    }

    #[inline]
    pub fn height(&self) -> u32 {
        self.indexed_image.height()
    }

    #[inline]
    pub fn size(&self) -> (u32, u32) {
        (self.width(), self.height())
    }

    #[inline]
    pub fn palette(&self) -> &Palette {
        &self.indexed_image.palette()
    }

    #[inline]
    pub fn palette_mut(&mut self) -> &mut Palette {
        self.indexed_image.palette_mut()
    }

    #[inline]
    pub fn get_index(&self, x: u32, y: u32) -> u8 {
        self.indexed_image().get_index(x, y)
    }

    #[inline]
    pub fn render_frame(&mut self, now: f64, blend: bool, target: &mut RgbImage) {
        self.frame_palette.apply_cycles_from(&self.indexed_image.palette, &self.cycles, now, blend);
        // self.frame_palette.clone_from(&self.indexed_image.palette);
        // self.frame_palette.apply_cycles(&self.cycles, now);
        self.indexed_image.apply_with_palette(target, &self.frame_palette);
    }

    #[inline]
    pub fn get_rect(&self, x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            frame_palette: self.frame_palette.clone(),
            indexed_image: self.indexed_image.get_rect(x, y, width, height),
            cycles: self.cycles.clone(),
        }
    }

    #[inline]
    pub fn get_rect_from(&mut self, x: u32, y: u32, width: u32, height: u32, other: &CycleImage) {
        self.indexed_image.get_rect_from(x, y, width, height, &other.indexed_image);
    }

    #[inline]
    pub fn resize(&mut self, width: u32, height: u32, index: u8) {
        self.indexed_image.resize(width, height, index);
    }
}

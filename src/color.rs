// color-cycle - render color cycle images
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

use std::{fmt::{Debug, Display}, ops::{Index, IndexMut}};

#[derive(Clone, Copy, PartialEq, Eq, Default, Hash)]
#[repr(transparent)]
pub struct Rgb(pub [u8; 3]);

impl Index<usize> for Rgb {
    type Output = u8;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl IndexMut<usize> for Rgb {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut u8 {
        &mut self.0[index]
    }
}

impl From<[u8; 3]> for Rgb {
    #[inline]
    fn from(value: [u8; 3]) -> Self {
        Self(value)
    }
}

impl Display for Rgb {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Rgb([r, g, b]) = *self;
        write!(f, "#{r:02X}{g:02X}{b:02X}")
    }
}

impl Debug for Rgb {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

impl Rgb {
    #[inline]
    pub fn r(&self) -> u8 {
        self.0[0]
    }

    #[inline]
    pub fn g(&self) -> u8 {
        self.0[1]
    }

    #[inline]
    pub fn b(&self) -> u8 {
        self.0[2]
    }
}

pub fn blend(c1: Rgb, c2: Rgb, mid: f64) -> Rgb {
    let Rgb([r1, g1, b1]) = c1;
    let Rgb([r2, g2, b2]) = c2;

    let inv_mid = 1.0 - mid;
    let r = (r1 as f64 * inv_mid + r2 as f64 * mid).round();
    let g = (g1 as f64 * inv_mid + g2 as f64 * mid).round();
    let b = (b1 as f64 * inv_mid + b2 as f64 * mid).round();

    Rgb([r as u8, g as u8, b as u8])
}

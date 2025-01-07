use std::{fmt::Display, ops::{Index, IndexMut}};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
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

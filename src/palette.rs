use std::{fmt::Display, ops::{Index, IndexMut}};

use crate::color::Rgb;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Palette(pub Box<[Rgb; 256]>);

impl Display for Palette {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}", self[0])?;

        for color in &self.0[1..] {
            write!(f, ", {}", color)?;
        }

        "]".fmt(f)
    }
}

impl Index<u8> for Palette {
    type Output = Rgb;

    #[inline]
    fn index(&self, index: u8) -> &Self::Output {
        &self.0[index as usize]
    }
}

impl IndexMut<u8> for Palette {
    #[inline]
    fn index_mut(&mut self, index: u8) -> &mut Rgb {
        &mut self.0[index as usize]
    }
}

impl From<[Rgb; 256]> for Palette {
    #[inline]
    fn from(value: [Rgb; 256]) -> Self {
        Self(value.into())
    }
}

pub const LBM_CYCLE_RATE_DIVISOR: u32 = 280;

impl Palette {
    // TODO: blend mode

    pub fn rotate_right(&mut self, low: u8, high: u8, distance: u32) {
        let slice = &mut self.0[low as usize..high as usize + 1];
        slice.rotate_right(distance as usize);
    }

    pub fn rotate_left(&mut self, low: u8, high: u8, distance: u32) {
        let slice = &mut self.0[low as usize..high as usize + 1];
        slice.rotate_left(distance as usize);
    }

    pub fn apply_cycle(&mut self, cycle: &Cycle, now: f64) {
        let low = cycle.low();
        let high = cycle.high();
        if high > low {
            let size = (high - low + 1) as f64;
            let rate = cycle.rate() as f64 / LBM_CYCLE_RATE_DIVISOR as f64;
            let distance = ((rate * now) % size) as u32;
            //eprintln!("low: {low}, high: {high}, rate: {rate}, size: {size}, rate * now: {}, distance: {distance}", rate * now);
            //let distance = 1;
            if cycle.reverse() {
                self.rotate_left(cycle.low(), cycle.high(), distance);
            } else {
                self.rotate_right(cycle.low(), cycle.high(), distance);
            }
        }
    }

    pub fn apply_cycles(&mut self, cycles: &[Cycle], now: f64) {
        //eprintln!("now: {now}");
        for cycle in cycles {
            self.apply_cycle(cycle, now);
        }
        //eprintln!();
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Cycle {
    low: u8,
    high: u8,
    rate: u32,
    reverse: bool,
}

impl Cycle {
    #[inline]
    pub fn new(low: u8, high: u8, rate: u32, reverse: bool) -> Self {
        Self {
            low,
            high,
            rate,
            reverse,
        }
    }

    #[inline]
    pub fn low(&self) -> u8 {
        self.low
    }

    #[inline]
    pub fn high(&self) -> u8 {
        self.high
    }

    #[inline]
    pub fn rate(&self) -> u32 {
        self.rate
    }

    #[inline]
    pub fn reverse(&self) -> bool {
        self.reverse
    }
}

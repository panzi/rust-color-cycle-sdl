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

// TODO: implement https://moddingwiki.shikadi.net/wiki/LBM_Format

use std::{error::Error, fmt::Display, io::{Read, Seek}, mem::MaybeUninit};

use crate::color::Rgb;

#[derive(Debug)]
pub struct IlbmReadError {
    message: String,
    cause: Option<Box<dyn Error>>
}

impl IlbmReadError {
    #[inline]
    pub fn new<S>(message: S) -> Self
    where S: Into<String> {
        Self {
            message: message.into(),
            cause: None
        }
    }

    #[inline]
    pub fn with_cause<S>(message: S, cause: Box<dyn Error>) -> Self
    where S: Into<String> {
        Self {
            message: message.into(),
            cause: Some(cause)
        }
    }
}

impl Display for IlbmReadError {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(cause) = &self.cause {
            write!(f, "{}: {}", self.message, cause)
        } else {
            self.message.fmt(f)
        }
    }
}

impl Error for IlbmReadError {
    #[inline]
    fn cause(&self) -> Option<&dyn Error> {
        self.cause.as_deref()
    }
}

impl From<std::io::Error> for IlbmReadError {
    #[inline]
    fn from(value: std::io::Error) -> Self {
        Self::with_cause("IO error", Box::new(value))
    }
}

#[derive(Debug)]
pub struct BMHD {
    width: u16,
    height: u16,
    x_origin: i16,
    y_origin: i16,
    num_plans: u8,
    mask: u8,
    compression: u8,
    trans_color: u16,
    x_aspect: u8,
    y_aspect: u8,
    page_width: i16,
    page_heigt: i16,
}


impl BMHD {
    pub const SIZE: u32 = 34;

    #[inline]
    pub fn width(&self) -> u16 {
        self.width
    }

    #[inline]
    pub fn height(&self) -> u16 {
        self.height
    }

    #[inline]
    pub fn x_origin(&self) -> i16 {
        self.x_origin
    }

    #[inline]
    pub fn y_origin(&self) -> i16 {
        self.y_origin
    }

    #[inline]
    pub fn num_plans(&self) -> u8 {
        self.num_plans
    }

    #[inline]
    pub fn mask(&self) -> u8 {
        self.mask
    }

    #[inline]
    pub fn compression(&self) -> u8 {
        self.compression
    }

    #[inline]
    pub fn trans_color(&self) -> u16 {
        self.trans_color
    }

    #[inline]
    pub fn x_aspect(&self) -> u8 {
        self.x_aspect
    }

    #[inline]
    pub fn y_aspect(&self) -> u8 {
        self.y_aspect
    }

    #[inline]
    pub fn page_width(&self) -> i16 {
        self.page_width
    }

    #[inline]
    pub fn page_heigt(&self) -> i16 {
        self.page_heigt
    }

    pub fn read<R>(reader: &mut R, chunk_len: u32) -> Result<Self, IlbmReadError>
    where R: Read + Seek {
        if chunk_len < Self::SIZE {
            return Err(IlbmReadError::new("BMHD chunk too small"));
        }

        let width = read_u16be(reader)?;
        let height = read_u16be(reader)?;
        let x_origin = read_i16be(reader)?;
        let y_origin = read_i16be(reader)?;
        let num_plans = read_u8(reader)?;
        let mask = read_u8(reader)?;
        let compression = read_u8(reader)?;
        let _pad1 = read_u8(reader)?;
        let trans_color = read_u16be(reader)?;
        let x_aspect = read_u8(reader)?;
        let y_aspect = read_u8(reader)?;
        let page_width = read_i16be(reader)?;
        let page_heigt = read_i16be(reader)?;

        if chunk_len > Self::SIZE {
            reader.seek_relative((chunk_len - Self::SIZE).into())?;
        }

        Ok(BMHD {
            width,
            height,
            x_origin,
            y_origin,
            num_plans,
            mask,
            compression,
            trans_color,
            x_aspect,
            y_aspect,
            page_width,
            page_heigt,
        })
    }

}

#[derive(Debug)]
pub struct ILBM {
    header: BMHD,
    body: Option<BODY>,
    cmaps: Vec<CMAP>,
    crngs: Vec<CRNG>,
    ccrts: Vec<CCRT>,
}

impl ILBM {
    pub const MIN_SIZE: u32 = BMHD::SIZE + 12;

    #[inline]
    pub fn header(&self) -> &BMHD {
        &self.header
    }

    #[inline]
    pub fn body(&self) -> Option<&BODY> {
        self.body.as_ref()
    }

    #[inline]
    pub fn cmaps(&self) -> &[CMAP] {
        &self.cmaps
    }

    #[inline]
    pub fn crngs(&self) -> &[CRNG] {
        &self.crngs
    }

    #[inline]
    pub fn ccrts(&self) -> &[CCRT] {
        &self.ccrts
    }

    pub fn read<R>(reader: &mut R) -> Result<ILBM, IlbmReadError>
    where R: Read + Seek {
        let mut fourcc = [0u8; 4];
        reader.read_exact(&mut fourcc)?;

        if fourcc != *b"FORM" {
            return Err(IlbmReadError::new(format!("illegal FOURCC: {:?}", fourcc)));
        }

        let main_chunk_len = read_u32be(reader)?;
        if main_chunk_len <= Self::MIN_SIZE {
            return Err(IlbmReadError::new("file too short"));
        }

        reader.read_exact(&mut fourcc)?;
        if fourcc != *b"ILBM" && fourcc != *b"PBM " {
            return Err(IlbmReadError::new(format!("unsupported file format: {:?}", fourcc)));
        }

        let mut header = None;
        let mut body = None;
        let mut cmaps = Vec::new();
        let mut crngs = Vec::new();
        let mut ccrts = Vec::new();

        let mut pos = 4;
        while pos < main_chunk_len {
            reader.read_exact(&mut fourcc)?;
            let chunk_len = read_u32be(reader)?;

            match &fourcc {
                b"BMHD" => {
                    header = Some(BMHD::read(reader, chunk_len)?);
                }
                b"BODY" => {
                    let Some(header) = &header else {
                        return Err(IlbmReadError::new("BMHD chunk not found before BODY chunk"));
                    };
                    body = Some(BODY::read(reader, chunk_len, header)?);
                }
                b"CMAP" => {
                    cmaps.push(CMAP::read(reader, chunk_len)?);
                }
                b"CRNG" => {
                    crngs.push(CRNG::read(reader, chunk_len)?);
                }
                b"CCRT" => {
                    ccrts.push(CCRT::read(reader, chunk_len)?);
                }
                _ => {
                    // skip unknown chunk
                    reader.seek_relative(chunk_len.into())?;
                }
            }

            if chunk_len & 1 != 0 {
                // Chunks are always padded to an even number of bytes.
                // This padding byte is not included in the chunk size.
                let _pad = read_u8(reader)?;
                pos += 1;
            }

            pos += 8 + chunk_len;
        }

        let Some(header) = header else {
            return Err(IlbmReadError::new("BMHD chunk missing"));
        };

        Ok(Self {
            header,
            body,
            cmaps,
            crngs,
            ccrts,
        })
    }
}

#[derive(Debug)]
pub struct BODY {
    data: Vec<u8>,
}

impl BODY {
    #[inline]
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn read<R>(reader: &mut R, chunk_len: u32, header: &BMHD) -> Result<Self, IlbmReadError>
    where R: Read + Seek {
        // TODO: parse BODY
        Err(IlbmReadError::new("not implemented"))
    }
}

#[derive(Debug)]
pub struct CMAP {
    colors: Vec<Rgb>,
}

impl CMAP {
    #[inline]
    pub fn colors(&self) -> &[Rgb] {
        &self.colors
    }

    pub fn read<R>(reader: &mut R, chunk_len: u32) -> Result<Self, IlbmReadError>
    where R: Read + Seek {
        let num_colors = chunk_len / 3;
        let mut colors = Vec::with_capacity(num_colors as usize);
        let mut buf = [0u8; 3];
        for _ in 0..num_colors {
            reader.read_exact(&mut buf)?;
            colors.push(Rgb(buf.clone()));
        }

        let padding = chunk_len - num_colors * 3;
        if padding > 0 {
            reader.seek_relative(padding.into())?;
        }

        Ok(Self {
            colors
        })
    }
}

#[derive(Debug)]
pub struct CRNG {
    rate: u16,
    flags: u16,
    low: u8,
    high: u8,
}


impl CRNG {
    pub const SIZE: u32 = 14;

    #[inline]
    pub fn rate(&self) -> u16 {
        self.rate
    }

    #[inline]
    pub fn flags(&self) -> u16 {
        self.flags
    }

    #[inline]
    pub fn low(&self) -> u8 {
        self.low
    }

    #[inline]
    pub fn high(&self) -> u8 {
        self.high
    }

    pub fn read<R>(reader: &mut R, chunk_len: u32) -> Result<Self, IlbmReadError>
    where R: Read + Seek {
        if chunk_len < Self::SIZE {
            return Err(IlbmReadError::new("truncated CRNG chunk"));
        }

        let _padding = read_u16be(reader)?;
        let rate = read_u16be(reader)?;
        let flags = read_u16be(reader)?;
        let low = read_u8(reader)?;
        let high = read_u8(reader)?;

        if chunk_len > Self::SIZE {
            reader.seek_relative((chunk_len - Self::SIZE).into())?;
        }

        Ok(Self {
            rate,
            flags,
            low,
            high,
        })
    }
}

#[derive(Debug)]
pub struct CCRT {
    direction: i16,
    low: u8,
    high: u8,
    delay_sec: i32,
    delay_usec: i32,
}

impl CCRT {
    pub const SIZE: u32 = 26;

    #[inline]
    pub fn direction(&self) -> i16 {
        self.direction
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
    pub fn delay_sec(&self) -> i32 {
        self.delay_sec
    }

    #[inline]
    pub fn delay_usec(&self) -> i32 {
        self.delay_usec
    }

    pub fn read<R>(reader: &mut R, chunk_len: u32) -> Result<Self, IlbmReadError>
    where R: Read + Seek {
        if chunk_len < Self::SIZE {
            return Err(IlbmReadError::new("truncated CRNG chunk"));
        }

        let direction = read_i16be(reader)?;
        let low = read_u8(reader)?;
        let high = read_u8(reader)?;
        let delay_sec = read_i32be(reader)?;
        let delay_usec = read_i32be(reader)?;
        let _padding = read_u16be(reader)?;

        if chunk_len > Self::SIZE {
            reader.seek_relative((chunk_len - Self::SIZE).into())?;
        }

        Ok(Self {
            direction,
            low,
            high,
            delay_sec,
            delay_usec,
        })
    }
}

#[inline]
pub fn read_u8(reader: &mut impl Read) -> Result<u8, IlbmReadError> {
    let mut buf = MaybeUninit::<[u8; 1]>::uninit();
    reader.read_exact(unsafe { buf.assume_init_mut() })?;
    let buf = unsafe { buf.assume_init_ref() };
    Ok(buf[0])
}

#[inline]
pub fn read_u32be(reader: &mut impl Read) -> Result<u32, IlbmReadError> {
    let mut buf = MaybeUninit::<[u8; 4]>::uninit();
    reader.read_exact(unsafe { buf.assume_init_mut() })?;
    let buf = unsafe { buf.assume_init_ref() };
    Ok(u32::from_be_bytes(*buf))
}

#[inline]
pub fn read_i32be(reader: &mut impl Read) -> Result<i32, IlbmReadError> {
    let mut buf = MaybeUninit::<[u8; 4]>::uninit();
    reader.read_exact(unsafe { buf.assume_init_mut() })?;
    let buf = unsafe { buf.assume_init_ref() };
    Ok(i32::from_be_bytes(*buf))
}

#[inline]
pub fn read_u16be(reader: &mut impl Read) -> Result<u16, IlbmReadError> {
    let mut buf = MaybeUninit::<[u8; 2]>::uninit();
    reader.read_exact(unsafe { buf.assume_init_mut() })?;
    let buf = unsafe { buf.assume_init_ref() };
    Ok(u16::from_be_bytes(*buf))
}

#[inline]
pub fn read_i16be(reader: &mut impl Read) -> Result<i16, IlbmReadError> {
    let mut buf = MaybeUninit::<[u8; 2]>::uninit();
    reader.read_exact(unsafe { buf.assume_init_mut() })?;
    let buf = unsafe { buf.assume_init_ref() };
    Ok(i16::from_be_bytes(*buf))
}

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

use std::{fmt::Display, io::{Read, Seek}, mem::MaybeUninit};

use crate::{bitvec::BitVec, color::Rgb, image::{CycleImage, IndexedImage}, palette::{Cycle, Palette}};

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ErrorKind {
    UnsupportedFileFormat,
    BrokenFile,
    IO,
}

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    message: String,
    cause: Option<Box<dyn std::error::Error>>
}

impl Error {
    #[inline]
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    #[inline]
    pub fn message(&self) -> &str {
        &self.message
    }

    #[inline]
    pub fn new<S>(kind: ErrorKind, message: S) -> Self
    where S: Into<String> {
        Self {
            kind,
            message: message.into(),
            cause: None
        }
    }

    #[inline]
    pub fn with_cause<S>(kind: ErrorKind, message: S, cause: Box<dyn std::error::Error>) -> Self
    where S: Into<String> {
        Self {
            kind,
            message: message.into(),
            cause: Some(cause)
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

impl Display for Error {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(cause) = &self.cause {
            write!(f, "{}: {}", self.message, cause)
        } else {
            self.message.fmt(f)
        }
    }
}

impl std::error::Error for Error {
    #[inline]
    fn cause(&self) -> Option<&dyn std::error::Error> {
        self.cause.as_deref()
    }
}

impl From<std::io::Error> for Error {
    #[inline]
    fn from(value: std::io::Error) -> Self {
        Self::with_cause(ErrorKind::IO, "IO error", Box::new(value))
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
    pub const SIZE: u32 = 20;

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

    pub fn read<R>(reader: &mut R, chunk_len: u32) -> Result<Self>
    where R: Read + Seek {
        if chunk_len < Self::SIZE {
            return Err(Error::new(ErrorKind::BrokenFile,
                format!("truncated BMHD chunk: {} < {}", chunk_len, Self::SIZE)));
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

    pub fn can_read<R>(reader: &mut R) -> bool
    where R: Read + Seek {
        let mut fourcc = [0u8; 4];
        if reader.read_exact(&mut fourcc).is_err() {
            return false;
        }

        if fourcc != *b"FORM" {
            return false;
        }

        let Ok(main_chunk_len) = read_u32be(reader) else {
            return false;
        };

        if main_chunk_len <= Self::MIN_SIZE {
            return false;
        }

        if reader.read_exact(&mut fourcc).is_err() {
            return false;
        }

        if fourcc != *b"ILBM" && fourcc != *b"PBM " {
            return false;
        }

        true
    }

    pub fn read<R>(reader: &mut R) -> Result<ILBM>
    where R: Read + Seek {
        let mut fourcc = [0u8; 4];
        reader.read_exact(&mut fourcc)?;

        if fourcc != *b"FORM" {
            return Err(Error::new(ErrorKind::UnsupportedFileFormat, format!("illegal FOURCC: {:?}", fourcc)));
        }

        let main_chunk_len = read_u32be(reader)?;
        if main_chunk_len <= Self::MIN_SIZE {
            return Err(Error::new(ErrorKind::UnsupportedFileFormat, "file too short"));
        }

        reader.read_exact(&mut fourcc)?;
        if fourcc != *b"ILBM" && fourcc != *b"PBM " {
            return Err(Error::new(ErrorKind::UnsupportedFileFormat, format!("unsupported file format: {:?}", fourcc)));
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
                        return Err(Error::new(ErrorKind::BrokenFile, "BMHD chunk not found before BODY chunk"));
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
            return Err(Error::new(ErrorKind::BrokenFile, "BMHD chunk missing"));
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
    pixels: Vec<u8>,
    mask: Option<BitVec>,
}

impl BODY {
    #[inline]
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    #[inline]
    pub fn mask(&self) -> Option<&BitVec> {
        self.mask.as_ref()
    }

    pub fn read<R>(reader: &mut R, chunk_len: u32, header: &BMHD) -> Result<Self>
    where R: Read + Seek {
        // TODO: parse BODY
        let num_planes = header.num_plans() as usize;
        let plane_len = (header.width() as usize + 15) / 16 * 2;
        let mut line_len = num_planes * plane_len;
        if header.mask() == 1 {
            line_len += plane_len;
        }
        let mut line = vec![0u8; line_len].into_boxed_slice();

        let data_len = header.height() as usize * line_len;
        let mut pixels = Vec::with_capacity(header.width() as usize * header.height() as usize * num_planes);
        let mut mask = if header.mask() == 1 {
            Some(BitVec::with_capacity(header.width() as usize * header.height() as usize))
        } else {
            None
        };

        fn push_line(pixels: &mut Vec<u8>, mask: &mut Option<BitVec>, line: &[u8], width: u16, plane_len: usize, num_planes: usize) {
            for x in 0..width {
                let byte_offset = (x / 8) as usize;
                let bit_offset = x % 8;
                let mut value = 0u8;
                for plane_index in 0..num_planes {
                    let byte_index = plane_len * plane_index + byte_offset;
                    let bit = (line[byte_index] >> bit_offset) & 1;
                    value |= bit << plane_index;
                }
                pixels.push(value);
            }
            if let Some(mask) = mask {
                let byte_index = plane_len * num_planes;
                let input = &line[byte_index..];
                mask.extend_from_bytes(input, width as usize);
            }
        }

        match header.compression() {
            0 => {
                // uncompressed
                if data_len > chunk_len as usize {
                    return Err(Error::new(ErrorKind::BrokenFile,
                        format!("truncated BODY chunk: {} < {}", chunk_len, data_len)));
                }

                for _y in 0..header.height() {
                    reader.read_exact(&mut line)?;
                    push_line(&mut pixels, &mut mask, &line, header.width(), plane_len, num_planes);
                }

                if data_len < chunk_len as usize {
                    reader.seek_relative((data_len - chunk_len as usize) as i64)?;
                }
            }
            1 => {
                // compressed
                let mut read_len = 0;
                eprintln!("mask: {}, width: {}, height: {}, num_planes: {}", header.mask(), header.width(), header.height(), num_planes);
                for _y in 0..header.height() {
                    let mut pos = 0;

                    while pos < line_len {
                        let cmd = read_u8(reader)?;
                        read_len += 1;
                        if cmd < 128 {
                            let count = read_u8(reader)? as usize + 1;
                            eprintln!("pos: {pos:3}, cmd: {cmd:3} < 128, count: {count}");
                            read_len += 1;
                            let next_pos = pos + count;
                            if next_pos > line_len {
                                return Err(Error::new(ErrorKind::BrokenFile,
                                    format!("[A] broken BODY compression, more date than fits into row: {} > {}", next_pos, line_len)));
                            }
                            reader.read_exact(&mut line[pos..next_pos])?;
                            read_len += count;
                            pos = next_pos;
                        } else if cmd > 128 {
                            let count = 257 - cmd as usize;
                            eprintln!("pos: {pos:3}, cmd: {cmd:3} > 128, count: {count}");
                            let value = read_u8(reader)?;
                            read_len += 1;
                            let next_pos = pos + count;
                            if next_pos > line_len {
                                return Err(Error::new(ErrorKind::BrokenFile,
                                    format!("[B] broken BODY compression, more date than fits into row: {} > {}", next_pos, line_len)));
                            }
                            line[pos..next_pos].fill(value);
                            pos = next_pos;
                        } else {
                            break;
                        }
                        assert!(pos <= line_len);

                        line[pos..].fill(0);
                    }
                    push_line(&mut pixels, &mut mask, &line, header.width(), plane_len, num_planes);
                }

                if read_len > chunk_len as usize {
                    return Err(Error::new(ErrorKind::BrokenFile,
                        format!("truncated compressed BODY chunk: {} < {}", chunk_len, read_len)));
                }

                if read_len < chunk_len as usize {
                    reader.seek_relative((chunk_len as usize - read_len) as i64)?;
                }
            }
            _ => {
                return Err(Error::new(
                    ErrorKind::UnsupportedFileFormat,
                    format!("unsupported compression flag: {}", header.compression())));
            }
        }

        Ok(Self {
            pixels,
            mask,
        })
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

    pub fn read<R>(reader: &mut R, chunk_len: u32) -> Result<Self>
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
    pub const SIZE: u32 = 8;

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

    pub fn read<R>(reader: &mut R, chunk_len: u32) -> Result<Self>
    where R: Read + Seek {
        if chunk_len < Self::SIZE {
            return Err(Error::new(ErrorKind::BrokenFile,
                format!("truncated CRNG chunk: {} < {}", chunk_len, Self::SIZE)));
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
    delay_sec: u32,
    delay_usec: u32,
}

impl CCRT {
    pub const SIZE: u32 = 14;

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
    pub fn delay_sec(&self) -> u32 {
        self.delay_sec
    }

    #[inline]
    pub fn delay_usec(&self) -> u32 {
        self.delay_usec
    }

    pub fn read<R>(reader: &mut R, chunk_len: u32) -> Result<Self>
    where R: Read + Seek {
        if chunk_len < Self::SIZE {
            return Err(Error::new(ErrorKind::BrokenFile,
                format!("truncated CRNG chunk: {} < {}", chunk_len, Self::SIZE)));
        }

        let direction = read_i16be(reader)?;
        if direction < -1 || direction > 1 {
            return Err(Error::new(ErrorKind::BrokenFile,
                format!("invalid CCRT direction: {}", direction)));
        }

        let low = read_u8(reader)?;
        let high = read_u8(reader)?;
        let delay_sec = read_u32be(reader)?;
        let delay_usec = read_u32be(reader)?;
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

impl TryFrom<ILBM> for CycleImage {
    type Error = String;

    fn try_from(ilbm: ILBM) -> std::result::Result<Self, Self::Error> {
        // convert ILBM to LivingWorld
        let header = ilbm.header();
        let width  = header.width()  as u32;
        let height = header.height() as u32;
        let mut cycles = Vec::with_capacity(ilbm.ccrts().len() + ilbm.crngs().len());
        let body = ilbm.body();
        let palette = if let Some(cmap) = ilbm.cmaps().first() {
            cmap.colors().into()
        } else {
            Palette::default()
        };

        let indexed_image = if let Some(body) = body {
            if let Some(indexed_image) = IndexedImage::from_buffer(width, height, body.pixels().into(), palette) {
                indexed_image
            } else {
                return Err("image buffer is too small for given width/height".to_owned());
            }
        } else {
            IndexedImage::new(width, height, palette)
        };

        for crng in ilbm.crngs() {
            if crng.flags() & 1 != 0 {
                cycles.push(Cycle::new(
                    crng.low(),
                    crng.high(),
                    crng.rate() as u32,
                    crng.flags() & 2 != 0
                ));
            }
        }

        for ccrt in ilbm.ccrts() {
            if ccrt.direction() != 0 {
                let usec = ccrt.delay_sec() as u64 * 1000_000 + ccrt.delay_usec() as u64;

                // 1s / 60 = 16384x
                // 1s * 1000_000 = ?x

                // 16384s / 60 = 1x

                // Is this correct?
                // See: https://moddingwiki.shikadi.net/wiki/LBM_Format#CRNG:_Colour_range

                let rate = usec * 16384 / (60 * 1000_000);

                cycles.push(Cycle::new(
                    ccrt.low(),
                    ccrt.high(),
                    rate as u32,
                    ccrt.direction() == -1,
                ));
            }
        }

        Ok(CycleImage::new(None, indexed_image, cycles.into()))
    }
}

#[inline]
pub fn read_u8(reader: &mut impl Read) -> Result<u8> {
    let mut buf = MaybeUninit::<[u8; 1]>::uninit();
    reader.read_exact(unsafe { buf.assume_init_mut() })?;
    let buf = unsafe { buf.assume_init_ref() };
    Ok(buf[0])
}

#[inline]
pub fn read_u32be(reader: &mut impl Read) -> Result<u32> {
    let mut buf = MaybeUninit::<[u8; 4]>::uninit();
    reader.read_exact(unsafe { buf.assume_init_mut() })?;
    let buf = unsafe { buf.assume_init_ref() };
    Ok(u32::from_be_bytes(*buf))
}

#[inline]
pub fn read_i32be(reader: &mut impl Read) -> Result<i32> {
    let mut buf = MaybeUninit::<[u8; 4]>::uninit();
    reader.read_exact(unsafe { buf.assume_init_mut() })?;
    let buf = unsafe { buf.assume_init_ref() };
    Ok(i32::from_be_bytes(*buf))
}

#[inline]
pub fn read_u16be(reader: &mut impl Read) -> Result<u16> {
    let mut buf = MaybeUninit::<[u8; 2]>::uninit();
    reader.read_exact(unsafe { buf.assume_init_mut() })?;
    let buf = unsafe { buf.assume_init_ref() };
    Ok(u16::from_be_bytes(*buf))
}

#[inline]
pub fn read_i16be(reader: &mut impl Read) -> Result<i16> {
    let mut buf = MaybeUninit::<[u8; 2]>::uninit();
    reader.read_exact(unsafe { buf.assume_init_mut() })?;
    let buf = unsafe { buf.assume_init_ref() };
    Ok(i16::from_be_bytes(*buf))
}

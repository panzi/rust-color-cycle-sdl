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

use std::fmt::Display;

use sdl2::{render::TextureValueError, ttf::FontError, video::WindowBuildError, IntegerOrSdlError};

#[derive(Debug)]
pub struct Error {
    message: String,
    source: Option<Box<dyn std::error::Error>>,
}

impl Error {
    pub fn new<S>(message: S) -> Self
    where S: Into<String> {
        Self {
            message: message.into(),
            source: None,
        }
    }

    pub fn with_source<S>(message: S, source: Box<dyn std::error::Error>) -> Self
    where S: Into<String> {
        Self {
            message: message.into(),
            source: Some(source),
        }
    }
}

impl Display for Error {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(source) = &self.source {
            write!(f, "{}: {source}", self.message)
        } else {
            self.message.fmt(f)
        }
    }
}

impl std::error::Error for Error {
    #[inline]
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_deref()
    }
}

impl From<crate::ilbm::Error> for Error {
    #[inline]
    fn from(value: crate::ilbm::Error) -> Self {
        Self::with_source("ILBM error", Box::new(value))
    }
}

impl From<std::io::Error> for Error {
    #[inline]
    fn from(value: std::io::Error) -> Self {
        Self::with_source("IO error", Box::new(value))
    }
}

impl From<serde_json::error::Error> for Error {
    #[inline]
    fn from(value: serde_json::error::Error) -> Self {
        Self::with_source("JSON error", Box::new(value))
    }
}

impl From<TextureValueError> for Error {
    #[inline]
    fn from(value: TextureValueError) -> Self {
        Self::with_source("Texture value error", Box::new(value))
    }
}

impl From<FontError> for Error {
    #[inline]
    fn from(value: FontError) -> Self {
        Self::with_source("Font error", Box::new(value))
    }
}

impl From<WindowBuildError> for Error {
    #[inline]
    fn from(value: WindowBuildError) -> Self {
        Self::with_source("Window build error", Box::new(value))
    }
}

impl From<IntegerOrSdlError> for Error {
    #[inline]
    fn from(value: IntegerOrSdlError) -> Self {
        match &value {
            IntegerOrSdlError::IntegerOverflows(_, _) => {
                Self::with_source("Integer overflow error", Box::new(value))
            }
            IntegerOrSdlError::SdlError(_) => {
                Self::with_source("SDL error", Box::new(value))
            }
        }
    }
}


impl From<String> for Error {
    #[inline]
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

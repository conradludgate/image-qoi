use std::{num::Wrapping, mem::MaybeUninit};

use image::error::{DecodingError, ImageFormatHint};

mod decoder;
mod reader;
pub use {decoder::QoiDecoder, reader::QoiReader};

#[repr(C)]
#[derive(PartialEq, Eq, Clone, Copy)]
pub struct Rgba([Wrapping<u8>; 4]);

impl Rgba {
    const ZERO: Self = Self([Wrapping(0); 4]);
    const INIT: Self = Self([Wrapping(0), Wrapping(0), Wrapping(0), Wrapping(255)]);
    fn alpha(self) -> Wrapping<u8> {
        self.0[3]
    }
    fn hash(self) -> u8 {
        let Self([r, g, b, a]) = self;
        (r * Wrapping(3) + g * Wrapping(5) + b * Wrapping(7) + a * Wrapping(11)).0 % 64
    }
    fn bytes(self) -> [u8; 4] {
        let Self([Wrapping(r), Wrapping(g), Wrapping(b), Wrapping(a)]) = self;
        [r, g, b, a]
    }
    fn from_bytes(bytes: [u8; 4]) -> Self {
        let [r, g, b, a] = bytes;
        Self([Wrapping(r), Wrapping(g), Wrapping(b), Wrapping(a)])
    }
}

#[repr(C)]
struct QoiHeader {
    magic: [u8; 4], // magic bytes "qoif"
    width: u32,     // image width in pixels (BE)
    height: u32,    // image height in pixels (BE)
    channels: u8,   // 3 = RGB, 4 = RGBA
    colorspace: u8, // 0 = sRGB with linear alpha
                    // 1 = all channels linear
}

impl QoiHeader {
    fn is_rgba(&self) -> bool {
        self.channels == 4
    }
}

impl TryFrom<&[u8]> for QoiHeader {
    type Error = image::error::DecodingError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() < 14 {
            return Err(DecodingError::new(
                ImageFormatHint::Unknown,
                "not enough bytes for header",
            ));
        }

        let mut this = unsafe {
            let mut this = MaybeUninit::<QoiHeader>::uninit();
            this.as_mut_ptr()
                .cast::<u8>()
                .copy_from_nonoverlapping(value.as_ptr(), 14);
            this.assume_init()
        };

        if &this.magic != b"qoif" {
            return Err(DecodingError::new(
                ImageFormatHint::Unknown,
                "qoif magic header not found",
            ));
        }

        this.width = u32::from_be(this.width);
        this.height = u32::from_be(this.height);

        Ok(this)
    }
}

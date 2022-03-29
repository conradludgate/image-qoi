use std::num::Wrapping;

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

use std::{
    io::{BufReader, Read},
    mem::MaybeUninit,
    num::Wrapping,
};

use image::{
    error::{DecodingError, ImageFormatHint},
    ImageDecoder, ImageError, ImageResult,
};

#[repr(C)]
pub struct QoiHeader {
    magic: [u8; 4], // magic bytes "qoif"
    width: u32,     // image width in pixels (BE)
    height: u32,    // image height in pixels (BE)
    channels: u8,   // 3 = RGB, 4 = RGBA
    colorspace: u8, // 0 = sRGB with linear alpha
                    // 1 = all channels linear
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

pub struct QoiDecoder<R> {
    header: QoiHeader,
    buffer: BufReader<R>,
}

impl<R: Read> QoiDecoder<R> {
    pub fn new(read: R) -> ImageResult<Self> {
        let mut buffer = BufReader::new(read);
        let mut header_bytes = [0; 14];
        buffer.read_exact(&mut header_bytes)?;
        let header = QoiHeader::try_from(&header_bytes[..]).map_err(ImageError::Decoding)?;
        Ok(Self { header, buffer })
    }
}

impl<'a, R: Read + 'a> ImageDecoder<'a> for QoiDecoder<R> {
    type Reader = QoiReader<R>;

    fn dimensions(&self) -> (u32, u32) {
        (self.header.width, self.header.height)
    }

    fn color_type(&self) -> image::ColorType {
        // if self.header.channels == 3 {
        //     image::ColorType::Rgb8
        // } else {
        image::ColorType::Rgba8
        // }
    }

    fn into_reader(self) -> ImageResult<Self::Reader> {
        Ok(QoiReader {
            buffer: self.buffer,
            pixels: [Rgba::ZERO; 64],
            latest: Rgba::INIT,
            remain: QoiRemaining {
                bytes: [0; 4],
                count: 0,
            },
        })
    }
}

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

pub struct QoiReader<R> {
    buffer: BufReader<R>,
    pixels: [Rgba; 64],
    latest: Rgba,
    remain: QoiRemaining,
}

// we don't always have the liberty of writing all the data we have
// since the buffer may be full, so this is a way that data compactly
struct QoiRemaining {
    bytes: [u8; 4],
    count: usize,
}

impl Read for QoiRemaining {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = buf.len().min(self.count);
        if n == 0 {
            return Ok(0);
        }
        let mut i = 0;
        loop {
            let j = i + 4;
            if j >= n {
                break;
            }
            buf[i..j].copy_from_slice(&self.bytes);
            i = j;
        }

        let rem = n - i;
        if rem > 0 {
            buf[i..n].copy_from_slice(&self.bytes[..rem]);
            self.bytes.rotate_left(rem);
        }

        self.count -= n;
        Ok(n)
    }
}

impl<R: Read> Read for QoiReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.remain.count == 0 {
            let remain = match self.read_tag()? {
                0b1111_1111 => self.read_rgba()?,
                0b1111_1110 => self.read_rgb()?,
                tag => match tag >> 6 {
                    0b11 => {
                        let run = (tag & 0b0011_1111) + 1;
                        QoiRemaining {
                            bytes: self.latest.bytes(),
                            count: run as usize * 4,
                        }
                    }
                    0b10 => {
                        let dg = Wrapping(tag & 0b0011_1111) - Wrapping(32);
                        let dr_db = self.read_tag()?;
                        let dr_dg = Wrapping(dr_db >> 4) - Wrapping(8);
                        let db_dg = Wrapping(dr_db & 0b0000_1111) - Wrapping(8);
                        let dr = dr_dg + dg;
                        let db = db_dg + dg;
                        let Rgba([r, g, b, a]) = self.latest;

                        self.save_pixel(Rgba([r + dr, g + dg, b + db, a]))
                    }
                    0b01 => {
                        let dr = Wrapping((tag >> 4) & 0b0011) - Wrapping(2);
                        let dg = Wrapping((tag >> 2) & 0b0011) - Wrapping(2);
                        let db = Wrapping(tag & 0b0011) - Wrapping(2);
                        let Rgba([r, g, b, a]) = self.latest;

                        self.save_pixel(Rgba([r + dr, g + dg, b + db, a]))
                    }
                    _ => {
                        let index = tag & 0b0011_1111;
                        let pixel = self.pixels[index as usize];
                        self.save_pixel(pixel)
                    }
                },
            };
            self.remain = remain;
        }
        self.remain.read(buf)
    }
}

impl<R: Read> QoiReader<R> {
    fn save_pixel(&mut self, pixel: Rgba) -> QoiRemaining {
        self.latest = pixel;
        self.pixels[pixel.hash() as usize] = pixel;
        QoiRemaining {
            bytes: pixel.bytes(),
            count: 4,
        }
    }
    fn read_tag(&mut self) -> std::io::Result<u8> {
        let mut tag = [0; 1];
        self.buffer.read_exact(&mut tag)?;
        Ok(tag[0])
    }
    fn read_rgba(&mut self) -> std::io::Result<QoiRemaining> {
        let mut rgba = [0; 4];
        self.buffer.read_exact(&mut rgba)?;
        Ok(self.save_pixel(Rgba::from_bytes(rgba)))
    }
    fn read_rgb(&mut self) -> std::io::Result<QoiRemaining> {
        let mut rgba = [0, 0, 0, self.latest.alpha().0];
        self.buffer.read_exact(&mut rgba[0..3])?;
        Ok(self.save_pixel(Rgba::from_bytes(rgba)))
    }
}

#[cfg(test)]
mod tests {
    use std::{fs::File, path::PathBuf};

    use image::{codecs::png::PngDecoder, DynamicImage};
    use test_case::test_case;

    use crate::QoiDecoder;

    #[test_case("dice")]
    #[test_case("kodim10")]
    #[test_case("kodim23")]
    #[test_case("qoi_logo")]
    #[test_case("testcard_rgba")]
    #[test_case("testcard")]
    #[test_case("wikipedia_008")]
    fn validate(file: &str) {
        let base = PathBuf::from("qoi_test_images");

        let png = base.join(file).with_extension("png");
        let png = File::open(png).unwrap();
        let png = PngDecoder::new(png).unwrap();
        let png = DynamicImage::from_decoder(png).unwrap().into_rgba8();

        let qoi = base.join(file).with_extension("qoi");
        let qoi = File::open(qoi).unwrap();
        let qoi = QoiDecoder::new(qoi).unwrap();
        let qoi = DynamicImage::from_decoder(qoi).unwrap().into_rgba8();

        assert_eq!(qoi, png);
    }
}

use std::{
    io::{BufReader, Read},
    mem::MaybeUninit,
};

use image::{
    error::{DecodingError, ImageFormatHint},
    ImageDecoder, ImageError, ImageResult,
};

use crate::QoiReader;

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
        image::ColorType::Rgba8
    }

    fn into_reader(self) -> ImageResult<Self::Reader> {
        Ok(QoiReader::new(self.buffer))
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

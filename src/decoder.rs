use std::io::{BufReader, Read};

use image::{ImageDecoder, ImageError, ImageResult, Progress};

use crate::{QoiHeader, QoiReader};

/// An [`ImageDecoder`] for the [Quite Ok Image Format](https://qoiformat.org).
///
/// ```
/// # use std::fs::File;
/// use image::DynamicImage;
/// use image_qoi::QoiDecoder;
///
/// # fn main() -> image::ImageResult<()> {
/// let file = File::open("qoi_test_images/dice.qoi")?;
/// let decoder = QoiDecoder::new(file)?;
/// let image = DynamicImage::from_decoder(decoder)?;
/// # Ok(())
/// # }
/// ```
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
        if self.header.is_rgba() {
            image::ColorType::Rgba8
        } else {
            image::ColorType::Rgb8
        }
    }

    fn into_reader(self) -> ImageResult<Self::Reader> {
        Ok(QoiReader::new(self.header, self.buffer))
    }

    fn scanline_bytes(&self) -> u64 {
        self.color_type().bytes_per_pixel() as u64
    }

    fn read_image_with_progress<F: Fn(Progress)>(
        self,
        mut buf: &mut [u8],
        _progress_callback: F,
    ) -> ImageResult<()> {
        let total_bytes = self.total_bytes() as usize;
        assert_eq!(buf.len(), total_bytes);

        let mut reader = self.into_reader()?;

        while !buf.is_empty() {
            let pixel = reader.load_next_pixel()?;
            for _ in 0..(pixel.count / pixel.chans) {
                buf[..pixel.chans].copy_from_slice(&pixel.bytes[..pixel.chans]);
                buf = &mut buf[pixel.chans..]
            }
        }

        Ok(())
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

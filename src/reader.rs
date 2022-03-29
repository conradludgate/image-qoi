use std::{
    io::{BufReader, Read},
    num::Wrapping,
};

use crate::Rgba;

pub struct QoiReader<R> {
    buffer: BufReader<R>,
    pixels: [Rgba; 64],
    latest: Rgba,
    remain: QoiRemaining,
}

impl<R> QoiReader<R> {
    pub(crate) fn new(buffer: BufReader<R>) -> Self {
        Self {
            buffer,
            pixels: [Rgba::ZERO; 64],
            latest: Rgba::INIT,
            remain: QoiRemaining {
                bytes: [0; 4],
                count: 0,
            },
        }
    }
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

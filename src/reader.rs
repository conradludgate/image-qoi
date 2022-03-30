use std::{
    io::{BufReader, Read},
    num::Wrapping,
};

use crate::{QoiHeader, Rgba};

pub struct QoiReader<R> {
    header: QoiHeader,
    buffer: BufReader<R>,
    pixels: [Rgba; 64],
    latest: Rgba,
    remain: QoiRemaining,
}

impl<R> QoiReader<R> {
    pub(crate) fn new(header: QoiHeader, buffer: BufReader<R>) -> Self {
        let chans = header.channels as usize;
        Self {
            header,
            buffer,
            pixels: [Rgba::ZERO; 64],
            latest: Rgba::INIT,
            remain: QoiRemaining {
                bytes: [0; 4],
                count: 0,
                chans,
            },
        }
    }
}

/// we don't always have the liberty of writing all the data we have
/// since the buffer may be full, so this is a way that data compactly
pub(crate) struct QoiRemaining {
    pub(crate) bytes: [u8; 4],
    pub(crate) chans: usize,
    pub(crate) count: usize,
}

impl Read for QoiRemaining {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let b = &mut self.bytes[..self.chans];

        let n = buf.len().min(self.count);
        if n == 0 {
            return Ok(0);
        }
        let mut i = 0;
        loop {
            let j = i + self.chans;
            if j >= n {
                break;
            }
            buf[i..j].copy_from_slice(b);
            i = j;
        }

        let rem = n - i;
        if rem > 0 {
            buf[i..n].copy_from_slice(&b[..rem]);
            b.rotate_left(rem);
        }

        self.count -= n;
        Ok(n)
    }
}

impl<R: Read> Read for QoiReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.remain.count == 0 {
            self.remain = self.load_next_pixel()?;
        }
        self.remain.read(buf)
    }
}

impl<R: Read> QoiReader<R> {
    pub(crate) fn load_next_pixel(&mut self) -> std::io::Result<QoiRemaining> {
        match self.read_tag()? {
            0b1111_1111 => self.read_rgba(),
            0b1111_1110 => self.read_rgb(),
            tag => match tag >> 6 {
                0b11 => {
                    let run = (tag & 0b0011_1111) + 1;
                    Ok(QoiRemaining {
                        bytes: self.latest.bytes(),
                        chans: self.header.channels as usize,
                        count: run as usize * self.header.channels as usize,
                    })
                }
                0b10 => {
                    let dg = Wrapping(tag & 0b0011_1111) - Wrapping(32);
                    let dr_db = self.read_tag()?;
                    let dr_dg = Wrapping(dr_db >> 4) - Wrapping(8);
                    let db_dg = Wrapping(dr_db & 0b0000_1111) - Wrapping(8);
                    let dr = dr_dg + dg;
                    let db = db_dg + dg;
                    let Rgba([r, g, b, a]) = self.latest;

                    Ok(self.save_pixel(Rgba([r + dr, g + dg, b + db, a])))
                }
                0b01 => {
                    let dr = Wrapping((tag >> 4) & 0b0011) - Wrapping(2);
                    let dg = Wrapping((tag >> 2) & 0b0011) - Wrapping(2);
                    let db = Wrapping(tag & 0b0011) - Wrapping(2);
                    let Rgba([r, g, b, a]) = self.latest;

                    Ok(self.save_pixel(Rgba([r + dr, g + dg, b + db, a])))
                }
                _ => {
                    let index = tag & 0b0011_1111;
                    let pixel = self.pixels[index as usize];
                    Ok(self.save_pixel(pixel))
                }
            },
        }
    }

    fn save_pixel(&mut self, pixel: Rgba) -> QoiRemaining {
        self.latest = pixel;
        self.pixels[pixel.hash() as usize] = pixel;
        QoiRemaining {
            bytes: pixel.bytes(),
            chans: self.header.channels as usize,
            count: self.header.channels as usize,
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

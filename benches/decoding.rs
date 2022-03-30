use std::io::Cursor;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use image::{DynamicImage, codecs::png::PngDecoder};
use image_qoi::QoiDecoder;

static QOI: &[&[u8]] = &[
    include_bytes!("../qoi_test_images/dice.qoi").as_slice(),
    include_bytes!("../qoi_test_images/kodim10.qoi").as_slice(),
    include_bytes!("../qoi_test_images/kodim23.qoi").as_slice(),
    include_bytes!("../qoi_test_images/qoi_logo.qoi").as_slice(),
    include_bytes!("../qoi_test_images/testcard_rgba.qoi").as_slice(),
    include_bytes!("../qoi_test_images/testcard.qoi").as_slice(),
    include_bytes!("../qoi_test_images/wikipedia_008.qoi").as_slice(),
];
static PNG: &[&[u8]] = &[
    include_bytes!("../qoi_test_images/dice.png").as_slice(),
    include_bytes!("../qoi_test_images/kodim10.png").as_slice(),
    include_bytes!("../qoi_test_images/kodim23.png").as_slice(),
    include_bytes!("../qoi_test_images/qoi_logo.png").as_slice(),
    include_bytes!("../qoi_test_images/testcard_rgba.png").as_slice(),
    include_bytes!("../qoi_test_images/testcard.png").as_slice(),
    include_bytes!("../qoi_test_images/wikipedia_008.png").as_slice(),
];

pub fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("image-qoi", |b| {
        b.iter(|| {
            for file in QOI {
                let cursor = Cursor::new(black_box(file));
                let decoder = QoiDecoder::new(cursor).unwrap();
                let image = DynamicImage::from_decoder(decoder).unwrap();
                black_box(image);
            }
        })
    });

    c.bench_function("rapid-qoi", |b| {
        b.iter(|| {
            for file in QOI {
                let image = rapid_qoi::Qoi::decode_alloc(black_box(file)).unwrap();
                black_box(image);
            }
        })
    });

    c.bench_function("image-png", |b| {
        b.iter(|| {
            for file in PNG {
                let cursor = Cursor::new(black_box(file));
                let decoder = PngDecoder::new(cursor).unwrap();
                let image = DynamicImage::from_decoder(decoder).unwrap();
                black_box(image);
            }
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

# image-qoi

[Quote Ok Image format](https://qoiformat.org/) support within the [image](https://docs.rs/image/0.24.1/image/index.html) crate.

## Usage

```rust
let file = File::open("qoi_test_images/dice.qoi").unwrap();
let decoder = QoiDecoder::new(file).unwrap();
let image = DynamicImage::from_decoder(decoder).unwrap();
```

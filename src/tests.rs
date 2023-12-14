use std::error::Error;

use image::{codecs::qoi::QoiDecoder as ReferenceQoiDecoder, DynamicImage};

use super::*;

#[test]
fn decode_valid_header() {
    let header = [
        b"qoif",
        &42u32.to_be_bytes(),
        &69u32.to_be_bytes(),
        [3u8, 0u8].as_slice(),
    ]
    .concat();
    let parsed = parse_image_header(&header).unwrap();
    assert_eq!(
        parsed,
        Header {
            width: 42,
            height: 69,
            channels: Channels::Rgb,
            colorspace: Colorspace::Srgb
        }
    );

    let header = [
        b"qoif",
        &42u32.to_be_bytes(),
        &69u32.to_be_bytes(),
        [4u8, 1u8].as_slice(),
    ]
    .concat();
    let parsed = parse_image_header(&header).unwrap();
    assert_eq!(
        parsed,
        Header {
            width: 42,
            height: 69,
            channels: Channels::Rgba,
            colorspace: Colorspace::Linear
        }
    )
}

#[test]
fn decode_real_image_header() {
    let image_bytes = include_bytes!("../test_images/qoi_logo.qoi").as_slice();
    let decoder = QoiDecoder::new(Cursor::new(image_bytes)).unwrap();
    assert_eq!(decoder.dimensions(), (448, 220));
    assert_eq!(decoder.color_type(), ColorType::Rgba8);
}

#[test]
#[should_panic]
fn decode_invalid_colorspace() {
    let header = [
        b"qoif",
        &42u32.to_be_bytes(),
        &69u32.to_be_bytes(),
        [3u8, 2u8].as_slice(),
    ]
    .concat();
    parse_image_header(&header).unwrap();
}

#[test]
#[should_panic]
fn decode_invalid_channels() {
    let header = [
        b"qoif",
        &42u32.to_be_bytes(),
        &69u32.to_be_bytes(),
        [5u8, 0u8].as_slice(),
    ]
    .concat();
    parse_image_header(&header).unwrap();
}

#[test]
#[should_panic]
fn decode_invalid_magic() {
    let header = [
        b"qoi#",
        &42u32.to_be_bytes(),
        &69u32.to_be_bytes(),
        [3u8, 0u8].as_slice(),
    ]
    .concat();
    parse_image_header(&header).unwrap();
}

#[test]
#[ignore]
fn decode_testcard() {
    let image_bytes = include_bytes!("../test_images/testcard.qoi").as_slice();
    test_decoding_correctness(image_bytes);
}

fn test_decoding_correctness(image_bytes: &[u8]) {
    let reference_image = reference_decode(image_bytes)
        .expect("There should be no errors in reference implemenation");
    let decoded_image = decode(image_bytes);
    assert!(
        decoded_image == reference_image,
        "Decoded image differs from the reference"
    );
}

fn decode(image_bytes: &[u8]) -> DynamicImage {
    let decoder = QoiDecoder::new(Cursor::new(image_bytes)).unwrap();
    DynamicImage::from_decoder(decoder).unwrap()
}

fn reference_decode(image_bytes: &[u8]) -> Result<DynamicImage, Box<dyn Error>> {
    let decoder = ReferenceQoiDecoder::new(Cursor::new(image_bytes))?;
    Ok(DynamicImage::from_decoder(decoder)?)
}

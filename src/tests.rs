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
fn decode_qoi_op_run() {
    let image_bytes = [
        b"qoif",
        &1u32.to_be_bytes(),
        &1u32.to_be_bytes(),
        [3u8, 0].as_slice(),
        &[0b11000000],
        &[0u8, 0, 0, 0, 0, 0, 0, 1],
    ]
    .concat();
    test_decoding_correctness(&image_bytes);
}

#[test]
fn decode_qoi_op_index() {
    let image_bytes = [
        b"qoif",
        &2u32.to_be_bytes(),
        &1u32.to_be_bytes(),
        [3u8, 0].as_slice(),
        &[0b11111110, 127, 127, 127],
        &[0b00110001],
        &[0, 0, 0, 0, 0, 0, 0, 1],
    ]
    .concat();
    test_decoding_correctness(&image_bytes);
}

#[test]
fn decode_qoi_op_diff() {
    let image_bytes = [
        b"qoif",
        &2u32.to_be_bytes(),
        &1u32.to_be_bytes(),
        [3u8, 0].as_slice(),
        &[0b11111110, 127, 127, 127],
        &[0b01000111],
        &[0, 0, 0, 0, 0, 0, 0, 1],
    ]
    .concat();
    test_decoding_correctness(&image_bytes);
}

#[test]
fn decode_qoi_op_luma() {
    let image_bytes = [
        b"qoif",
        &2u32.to_be_bytes(),
        &1u32.to_be_bytes(),
        [3u8, 0].as_slice(),
        &[0b11111110, 127, 127, 127],
        &[0b10001010, 0b11110001],
        &[0, 0, 0, 0, 0, 0, 0, 1],
    ]
    .concat();
    test_decoding_correctness(&image_bytes);
}

#[test]
fn decode_dice() {
    let image_bytes = include_bytes!("../test_images/dice.qoi").as_slice();
    test_decoding_correctness(image_bytes);
}

// qoi library from image-rs decodes edgecase incorrectly,
// but my program does it correctly, so the results are different
#[test]
#[ignore]
fn decode_edgecase() {
    let image_bytes = include_bytes!("../test_images/edgecase.qoi").as_slice();
    test_decoding_correctness(image_bytes);
}

#[test]
fn decode_kodim10() {
    let image_bytes = include_bytes!("../test_images/kodim10.qoi").as_slice();
    test_decoding_correctness(image_bytes);
}

#[test]
fn decode_kodim23() {
    let image_bytes = include_bytes!("../test_images/kodim23.qoi").as_slice();
    test_decoding_correctness(image_bytes);
}

#[test]
fn decode_qoi_logo() {
    let image_bytes = include_bytes!("../test_images/qoi_logo.qoi").as_slice();
    test_decoding_correctness(image_bytes);
}

#[test]
fn decode_testcard_rgba() {
    let image_bytes = include_bytes!("../test_images/testcard_rgba.qoi").as_slice();
    test_decoding_correctness(image_bytes);
}

#[test]
fn decode_testcard() {
    let image_bytes = include_bytes!("../test_images/testcard.qoi").as_slice();
    test_decoding_correctness(image_bytes);
}

#[test]
fn decode_wikipedia_008() {
    let image_bytes = include_bytes!("../test_images/wikipedia_008.qoi").as_slice();
    test_decoding_correctness(image_bytes);
}

fn test_decoding_correctness(image_bytes: &[u8]) {
    let reference_image = reference_decode(image_bytes)
        .expect("There should be no errors in reference implemenation");
    let decoded_image = decode(image_bytes);
    // decoded_image
    //     .save(format!("decoded{}.png", image_bytes[16]))
    //     .unwrap();
    // reference_image
    //     .save(format!("ref_decoded{}.png", image_bytes[16]))
    //     .unwrap();
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

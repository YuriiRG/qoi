use std::error::Error;

use image::{codecs::qoi::QoiDecoder as ReferenceQoiDecoder, DynamicImage};

use super::*;

#[test]
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

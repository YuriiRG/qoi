use std::io::{Cursor, Read};

use image::{
    error::{DecodingError, ImageFormatHint},
    ColorType, ImageDecoder, ImageError, ImageFormat, ImageResult,
};

const QOIF_HEADER_LENGTH: usize = 14;

#[cfg(test)]
mod tests;

mod parser;

pub use parser::*;

pub struct QoiDecoder<R> {
    reader: R,
    header: Header,
}

impl<R: Read> QoiDecoder<R> {
    pub fn new(mut reader: R) -> ImageResult<Self> {
        let mut header_buf = [0u8; QOIF_HEADER_LENGTH];
        reader
            .read_exact(&mut header_buf)
            .map_err(ImageError::IoError)?;
        Ok(QoiDecoder {
            header: parse_image_header(&header_buf).map_err(decoding_error)?,
            reader,
        })
    }
}

#[derive(Clone, Copy, Debug)]
struct ParserState {
    prev: Pixel,
    seen: [Pixel; 64],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
struct Pixel {
    red: u8,
    green: u8,
    blue: u8,
    alpha: u8,
}

impl<R: Read> ImageDecoder<'_> for QoiDecoder<R> {
    type Reader = Cursor<Vec<u8>>;

    fn color_type(&self) -> ColorType {
        match self.header.channels {
            Channels::Rgb => ColorType::Rgb8,
            Channels::Rgba => ColorType::Rgba8,
        }
    }

    fn dimensions(&self) -> (u32, u32) {
        (self.header.width, self.header.height)
    }

    fn into_reader(mut self) -> ImageResult<Self::Reader> {
        let mut input_buf = vec![];
        self.reader
            .read_to_end(&mut input_buf)
            .map_err(ImageError::IoError)?;
        let raw_content = parse_image_content(&input_buf, self.header).map_err(decoding_error)?;
        Ok(Cursor::new(raw_content))
    }
}

fn decoding_error(err: DecoderError) -> ImageError {
    ImageError::Decoding(DecodingError::new(
        ImageFormatHint::Exact(ImageFormat::Qoi),
        err,
    ))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    width: u32,
    height: u32,
    channels: Channels,
    colorspace: Colorspace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum Channels {
    Rgb = 3,
    Rgba = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum Colorspace {
    Srgb = 0,
    Linear = 1,
}

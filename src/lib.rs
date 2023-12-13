use std::io::{Cursor, Read, Seek};

use image::{
    error::{DecodingError, ImageFormatHint},
    ColorType, ImageDecoder, ImageError, ImageFormat, ImageResult,
};

#[cfg(test)]
mod tests;

pub struct QoiDecoder<R> {
    reader: R,
    header: Header,
}

impl<R: Read + Seek> QoiDecoder<R> {
    pub fn new(mut reader: R) -> ImageResult<Self> {
        Ok(QoiDecoder {
            header: parse_image_header(&mut reader).map_err(decoding_error)?,
            reader,
        })
    }
}

// TODO: add proper error type
fn parse_image_header(reader: impl Read) -> Result<Header, &'static str> {
    todo!()
}

// TODO: add proper error type
fn parse_image_content(reader: impl Read) -> Result<Vec<u8>, &'static str> {
    todo!()
}

impl<R: Read + Seek> ImageDecoder<'_> for QoiDecoder<R> {
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

    fn into_reader(self) -> ImageResult<Self::Reader> {
        let raw_content = parse_image_content(self.reader).map_err(decoding_error)?;
        Ok(Cursor::new(raw_content))
    }
}

// TODO: replace with proper error type
fn decoding_error(err: &str) -> ImageError {
    ImageError::Decoding(DecodingError::new(
        ImageFormatHint::Exact(ImageFormat::Qoi),
        err,
    ))
}

struct Header {
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

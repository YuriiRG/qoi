use std::io::{Cursor, Read};

use image::{
    error::{DecodingError, ImageFormatHint},
    ColorType, ImageDecoder, ImageError, ImageFormat, ImageResult,
};

use winnow::{
    binary::{be_u32, u8},
    combinator::preceded,
};
use winnow::{PResult, Parser};

#[cfg(test)]
mod tests;

const QOI_HEADER_LENGTH: usize = 14;

pub struct QoiDecoder<R> {
    reader: R,
    header: Header,
}

impl<R: Read> QoiDecoder<R> {
    pub fn new(mut reader: R) -> ImageResult<Self> {
        let mut header_buf = [0u8; QOI_HEADER_LENGTH];
        reader
            .read_exact(&mut header_buf)
            .map_err(ImageError::IoError)?;
        Ok(QoiDecoder {
            header: parse_image_header(&header_buf).map_err(decoding_error)?,
            reader,
        })
    }
}

pub fn parse_image_header(header_bytes: &[u8]) -> Result<Header, String> {
    fn header_parser(input: &mut &[u8]) -> PResult<Header> {
        preceded(
            b"qoif",
            (
                be_u32,
                be_u32,
                u8.verify_map(|channels| match channels {
                    3 => Some(Channels::Rgb),
                    4 => Some(Channels::Rgba),
                    _ => None,
                }),
                u8.verify_map(|colorspace| match colorspace {
                    0 => Some(Colorspace::Srgb),
                    1 => Some(Colorspace::Linear),
                    _ => None,
                }),
            ),
        )
        .map(|(width, height, channels, colorspace)| Header {
            width,
            height,
            channels,
            colorspace,
        })
        .parse_next(input)
    }

    header_parser
        .parse(header_bytes)
        .map_err(|err| err.to_string())
}

fn parse_image_content(_content_bytes: &mut [u8], _channels: Channels) -> Result<Vec<u8>, String> {
    todo!()
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
        let raw_content =
            parse_image_content(&mut input_buf, self.header.channels).map_err(decoding_error)?;
        Ok(Cursor::new(raw_content))
    }
}

fn decoding_error(err: String) -> ImageError {
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

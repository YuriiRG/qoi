use std::fmt::Display;

use thiserror::Error;

use crate::{Channels, Colorspace, Header, Pixel};

const QOIF_MAGIC: &[u8] = b"qoif";

pub fn parse_image_header(header_bytes: &[u8]) -> Result<Header, DecoderError> {
    let mut bytes_left = header_bytes;

    tag(QOIF_MAGIC, &mut bytes_left).map_err(|_| DecoderError::InvalidMagic)?;

    let width = be_u32(&mut bytes_left).map_err(|_| DecoderError::TooShortHeader)?;

    let height = be_u32(&mut bytes_left).map_err(|_| DecoderError::TooShortHeader)?;

    let channels = match u8(&mut bytes_left).map_err(|_| DecoderError::TooShortHeader)? {
        3 => Channels::Rgb,
        4 => Channels::Rgba,
        _ => return Err(DecoderError::InvalidChannels),
    };

    let colorspace = match u8(&mut bytes_left).map_err(|_| DecoderError::TooShortHeader)? {
        0 => Colorspace::Srgb,
        1 => Colorspace::Linear,
        _ => return Err(DecoderError::InvalidColorspace),
    };

    Ok(Header {
        width,
        height,
        channels,
        colorspace,
    })
}

pub fn parse_image_content(content_bytes: &[u8], header: Header) -> Result<Vec<u8>, DecoderError> {
    let mut pixels = Vec::with_capacity(match header.channels {
        Channels::Rgba => header.height * header.width * 4,
        Channels::Rgb => header.height * header.width * 3,
    } as usize);

    let mut bytes_left = content_bytes;

    let mut state = ParserState {
        prev: Pixel {
            red: 0,
            green: 0,
            blue: 0,
            alpha: 255,
        },
        seen: [Default::default(); 64],
    };

    while !bytes_left.is_empty() {
        alt(&mut bytes_left, &mut pixels, &mut state, [qoi_op_rgb]).map_err(|err| match err {
            ParserError::Recoverable => DecoderError::InvalidPixel,
            ParserError::Invalid => DecoderError::TooFewPixels,
        })?;
    }

    Ok(vec![])
}

fn qoi_op_rgb(
    input: &mut &[u8],
    pixels: &mut Vec<Pixel>,
    state: &mut ParserState,
) -> Result<(), ParserError> {
    let red = u8(input)?;
    let green = u8(input)?;
    let blue = u8(input)?;
    let pixel = Pixel {
        red,
        green,
        blue,
        alpha: state.prev.alpha,
    };
    pixels.push(pixel);
    update_state(pixel, state);
    Ok(())
}

fn qoi_op_rgba(
    input: &mut &[u8],
    pixels: &mut Vec<Pixel>,
    state: &mut ParserState,
) -> Result<(), ParserError> {
    let red = u8(input)?;
    let green = u8(input)?;
    let blue = u8(input)?;
    let alpha = u8(input)?;
    let pixel = Pixel {
        red,
        green,
        blue,
        alpha,
    };
    pixels.push(pixel);
    update_state(pixel, state);
    Ok(())
}

#[derive(Clone, Copy, Debug)]
struct ParserState {
    prev: Pixel,
    seen: [Pixel; 64],
}

fn hash_pixel(pixel: Pixel) -> usize {
    ((pixel.red * 3 + pixel.green * 5 + pixel.blue * 7 + pixel.alpha * 11) % 64).into()
}

fn update_state(pixel: Pixel, state: &mut ParserState) {
    state.prev = pixel;
    state.seen[hash_pixel(pixel)] = pixel;
}

type Parser<O, S> = fn(&mut &[u8], &mut O, &mut S) -> Result<(), ParserError>;

fn alt<const N: usize, O, S>(
    input: &mut &[u8],
    output: &mut O,
    state: &mut S,
    parsers: [Parser<O, S>; N],
) -> Result<(), ParserError> {
    for parser in parsers {
        match parser(input, output, state) {
            Ok(()) => return Ok(()),
            Err(ParserError::Invalid) => return Err(ParserError::Invalid),
            Err(ParserError::Recoverable) => (),
        }
    }
    Err(ParserError::Recoverable)
}

fn tag(tag: &[u8], input: &mut &[u8]) -> Result<(), ParserError> {
    if input.len() < tag.len() {
        return Err(ParserError::Invalid);
    }
    let (start, rest) = input.split_at(tag.len());
    if start == tag {
        *input = rest;
        Ok(())
    } else {
        Err(ParserError::Recoverable)
    }
}

fn be_u32(input: &mut &[u8]) -> Result<u32, ParserError> {
    if input.len() < 4 {
        return Err(ParserError::Invalid);
    }
    let (bytes, rest) = input.split_at(4);
    *input = rest;
    Ok(u32::from_be_bytes(bytes.try_into().unwrap()))
}

fn u8(input: &mut &[u8]) -> Result<u8, ParserError> {
    if input.is_empty() {
        return Err(ParserError::Invalid);
    }
    let num = input[0];
    *input = &input[1..];
    Ok(num)
}

#[derive(Error, Debug, Clone, Copy)]
pub enum DecoderError {
    InvalidMagic,
    InvalidChannels,
    InvalidColorspace,
    TooShortHeader,
    InvalidPixel,
    TooFewPixels,
}

impl Display for DecoderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Error, Debug, Clone, Copy)]
enum ParserError {
    Recoverable,
    Invalid,
}

impl Display for ParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
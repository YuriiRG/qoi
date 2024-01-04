use std::fmt::Display;

use thiserror::Error;

use crate::{Channels, Colorspace, Header, Pixel};

const QOIF_MAGIC: &[u8] = b"qoif";

pub fn parse_image_header(header_bytes: &[u8]) -> Result<Header, DecoderError> {
    let bytes_left = tag(QOIF_MAGIC, header_bytes).map_err(|_| DecoderError::InvalidMagic)?;

    let (width, bytes_left) = be_u32(bytes_left).map_err(|_| DecoderError::TooShortHeader)?;

    let (height, bytes_left) = be_u32(bytes_left).map_err(|_| DecoderError::TooShortHeader)?;

    let (channels, bytes_left) = u8(bytes_left).map_err(|_| DecoderError::TooShortHeader)?;
    let channels = match channels {
        3 => Channels::Rgb,
        4 => Channels::Rgba,
        _ => {
            return Err(DecoderError::InvalidChannels);
        }
    };

    let (colorspace, _) = u8(bytes_left).map_err(|_| DecoderError::TooShortHeader)?;

    let colorspace = match colorspace {
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
    let result_len = match header.channels {
        Channels::Rgba => header.height * header.width * 4,
        Channels::Rgb => header.height * header.width * 3,
    } as usize;

    let mut pixels = Vec::with_capacity(result_len);

    let mut bytes_left = content_bytes;

    let mut state = ParserState {
        prev: Pixel {
            red: 0,
            green: 0,
            blue: 0,
            alpha: 255,
        },
        seen: [Default::default(); 64],
        is_alpha: match header.channels {
            Channels::Rgba => true,
            Channels::Rgb => false,
        },
    };

    while !bytes_left.is_empty() {
        'pixel_block: {
            for parser in [
                qoi_op_rgb,
                qoi_op_rgba,
                qoi_op_end,
                qoi_op_index,
                qoi_op_diff,
                qoi_op_luma,
                qoi_op_run,
            ] {
                match parser(bytes_left, &mut pixels, &mut state) {
                    Err(ParserError::Recoverable) => {}
                    Err(ParserError::Invalid) => return Err(DecoderError::TooFewPixels),
                    Ok(new_input) => {
                        bytes_left = new_input;
                        break 'pixel_block;
                    }
                }
            }
            return Err(DecoderError::InvalidPixel);
        }
    }

    if pixels.len() < result_len {
        return Err(DecoderError::TooFewPixels);
    }

    if pixels.len() > result_len {
        return Err(DecoderError::TooManyPixels);
    }

    Ok(pixels)
}

fn qoi_op_rgb<'a>(
    input: &'a [u8],
    pixels: &mut Vec<u8>,
    state: &mut ParserState,
) -> Result<&'a [u8], ParserError> {
    let input = tag(&[0b11111110], input)?;
    let (red, input) = u8(input)?;
    let (green, input) = u8(input)?;
    let (blue, input) = u8(input)?;
    let pixel = Pixel {
        red,
        green,
        blue,
        alpha: state.prev.alpha,
    };
    push_pixel(pixels, pixel, state.is_alpha);
    update_state(pixel, state);
    Ok(input)
}

fn qoi_op_rgba<'a>(
    input: &'a [u8],
    pixels: &mut Vec<u8>,
    state: &mut ParserState,
) -> Result<&'a [u8], ParserError> {
    let input = tag(&[0b11111111], input)?;
    let (red, input) = u8(input)?;
    let (green, input) = u8(input)?;
    let (blue, input) = u8(input)?;
    let (alpha, input) = u8(input)?;
    let pixel = Pixel {
        red,
        green,
        blue,
        alpha,
    };
    push_pixel(pixels, pixel, state.is_alpha);
    update_state(pixel, state);
    Ok(input)
}

fn qoi_op_index<'a>(
    input: &'a [u8],
    pixels: &mut Vec<u8>,
    state: &mut ParserState,
) -> Result<&'a [u8], ParserError> {
    let (byte, input) = u8(input)?;
    if (byte & 0b11000000) >> 6 != 0b00 {
        return Err(ParserError::Recoverable);
    }

    let pixel = state.seen[byte as usize];
    push_pixel(pixels, pixel, state.is_alpha);
    update_state(pixel, state);

    Ok(input)
}

fn qoi_op_diff<'a>(
    input: &'a [u8],
    pixels: &mut Vec<u8>,
    state: &mut ParserState,
) -> Result<&'a [u8], ParserError> {
    let (byte, input) = u8(input)?;
    if (byte & 0b11000000) >> 6 != 0b01 {
        return Err(ParserError::Recoverable);
    }

    let dr = ((byte & 0b00110000) >> 4).wrapping_sub(2);
    let dg = ((byte & 0b00001100) >> 2).wrapping_sub(2);
    let db = (byte & 0b00000011).wrapping_sub(2);
    let pixel = Pixel {
        red: state.prev.red.wrapping_add(dr),
        green: state.prev.green.wrapping_add(dg),
        blue: state.prev.blue.wrapping_add(db),
        alpha: state.prev.alpha,
    };
    push_pixel(pixels, pixel, state.is_alpha);
    update_state(pixel, state);

    Ok(input)
}

fn qoi_op_luma<'a>(
    input: &'a [u8],
    pixels: &mut Vec<u8>,
    state: &mut ParserState,
) -> Result<&'a [u8], ParserError> {
    let (byte1, input) = u8(input)?;
    if (byte1 & 0b11000000) >> 6 != 0b10 {
        return Err(ParserError::Recoverable);
    }

    let dg = (byte1 & 0b00111111).wrapping_sub(32);

    let (byte2, input) = u8(input)?;
    let dr = dg.wrapping_add(((byte2 & 0b11110000) >> 4).wrapping_sub(8));
    let db = dg.wrapping_add((byte2 & 0b00001111).wrapping_sub(8));

    let pixel = Pixel {
        red: state.prev.red.wrapping_add(dr),
        green: state.prev.green.wrapping_add(dg),
        blue: state.prev.blue.wrapping_add(db),
        alpha: state.prev.alpha,
    };

    push_pixel(pixels, pixel, state.is_alpha);
    update_state(pixel, state);

    Ok(input)
}

fn qoi_op_run<'a>(
    input: &'a [u8],
    pixels: &mut Vec<u8>,
    state: &mut ParserState,
) -> Result<&'a [u8], ParserError> {
    let (byte, input) = u8(input)?;
    if (byte & 0b11000000) >> 6 != 0b11 {
        return Err(ParserError::Recoverable);
    }

    let run = (byte & 0b00111111).wrapping_add(1);

    push_pixels(pixels, state.prev, run as usize, state.is_alpha);

    update_state(state.prev, state);

    Ok(input)
}

fn qoi_op_end<'a>(
    input: &'a [u8],
    #[allow(clippy::ptr_arg)] _pixels: &mut Vec<u8>,
    _state: &mut ParserState,
) -> Result<&'a [u8], ParserError> {
    let input = tag(&[0u8, 0, 0, 0, 0, 0, 0, 1], input)?;

    Ok(input)
}

#[derive(Clone, Copy, Debug)]
struct ParserState {
    prev: Pixel,
    seen: [Pixel; 64],
    is_alpha: bool,
}

fn hash_pixel(pixel: Pixel) -> usize {
    (pixel.red as usize * 3
        + pixel.green as usize * 5
        + pixel.blue as usize * 7
        + pixel.alpha as usize * 11)
        % 64
}

fn update_state(pixel: Pixel, state: &mut ParserState) {
    state.prev = pixel;
    state.seen[hash_pixel(pixel)] = pixel;
}

fn push_pixel(pixels: &mut Vec<u8>, pixel: Pixel, is_alpha: bool) {
    pixels.push(pixel.red);
    pixels.push(pixel.green);
    pixels.push(pixel.blue);
    if is_alpha {
        pixels.push(pixel.alpha);
    }
}

fn push_pixels(pixels: &mut Vec<u8>, pixel: Pixel, run: usize, is_alpha: bool) {
    if is_alpha {
        for _ in 0..run {
            pixels.push(pixel.red);
            pixels.push(pixel.green);
            pixels.push(pixel.blue);
            pixels.push(pixel.alpha);
        }
    } else {
        for _ in 0..run {
            pixels.push(pixel.red);
            pixels.push(pixel.green);
            pixels.push(pixel.blue);
        }
    }
}

fn tag<'a>(tag: &[u8], input: &'a [u8]) -> Result<&'a [u8], ParserError> {
    if input.len() < tag.len() {
        return Err(ParserError::Invalid);
    }
    let (start, rest) = input.split_at(tag.len());
    if start == tag {
        Ok(rest)
    } else {
        Err(ParserError::Recoverable)
    }
}

fn be_u32(input: &[u8]) -> Result<(u32, &[u8]), ParserError> {
    if input.len() < 4 {
        return Err(ParserError::Invalid);
    }
    let (bytes, rest) = input.split_at(4);
    Ok((u32::from_be_bytes(bytes.try_into().unwrap()), rest))
}

fn u8(input: &[u8]) -> Result<(u8, &[u8]), ParserError> {
    if input.is_empty() {
        return Err(ParserError::Invalid);
    }
    let num = input[0];
    Ok((num, &input[1..]))
}

#[derive(Error, Debug, Clone, Copy)]
pub enum DecoderError {
    InvalidMagic,
    InvalidChannels,
    InvalidColorspace,
    TooShortHeader,
    InvalidPixel,
    TooFewPixels,
    TooManyPixels,
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

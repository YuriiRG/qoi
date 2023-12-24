use std::io::{Cursor, Read};

use image::{
    error::{DecodingError, ImageFormatHint},
    ColorType, ImageDecoder, ImageError, ImageFormat, ImageResult,
};

use winnow::{
    binary::{be_u32, u8},
    combinator::{alt, preceded, repeat},
    token::tag,
    Stateful,
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

fn parse_image_content(content_bytes: &[u8], header: Header) -> Result<Vec<u8>, String> {
    type Stream<'is> = Stateful<&'is [u8], ParserState>;
    fn hash_pixel(pixel: Pixel) -> usize {
        ((pixel.red * 3 + pixel.green * 5 + pixel.blue * 7 + pixel.alpha * 11) % 64).into()
    }

    fn qoi_op_rgb(i: &mut Stream) -> PResult<Pixel> {
        let prev_alpha = i.state.prev.alpha;
        let pixel = preceded(
            tag(&[0b11111110]),
            (u8, u8, u8).map(|(red, green, blue)| Pixel {
                red,
                green,
                blue,
                alpha: prev_alpha,
            }),
        )
        .parse_next(i)?;
        i.state.prev = pixel;
        i.state.seen[hash_pixel(pixel)] = pixel;
        Ok(pixel)
    }

    fn qoi_op_rgba(i: &mut Stream) -> PResult<Pixel> {
        let pixel = preceded(
            tag(&[0b11111111]),
            (u8, u8, u8, u8).map(|(red, green, blue, alpha)| Pixel {
                red,
                green,
                blue,
                alpha,
            }),
        )
        .parse_next(i)?;
        i.state.prev = pixel;
        i.state.seen[hash_pixel(pixel)] = pixel;
        Ok(pixel)
    }

    fn qoi_op_index(i: &mut Stream) -> PResult<Pixel> {
        let seen_pixels = i.state.seen;
        let pixel = u8
            .verify_map(|byte| {
                if (byte & 0b11000000) >> 6 == 0b00 {
                    Some(seen_pixels[usize::from(byte)])
                } else {
                    None
                }
            })
            .parse_next(i)?;
        i.state.prev = pixel;
        i.state.seen[hash_pixel(pixel)] = pixel;
        Ok(pixel)
    }

    fn qoi_op_diff(i: &mut Stream) -> PResult<Pixel> {
        let prev = i.state.prev;

        let (dr, dg, db) = u8
            .verify_map(|byte| {
                if (byte & 0b11000000) >> 6 == 0b01 {
                    Some((
                        ((byte & 0b00110000) >> 4) as i16 - 2,
                        ((byte & 0b00001100) >> 2) as i16 - 2,
                        (byte & 0b00000011) as i16 - 2,
                    ))
                } else {
                    None
                }
            })
            .parse_next(i)?;
        let pixel = Pixel {
            red: (prev.red as i16 + dr) as u8,
            green: (prev.green as i16 + dg) as u8,
            blue: (prev.blue as i16 + db) as u8,
            alpha: prev.alpha,
        };
        Ok(pixel)
    }

    fn pixels_parser(i: &mut Stream) -> PResult<Vec<Pixel>> {
        repeat(
            0..,
            alt((qoi_op_rgb, qoi_op_rgba, qoi_op_index, qoi_op_diff)),
        )
        .parse_next(i)
    }

    let default_state = ParserState {
        prev: Pixel {
            red: 0,
            green: 0,
            blue: 0,
            alpha: 255,
        },
        seen: [Default::default(); 64],
    };

    let pixels = pixels_parser
        .parse(Stream {
            input: content_bytes,
            state: default_state,
        })
        .map_err(|err| err.to_string())?;

    let mut result = Vec::with_capacity(match header.channels {
        Channels::Rgba => header.height * header.width * 4,
        Channels::Rgb => header.height * header.width * 3,
    } as usize);

    for pixel in pixels {
        result.push(pixel.red);
        result.push(pixel.green);
        result.push(pixel.blue);
        if let Channels::Rgb = header.channels {
            result.push(pixel.alpha);
        }
    }

    Ok(result)
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

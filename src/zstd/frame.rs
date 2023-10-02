use super::parsing;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Frame parsing error: {0}")]
    ParsingError(#[from] parsing::Error),

    #[error("Unrecognized magic number: {0}")]
    UnrecognizedMagic(u32),
}
use Error::*;
type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug)]
pub enum Frame<'a> {
    ZstandardFrame,
    SkippableFrame(SkippableFrame<'a>),
}

const STANDARD_MAGIC_NUMBER: u32 = 0xFD2FB528;
const SKIPPABLE_MAGIC_NUMBER: u32 = 0x184D2A5; // last 4bits: 0x0 to 0xF

#[derive(Debug)]
pub struct SkippableFrame<'a> {
    pub magic: u32,
    data: &'a [u8],
}

// pub struct ZstandardFrame {
//     header: u32,
// }

pub struct FrameIterator<'a> {
    parser: parsing::ForwardByteParser<'a>,
}

impl<'a> Frame<'a> {
    pub fn parse(input: &mut parsing::ForwardByteParser<'a>) -> Result<Self> {
        let magic = input.le_u32()?;
        match magic {
            STANDARD_MAGIC_NUMBER => todo!(),
            _ => {
                if magic >> 4 == SKIPPABLE_MAGIC_NUMBER {
                    let len = input.le_u32()? as usize;
                    let data = input.slice(len)?;
                    return Ok(Self::SkippableFrame(SkippableFrame { magic, data }));
                }
                Err(UnrecognizedMagic(magic))
            }
        }
    }

    pub fn decode(self) -> Vec<u8> {
        match self {
            Frame::SkippableFrame(f) => Vec::from(f.data),
            Frame::ZstandardFrame => todo!(),
        }
    }
}

impl<'a> FrameIterator<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            parser: parsing::ForwardByteParser::new(data),
        }
    }
}

impl<'a> Iterator for FrameIterator<'a> {
    type Item = Result<Frame<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.parser.is_empty() {
            return None;
        }
        Some(Frame::parse(&mut self.parser))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_skippable_frame() {
        let mut parser = parsing::ForwardByteParser::new(&[
            // Skippable frame with magic 0x184d2a53, length 3, content 0x10 0x20 0x30
            // and an extra byte at the end.
            0x53, 0x2a, 0x4d, 0x18, 0x03, 0x00, 0x00, 0x00, 0x10, 0x20, 0x30, 0x40,
        ]);
        let Frame::SkippableFrame(skippable) = Frame::parse(&mut parser).unwrap() else {
            panic!("unexpected frame type")
        };
        assert_eq!(0x184d2a53, skippable.magic);
        assert_eq!(&[0x10, 0x20, 0x30], skippable.data);
        assert_eq!(1, parser.len());
    }

    #[test]
    fn test_parse_truncated_skippable_frame() {
        let mut parser = parsing::ForwardByteParser::new(&[
            0x50, 0x2a, 0x4d, 0x18, 0x03, 0x00, 0x00, 0x00, 0x10, 0x20,
        ]);
        assert!(matches!(
            Frame::parse(&mut parser),
            Err(ParsingError(parsing::Error::NotEnoughBytes {
                requested: 3,
                available: 2
            }))
        ));
    }

    #[test]
    fn test_error_on_unknown_frame() {
        let mut parser = parsing::ForwardByteParser::new(&[0x10, 0x20, 0x30, 0x40]);
        assert!(matches!(
            Frame::parse(&mut parser),
            Err(UnrecognizedMagic(0x40302010))
        ));
    }

    // #[test]
    // fn test_decode_frame() {
    //     todo!()
    // }

    // #[test]
    // fn test_iterate_frame_iterator() {
    //     todo!()
    // }
}

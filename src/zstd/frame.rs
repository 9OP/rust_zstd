use super::parsing::{ForwardByteParser, ParsingError};

#[derive(Debug, thiserror::Error)]
pub enum FrameError {
    #[error("Frame parsing error: {0}")]
    ParsingError(#[from] ParsingError),

    #[error("Unrecognized magic number: {0}")]
    UnrecognizedMagic(u32),
}

type Result<T, E = FrameError> = std::result::Result<T, E>;

pub enum Frame<'a> {
    ZstandardFrame,
    SkippableFrame(SkippableFrame<'a>),
}

const STANDARD_MAGIC_NUMBER: u32 = 0xFD2FB528;
const SKIPPABLE_MAGIC_NUMBER: u32 = 0x184D2A5; // last 4bits: 0x0 to 0xF

pub struct SkippableFrame<'a> {
    magic: u32,
    data: &'a [u8],
}

impl<'a> Frame<'a> {
    pub fn parse(input: &mut ForwardByteParser<'a>) -> Result<Self> {
        let magic = input.le_u32()?;
        match magic {
            STANDARD_MAGIC_NUMBER => todo!(),
            _ => {
                if magic >> 4 == SKIPPABLE_MAGIC_NUMBER {
                    let len = input.le_u32()? as usize;
                    let data = input.slice(len)?;
                    return Ok(Self::SkippableFrame(SkippableFrame { magic, data }));
                }
                Err(FrameError::UnrecognizedMagic(magic))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_skippable_frame() {
        let mut parser = ForwardByteParser::new(&[
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
    fn test_error_on_unknown_frame() {
        let mut parser = ForwardByteParser::new(&[0x10, 0x20, 0x30, 0x40]);
        assert!(matches!(
            Frame::parse(&mut parser),
            Err(FrameError::UnrecognizedMagic(0x40302010))
        ));
    }
}

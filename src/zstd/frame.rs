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
                    let data = input.slice(input.len())?;
                    return Ok(Self::SkippableFrame(SkippableFrame { magic, data }));
                }
                Err(FrameError::UnrecognizedMagic(magic))
            }
        }
    }
}

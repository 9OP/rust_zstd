use crate::{decoders::HuffmanDecoder, parsing};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Generic error")]
    GenericError,
    // #[error("Frame parsing error: {0}")]
    // ParsingError(#[from] parsing::Error),

    // #[error(transparent)]
    // BlockError(#[from] block::Error),

    // #[error(transparent)]
    // DecoderError(#[from] decoders::Error),

    // #[error("Unrecognized magic number: {0}")]
    // UnrecognizedMagic(u32),

    // #[error("Corrupted frame, checksum mismatch: {got:#08x} != {expected:#08x}")]
    // CorruptedFrame { got: u32, expected: u32 },
}
use Error::*;
pub type Result<T, E = Error> = std::result::Result<T, E>;

pub enum LiteralsSection<'a> {
    RawLiteralsBlock(RawLiteralsBlock<'a>),
    RLELiteralsBlock(RLELiteralsBlock),
    CompressedLiteralsBlock(CompressedLiteralsBlock<'a>),
}

pub struct RawLiteralsBlock<'a>(&'a [u8]);

pub struct RLELiteralsBlock {
    byte: u8,
    repetition: usize,
}

pub struct CompressedLiteralsBlock<'a> {
    huffman: Option<HuffmanDecoder>,
    regenerated_size: usize,
    jump_table: [u8; 3],
    data: &'a [u8],
}

impl<'a> LiteralsSection<'a> {
    pub fn parse(input: &mut parsing::ForwardByteParser<'a>) -> Result<Self> {
        todo!();
    }
}

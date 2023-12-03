mod alternating;
mod bit_decoder;
mod decoding_context;
mod fse;
mod huffman;
mod rle;
mod sequence;

pub use crate::parsing::{BackwardBitParser, ForwardBitParser, ForwardByteParser, ParsingError};
pub use crate::sequences::SequenceCommand;
pub use alternating::*;
pub use bit_decoder::*;
pub use decoding_context::*;
pub use fse::*;
pub use huffman::*;
pub use rle::*;
pub use sequence::*;

#[derive(Debug, thiserror::Error)]
pub enum DecoderError {
    #[error("decoder parsing: {0}")]
    Parsing(#[from] ParsingError),

    #[error("decoder context: {0}")]
    Context(#[from] ContextError),

    #[error("decoder fse: {0}")]
    Fse(#[from] FseError),

    #[error("decoder huffman: {0}")]
    Huffman(#[from] HuffmanError),
}

type Error = DecoderError;
type Result<T, E = DecoderError> = std::result::Result<T, E>;

mod alternating;
mod bit_decoder;
mod decoding_context;
mod fse;
mod huffman;
mod rle;
mod sequence;

pub use crate::parsing::ParsingError;
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
    ParsingError(#[from] ParsingError),

    #[error("decoder context: {0}")]
    ContextError(#[from] ContextError),

    #[error("decoder fse: {0}")]
    FseError(#[from] FseError),

    #[error("decoder huffman: {0}")]
    HuffmanError(#[from] HuffmanError),
}

type Error = DecoderError;
type Result<T, E = DecoderError> = std::result::Result<T, E>;

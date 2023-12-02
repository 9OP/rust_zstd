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
    #[error(transparent)]
    ParsingError(#[from] ParsingError),

    #[error(transparent)]
    ContextError(#[from] ContextError),

    #[error(transparent)]
    FseError(#[from] FseError),

    #[error(transparent)]
    HuffmanError(#[from] HuffmanError),
}

type Error = DecoderError;
type Result<T, E = DecoderError> = std::result::Result<T, E>;

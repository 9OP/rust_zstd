mod backward_bit_parser;
mod forward_bit_parser;
mod forward_byte_parser;

pub use backward_bit_parser::BackwardBitParser;
pub use forward_bit_parser::ForwardBitParser;
pub use forward_byte_parser::ForwardByteParser;

#[derive(Debug, thiserror::Error)]
pub enum ParsingError {
    #[error("Not enough bytes: {requested} requested out of {available} available")]
    NotEnoughBytes { requested: usize, available: usize },

    #[error("Not enough bits: {requested} requested out of {available} available")]
    NotEnoughBits { requested: usize, available: usize },

    #[error("Bitstream header does not contain any '1'")]
    MalformedBitstream,
}

type Error = ParsingError;
type Result<T, E = ParsingError> = std::result::Result<T, E>;

mod backward_bit_parser;
// mod bit_parser;
mod error;
mod forward_bit_parser;
mod forward_byte_parser;

pub use backward_bit_parser::BackwardBitParser;
pub use error::{Error, Result};
pub use forward_bit_parser::ForwardBitParser;
pub use forward_byte_parser::ForwardByteParser;

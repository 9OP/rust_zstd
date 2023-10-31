use crate::parsing;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Decoder parsing error: {0}")]
    ParsingError(#[from] parsing::Error),

    #[error("Cannot compute missing huffman weight")]
    ComputeMissingWeight,

    #[error("Missing symbol for huffman code")]
    MissingSymbol,

    #[error("Error computing FSE coefficient")]
    ComputeFseCoefficient,
}
pub type Result<T, E = Error> = std::result::Result<T, E>;

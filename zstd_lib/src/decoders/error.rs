use crate::parsing;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Decoder parsing error: {0}")]
    ParsingError(#[from] parsing::Error),

    #[error("Cannot compute missing huffman weight")]
    ComputeMissingWeight,

    #[error("Huffman weight: {weight} bigger than max_weight: {max}")]
    WeightTooBig { weight: u32, max: u32 },

    #[error("Missing symbol for huffman code")]
    MissingSymbol,

    #[error("FSE AccLog: {log} greater than allowed maximum: {max}")]
    AccLogTooBig { log: u8, max: u8 },

    #[error("FSE distribution is corrupted")]
    DistributionCorrupted,

    #[error("Window size too large")]
    WindowSizeError,

    #[error("Offset size error")]
    OffsetError,
}
pub type Result<T, E = Error> = std::result::Result<T, E>;

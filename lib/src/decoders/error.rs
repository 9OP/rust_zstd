use crate::parsing;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Decoder parsing error: {0}")]
    ParsingError(#[from] parsing::Error),

    #[error("Cannot compute missing huffman weight")]
    ComputeMissingWeight,

    #[error("Missing symbol for huffman code")]
    MissingSymbol,

    #[error("FSE AccLog: {log} greater than allowed maximum: {max}")]
    AccLogTooBig { log: u8, max: u8 },

    #[error("FSE counter: {counter} exceeded expected sum: {expected_sum}")]
    CounterMismatch { counter: u32, expected_sum: u32 },
}
pub type Result<T, E = Error> = std::result::Result<T, E>;

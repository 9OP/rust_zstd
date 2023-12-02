#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Not enough bytes: {requested} requested out of {available} available")]
    NotEnoughBytes { requested: usize, available: usize },

    #[error("Not enough bits: {requested} requested out of {available} available")]
    NotEnoughBits { requested: usize, available: usize },

    #[error("Bitstream header does not contain any '1'")]
    MalformedBitstream,

    #[error("Overflow: {length} overflow expected range {range}")]
    Overflow { length: usize, range: usize },
}
pub type Result<T, E = Error> = std::result::Result<T, E>;

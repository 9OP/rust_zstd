#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Not enough bytes: {requested:#06x} requested out of {available:#06x} available")]
    NotEnoughBytes { requested: usize, available: usize },

    #[error("Not enough bits: {requested} requested out of {available} available")]
    NotEnoughBits { requested: usize, available: usize },

    #[error("Bitstream header does not contain any '1'")]
    MalformedBitstream,

    #[error("Cannot take {requested} bits at once")]
    LargeBitsTake { requested: usize },
}
pub type Result<T, E = Error> = std::result::Result<T, E>;

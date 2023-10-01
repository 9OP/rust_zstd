#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("not enough bytes: {requested:#06x} requested out of {available:#06x} available")]
    NotEnoughBytes { requested: usize, available: usize },

    // Encapsulate an I/O error without adding any more context
    // and add a `impl From<std::io::Error> for Error` implementation.
    #[error(transparent)]
    Io(#[from] std::io::Error),

    // Custom error originating from this module
    #[error("bad magic found {found:#06x} instead of expected {expected:#06x}")]
    BadMagic { expected: u32, found: u32 },
    // // // Encapsulate an error from another module while adding context
    // #[error("corrupted Huffman table weights")]
    // CorruptedWeights(#[source] fse::Error),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

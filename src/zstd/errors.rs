#[derive(Debug, thiserror::Error)]
pub enum Error {
    // Encapsulate an I/O error without adding any more context
    // and add a `impl From<std::io::Error> for Error` implementation.
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("not enough bytes: {requested:#06x} requested out of {available:#06x} available")]
    NotEnoughBytes { requested: usize, available: usize },

    // Custom error originating from this module
    #[error("bad magic found {found:#06x} instead of expected {expected:#06x}")]
    BadMagic { expected: u32, found: u32 },
    // // Encapsulate an error from another module while adding context
    // #[error("corrupted Huffman table weights")]
    // CorruptedWeights(#[source] fse::Error),
}

// Define a local `Result` type alias which can be used with one argument,
// the default for the second type being defined as the local `Error` type.
pub type Result<T, E = Error> = std::result::Result<T, E>;

// // Example function returning a `Result` without repeating the `Error` type.
// // The return type can be written as:
// // - `Result<Vec<u8>>` in this module, or `Result<Vec<u8>, Error>`
// // - `mymod::Result<Vec<u8>>` or `Result<Vec<u8>, mymod::Error>` from outside
// fn read_file(filename: &str) -> Result<Vec<u8>> {
//     Ok(std::file::read(filename)?)
// }

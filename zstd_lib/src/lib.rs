pub mod block;
pub mod decoders;
pub mod frame;
pub mod literals;
pub mod parsing;
pub mod sequences;

pub use block::*;
pub use decoders::*;
pub use frame::*;
pub use literals::*;
pub use parsing::*;
pub use sequences::*;

#[derive(Debug, thiserror::Error)]
pub enum ZstdLibError {
    #[error(transparent)]
    ParsingError(#[from] ParsingError),

    #[error(transparent)]
    BlockError(#[from] BlockError),

    #[error(transparent)]
    FrameError(#[from] FrameError),

    #[error(transparent)]
    DecoderError(#[from] DecoderError),

    #[error(transparent)]
    LiteralsError(#[from] LiteralsError),

    #[error(transparent)]
    SequencesError(#[from] SequencesError),
}
type Error = ZstdLibError;
type Result<T, E = ZstdLibError> = std::result::Result<T, E>;

pub fn decode(bytes: Vec<u8>, info: bool) -> Result<Vec<u8>> {
    let mut res: Vec<u8> = Vec::new();
    for frame in frame::FrameIterator::new(bytes.as_slice()) {
        let frame = frame?;
        if info {
            println!("{:#x?}", frame);
        }
        res.extend(frame.decode()?);
    }

    Ok(res)
}

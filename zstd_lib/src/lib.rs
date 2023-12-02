pub mod block;
pub mod decoders;
pub mod frame;
pub mod literals;
pub mod parsing;
pub mod sequences;

// #[derive(Debug, thiserror::Error)]
// pub enum Error {
//     #[error(transparent)]
//     ParsingError(#[from] ParsingError),

//     #[error(transparent)]
//     ContextError(#[from] ContextError),

//     #[error(transparent)]
//     FseError(#[from] FseError),

//     #[error(transparent)]
//     HuffmanError(#[from] HuffmanError),
// }

pub use Error as ZstdLibError;

pub use block::*;
pub use decoders::*;
pub use parsing::*;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    ParsingError(#[from] ParsingError),

    #[error(transparent)]
    DecoderError(#[from] DecoderError),

    #[error(transparent)]
    BlockError(#[from] BlockError),
}

type Result<T, E = Error> = std::result::Result<T, E>;

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

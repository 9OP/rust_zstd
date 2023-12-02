mod block;
mod decoders;
mod frame;
mod literals;
pub mod parsing;
mod sequences;

use block::*;
use decoders::*;
use frame::*;
use literals::*;
use parsing::*;
use sequences::*;

/*
    ZstdLib only export 2 things:
    - pub fn decode
    - ZstdLibError

    I think this is a clean design because as a user of the library I dont
    want to know the inner implementation details. I only want a handle to decode
    and a CustomError type.

    In case you whish to export more just prefix `pub` in front of
    the `use` statements above. For instance to expose literals mod
    to the outside:
        use literals::*;
*/

#[derive(Debug, thiserror::Error)]
pub enum ZstdLibError {
    #[error(transparent)]
    Parsing(#[from] ParsingError),

    #[error(transparent)]
    Block(#[from] BlockError),

    #[error(transparent)]
    Frame(#[from] FrameError),

    #[error(transparent)]
    Decoder(#[from] DecoderError),

    #[error(transparent)]
    Literals(#[from] LiteralsError),

    #[error(transparent)]
    Sequences(#[from] SequencesError),
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

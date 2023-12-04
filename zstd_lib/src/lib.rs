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

use std::thread;

/*
    ZstdLib only export 2+1 things:
        - pub fn decode
        - ZstdLibError
        (- parsing module)

    I think this is a clean design because as a user of the library I dont
    want to know the inner implementation details. I only want a handle to decode
    and a CustomError type.

    (Parsing module is exported for the sake of doc tests. It is not 100% relevant
    and we could remove them anyway and make the module private.)
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

    #[error("Parallel decoding panicked")]
    ParallelDecodingError,
}
type Error = ZstdLibError;
type Result<T, E = ZstdLibError> = std::result::Result<T, E>;

fn parse_frames(bytes: &[u8], info: bool) -> Result<Vec<Frame>> {
    let mut frames = Vec::new();
    for frame in frame::FrameIterator::new(bytes) {
        let frame = frame?;
        if info {
            println!("{:#x?}", frame);
        }
        frames.push(frame);
    }
    Ok(frames)
}

pub fn decode(bytes: Vec<u8>, info: bool) -> Result<Vec<u8>> {
    let mut decoded: Vec<u8> = Vec::new();
    let frames = parse_frames(bytes.as_slice(), info)?;

    thread::scope(|s| -> Result<(), ZstdLibError> {
        let handles: Vec<_> = frames
            .into_iter()
            .map(|frame| s.spawn(move || frame.decode()))
            .collect();

        for handle in handles {
            let result = handle.join().map_err(|_| Error::ParallelDecodingError)??;
            decoded.extend(result);
        }

        Ok(())
    })?;

    Ok(decoded)
}

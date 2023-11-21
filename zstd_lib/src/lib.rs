pub mod block;
pub mod decoders;
pub mod frame;
pub mod parsing;

pub fn decrypt(bytes: Vec<u8>, info: bool) -> frame::Result<Vec<u8>> {
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

use super::HuffmanDecoder;
use super::{Error::*, Result};

pub struct DecodingContext {
    pub huffman: Option<HuffmanDecoder>,
    pub decoded: Vec<u8>,
    pub window_size: u64,
}

const MAX_WINDOW_SIZE: u64 = 67108864; // 64Mib

impl DecodingContext {
    pub fn new(window_size: u64) -> Result<Self> {
        if window_size > MAX_WINDOW_SIZE {
            return Err(WindowSizeError);
        }
        Ok(Self {
            huffman: None,
            decoded: Vec::<u8>::new(),
            window_size,
        })
    }
}

use super::HuffmanDecoder;
use super::{Error::*, Result};

pub struct DecodingContext {
    pub huffman: Option<HuffmanDecoder>,
    pub decoded: Vec<u8>,
    pub window_size: u64,

    pub offset_1: usize,
    pub offset_2: usize,
    pub offset_3: usize,
}

const MAX_WINDOW_SIZE: u64 = 1024 * 1024 * 64; // 64Mib

impl DecodingContext {
    /// Create a new decoding context instance. Return `WindowSizeError` when `window_size` exceeds 64Mb
    pub fn new(window_size: u64) -> Result<Self> {
        if window_size > MAX_WINDOW_SIZE {
            return Err(WindowSizeError);
        }
        Ok(Self {
            huffman: None,
            decoded: Vec::<u8>::new(),
            window_size,
            offset_1: 1,
            offset_2: 4,
            offset_3: 8,
        })
    }

    /// Decode an offset and properly maintain the three repeat offsets
    pub fn decode_offset(&mut self, offset: usize, literals_length: usize) -> Result<usize> {
        match offset {
            1 => {
                if literals_length == 0 {
                    let offset_1 = self.offset_1;
                    self.offset_1 = self.offset_2;
                    self.offset_2 = offset_1;
                }
            }
            2 => {
                if literals_length == 0 {
                    let offset_1 = self.offset_1;
                    let offset_2 = self.offset_2;
                    self.offset_1 = self.offset_3;
                    self.offset_2 = offset_1;
                    self.offset_3 = offset_2;
                } else {
                    let offset_1 = self.offset_1;
                    self.offset_1 = self.offset_2;
                    self.offset_2 = offset_1;
                }
            }
            3 => {
                if literals_length == 0 {
                    self.offset_3 = self.offset_2;
                    self.offset_2 = self.offset_1;
                    self.offset_1 -= 1;
                } else {
                    let offset_1 = self.offset_1;
                    let offset_2 = self.offset_2;
                    self.offset_1 = self.offset_3;
                    self.offset_2 = offset_1;
                    self.offset_3 = offset_2;
                }
            }
            _ => {
                self.offset_3 = self.offset_2;
                self.offset_2 = self.offset_1;
                self.offset_1 = offset - 3;
            }
        }

        Ok(self.offset_1)
    }

    /// Execute the sequences while updating the offsets
    pub fn execute_sequences(
        &mut self,
        sequences: Vec<(usize, usize, usize)>,
        literals: &[u8],
    ) -> Result<()> {
        todo!() // Using the `Self::decode_offset()` method
    }
}

use super::{Error, HuffmanDecoder, Result, SymbolDecoder};

#[derive(Debug, thiserror::Error)]
pub enum ContextError {
    #[error("Window size too large")]
    WindowSizeError,

    #[error("Offset size error")]
    OffsetError,

    #[error("Not enough bytes: {requested} requested out of {available} available")]
    NotEnoughBytes { requested: usize, available: usize },
}
use ContextError::*;

pub struct DecodingContext {
    pub decoded: Vec<u8>,
    window_size: usize,
    pub huffman: Option<HuffmanDecoder>,
    repeat_offsets: RepeatOffset,
    pub literals_lengths_decoder: Option<Box<SymbolDecoder>>,
    pub offsets_decoder: Option<Box<SymbolDecoder>>,
    pub match_lengths_decoder: Option<Box<SymbolDecoder>>,
}

struct RepeatOffset {
    offset_1: usize,
    offset_2: usize,
    offset_3: usize,
}

impl RepeatOffset {
    /// Decode an offset and properly maintain the three repeat offsets
    pub fn decode_offset(&mut self, offset: usize, literals_length: usize) -> usize {
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
        self.offset_1
    }
}

const MAX_WINDOW_SIZE: usize = 1024 * 1024 * 64; // 64Mib

impl DecodingContext {
    /// Create a new decoding context instance. Return `WindowSizeError` when `window_size` exceeds 64Mb
    pub fn new(window_size: usize) -> Result<Self> {
        if window_size > MAX_WINDOW_SIZE {
            return Err(Error::ContextError(WindowSizeError));
        }

        Ok(Self {
            decoded: Vec::<u8>::new(),
            window_size,
            huffman: None,
            repeat_offsets: RepeatOffset {
                offset_1: 1,
                offset_2: 4,
                offset_3: 8,
            },
            literals_lengths_decoder: None,
            offsets_decoder: None,
            match_lengths_decoder: None,
        })
    }

    /// Decode an offset and properly maintain the three repeat offsets
    pub fn decode_offset(&mut self, offset: usize, literals_length: usize) -> Result<usize> {
        let offset = self.repeat_offsets.decode_offset(offset, literals_length);

        if offset > self.window_size as usize {
            return Err(Error::ContextError(OffsetError));
        }
        if offset > self.decoded.len() {
            return Err(Error::ContextError(OffsetError));
        }

        Ok(offset)
    }

    /// Execute the sequences while updating the offsets
    pub fn execute_sequences(
        &mut self,
        sequences: Vec<(usize, usize, usize)>,
        literals: &[u8],
    ) -> Result<()> {
        let mut copy_index = 0;

        for (literals_length, offset_value, match_value) in sequences {
            let start = copy_index;
            let end = copy_index + literals_length;
            copy_index = end;

            if end > literals.len() {
                return Err(Error::ContextError(NotEnoughBytes {
                    requested: literals_length,
                    available: literals.len(),
                }));
            }

            self.decoded.extend_from_slice(&literals[start..end]);
            let offset = self.decode_offset(offset_value, literals_length)?;
            let mut index = self.decoded.len() - offset;
            for _ in 0..match_value {
                let byte = self
                    .decoded
                    .get(index)
                    .unwrap_or_else(|| panic!("unexpected sequence index: {index}"));
                self.decoded.push(*byte);
                index += 1;
            }
        }

        let (_, rest) = literals.split_at(copy_index);
        self.decoded.extend_from_slice(rest);

        Ok(())
    }
}

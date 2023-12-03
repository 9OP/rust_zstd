use super::{Error, HuffmanDecoder, Result, SequenceCommand, SymbolDecoder};

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
    // Entropy tables
    pub huffman: Option<HuffmanDecoder>,
    pub literals_lengths_decoder: Option<Box<SymbolDecoder>>,
    pub match_lengths_decoder: Option<Box<SymbolDecoder>>,
    pub offsets_decoder: Option<Box<SymbolDecoder>>,

    // Raw content for back references
    pub decoded: Vec<u8>,
    window_size: usize,

    // Offset history
    repeat_offsets: RepeatOffset,
}

struct RepeatOffset {
    offset_1: u64,
    offset_2: u64,
    offset_3: u64,
}

impl RepeatOffset {
    /// Decode an offset and properly maintain the three repeat offsets
    fn compute_offset(&mut self, offset: usize, literals_length: usize) -> usize {
        // from reference
        let mut result_offset = 0;

        if offset <= 3 {
            let mut idx = offset - 1;

            if literals_length == 0 {
                idx += 1;
            }

            if idx == 0 {
                result_offset = self.offset_1;
            } else {
                result_offset = if idx < 3 {
                    match idx {
                        0 => self.offset_1,
                        1 => self.offset_2,
                        2 => self.offset_3,
                        _ => panic!("unexpected"),
                    }
                } else {
                    self.offset_1 - 1
                };

                if idx > 1 {
                    self.offset_3 = self.offset_2;
                }

                self.offset_2 = self.offset_1;
                self.offset_1 = result_offset;
            }
        } else {
            result_offset = (offset as u64) - 3;
            self.offset_3 = self.offset_2;
            self.offset_2 = self.offset_1;
            self.offset_1 = result_offset as u64;
        }

        result_offset as usize

        // Old
        // match offset {
        //     1 => {
        //         if literals_length == 0 {
        //             std::mem::swap(&mut self.offset_1, &mut self.offset_2);
        //         }
        //     }
        //     2 => {
        //         if literals_length == 0 {
        //             let offset_1 = self.offset_1;
        //             let offset_2 = self.offset_2;
        //             self.offset_1 = self.offset_3;
        //             self.offset_2 = offset_1;
        //             self.offset_3 = offset_2;
        //         } else {
        //             std::mem::swap(&mut self.offset_1, &mut self.offset_2);
        //         }
        //     }
        //     3 => {
        //         if literals_length == 0 {
        //             self.offset_3 = self.offset_2;
        //             self.offset_2 = self.offset_1;
        //             self.offset_1 -= 1;
        //         } else {
        //             let offset_1 = self.offset_1;
        //             let offset_2 = self.offset_2;
        //             self.offset_1 = self.offset_3;
        //             self.offset_2 = offset_1;
        //             self.offset_3 = offset_2;
        //         }
        //     }
        //     _ => {
        //         self.offset_3 = self.offset_2;
        //         self.offset_2 = self.offset_1;
        //         self.offset_1 = offset - 3;
        //     }
        // }
        // self.offset_1
    }
}

const MAX_WINDOW_SIZE: usize = 1024 * 1024 * 64; // 64Mib

impl DecodingContext {
    /// Create a new decoding context instance. Return `WindowSizeError` when `window_size` exceeds 64Mb
    pub fn new(window_size: usize) -> Result<Self> {
        if window_size > MAX_WINDOW_SIZE {
            return Err(Error::Context(WindowSizeError));
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
    fn compute_offset(&mut self, offset: usize, literals_length: usize) -> Result<usize> {
        let offset = self.repeat_offsets.compute_offset(offset, literals_length);
        let total_output = self.decoded.len();

        if offset > self.window_size || offset > total_output {
            return Err(Error::Context(OffsetError));
        }

        Ok(offset)
    }

    /// Execute the sequences while updating the offsets
    pub fn execute_sequences(
        &mut self,
        sequences: Vec<SequenceCommand>,
        literals: &[u8],
    ) -> Result<()> {
        let mut copy_index = 0;

        for seq in sequences {
            let start = copy_index;
            let end = copy_index + seq.literal_length;
            copy_index = end;

            if end > literals.len() {
                return Err(Error::Context(NotEnoughBytes {
                    requested: seq.literal_length,
                    available: literals.len(),
                }));
            }

            // Copy from literals
            self.decoded.extend_from_slice(&literals[start..end]);

            // Offset + match copy
            let offset = self.compute_offset(seq.offset, seq.literal_length)?;
            let mut index = self.decoded.len() - offset;

            for _ in 0..seq.match_length {
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

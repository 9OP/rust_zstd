use super::{Error::*, Result};
use super::{HuffmanDecoder, SequenceDecoder};

pub struct DecodingContext {
    pub huffman: Option<HuffmanDecoder>,
    pub decoded: Vec<u8>,
    pub window_size: usize,
    pub sequence_decoder: Option<SequenceDecoder>,
    pub repeat_offsets: RepeatOffset,
}

pub struct RepeatOffset {
    pub offset_1: usize,
    pub offset_2: usize,
    pub offset_3: usize,
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

impl DecodingContext<'_> {
    /// Create a new decoding context instance. Return `WindowSizeError` when `window_size` exceeds 64Mb
    pub fn new(window_size: usize) -> Result<Self> {
        if window_size > MAX_WINDOW_SIZE {
            return Err(WindowSizeError);
        }
        Ok(Self {
            huffman: None,
            decoded: Vec::<u8>::new(),
            window_size,
            sequence_decoder: None,
            repeat_offsets: RepeatOffset {
                offset_1: 1,
                offset_2: 4,
                offset_3: 8,
            },
        })
    }

    pub fn decode_offset(&mut self, offset: usize, literals_length: usize) -> Result<usize> {
        let offset = self.repeat_offsets.decode_offset(offset, literals_length);
        if offset > self.window_size as usize {
            return Err(OffsetError);
        }
        Ok(offset)
    }

    /// Execute the sequences while updating the offsets
    pub fn execute_sequences(
        &mut self,
        sequences: Vec<(usize, usize, usize)>,
        literals: &[u8],
    ) -> Result<()> {
        let mut buffer = Vec::<u8>::new();
        let mut literals = literals;

        for (literals_length, offset_value, match_value) in sequences {
            let offset_value = self.decode_offset(offset_value, literals_length)?;

            // TODO: return error or check ll<=buffer.len()
            assert!(literals_length <= literals.len());
            let (slice, rest) = literals.split_at(literals_length + 1);
            literals = rest;

            buffer.extend_from_slice(&literals);

            let mut index = buffer.len() - offset_value;
            while index != literals_length + match_value {
                // TODO: do not use unwrap / should panic explicitly with a messaage.
                // panic is ok because this is a bug in the implementation
                buffer.push(*buffer.get(index).unwrap());
                index += 1;
            }
        }

        buffer.extend_from_slice(&literals);
        self.decoded.extend(buffer);
        Ok(())
    }
}

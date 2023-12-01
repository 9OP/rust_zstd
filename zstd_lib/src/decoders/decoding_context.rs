use super::HuffmanDecoder;
use super::{Error::*, Result, SymbolDecoder};

pub struct DecodingContext {
    pub decoded: Vec<u8>,
    pub window_size: usize,
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
            return Err(WindowSizeError);
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
        let mut copy_index = 0;

        for (literals_length, offset_value, match_value) in sequences {
            // TODO: return error or check ll<=buffer.len()
            assert!(literals_length + copy_index <= literals.len());
            let slice = &literals[(copy_index)..(copy_index + literals_length)];
            copy_index += literals_length;

            self.decoded.extend_from_slice(&slice);

            let offset_value = self.decode_offset(offset_value, literals_length)?;
            let mut index = self.decoded.len() - offset_value;

            for _ in 0..match_value {
                // TODO: do not use unwrap / should panic explicitly with a messaage.
                // panic is ok because this is a bug in the implementation
                self.decoded.push(*self.decoded.get(index).unwrap());
                index += 1;
            }
        }

        let (_, rest) = literals.split_at(copy_index);
        self.decoded.extend_from_slice(rest);

        Ok(())
    }
}

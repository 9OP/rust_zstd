use super::{Error, HuffmanDecoder, Result, SequenceCommand, SequenceDecoder, SymbolDecoder};

#[derive(Debug, thiserror::Error)]
pub enum ContextError {
    #[error("Window size too large")]
    WindowSizeError,

    #[error("Offset size error")]
    OffsetError,

    #[error("Missing symbol decoder")]
    MissingSymbolDecoder,

    #[error("Not enough bytes: {requested} requested out of {available} available")]
    NotEnoughBytes { requested: usize, available: usize },

    #[error("Copy match error")]
    CopyMatchError,
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
    offset_1: usize,
    offset_2: usize,
    offset_3: usize,
}

impl RepeatOffset {
    /// Decode an offset and properly maintain the three repeat offsets
    fn compute_offset(&mut self, offset: usize, literals_length: usize) -> usize {
        match offset {
            1 => {
                if literals_length == 0 {
                    std::mem::swap(&mut self.offset_1, &mut self.offset_2);
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
                    std::mem::swap(&mut self.offset_1, &mut self.offset_2);
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

    pub fn get_sequence_decoder(&mut self) -> Result<SequenceDecoder<'_>> {
        Ok(SequenceDecoder::new(
            self.literals_lengths_decoder
                .as_mut()
                .ok_or(Error::Context(MissingSymbolDecoder))?,
            self.offsets_decoder
                .as_mut()
                .ok_or(Error::Context(MissingSymbolDecoder))?,
            self.match_lengths_decoder
                .as_mut()
                .ok_or(Error::Context(MissingSymbolDecoder))?,
        ))
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

    /// Execute a single sequence
    fn execute_sequence(&mut self, sequence: &SequenceCommand, literals: &[u8]) -> Result<()> {
        let SequenceCommand {
            offset,
            literal_length,
            match_length,
        } = *sequence;

        if literal_length > literals.len() {
            return Err(Error::Context(NotEnoughBytes {
                requested: literal_length,
                available: literals.len(),
            }));
        }

        // Copy from literals
        self.decoded.extend_from_slice(&literals[..literal_length]);

        // Offset + match copy
        let mut index = self.decoded.len() - self.compute_offset(offset, literal_length)?;

        for _ in 0..match_length {
            let byte = self
                .decoded
                .get(index)
                .ok_or(Error::Context(CopyMatchError))?;
            self.decoded.push(*byte);
            index += 1;
        }

        Ok(())
    }

    /// Execute the sequences while updating the offsets
    pub fn execute_sequences(
        &mut self,
        sequences: Vec<SequenceCommand>,
        literals: &[u8],
    ) -> Result<()> {
        let mut position = 0;

        for sequence in sequences {
            self.execute_sequence(&sequence, &literals[position..])?;
            position += sequence.literal_length;
        }

        self.decoded.extend_from_slice(&literals[position..]);
        Ok(())
    }
}

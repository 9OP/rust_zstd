use std::{sync::Arc, thread};

use crate::{
    decoders::{DecodingContext, Error as DecoderErrors, HuffmanDecoder},
    parsing::{BackwardBitParser, Error as ParsingErrors, ForwardByteParser},
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Frame parsing error: {0}")]
    ParsingError(#[from] ParsingErrors),

    #[error("Decoder error: {0}")]
    DecoderError(#[from] DecoderErrors),

    #[error("Missing huffman decoder")]
    MissingHuffmanDecoder,

    #[error("Parallel decoding panicked")]
    ParallelDecodingError,

    #[error("Data corrupted")]
    CorruptedDataError,
}
use Error::*;
pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, PartialEq)]
pub enum LiteralsSection<'a> {
    RawLiteralsBlock(RawLiteralsBlock<'a>),
    RLELiteralsBlock(RLELiteralsBlock),
    CompressedLiteralsBlock(CompressedLiteralsBlock<'a>),
}

#[derive(Debug, PartialEq)]
pub struct RawLiteralsBlock<'a>(&'a [u8]);

#[derive(Debug, PartialEq)]
pub struct RLELiteralsBlock {
    byte: u8,
    repeat: usize,
}

#[derive(Debug, PartialEq)]
pub struct CompressedLiteralsBlock<'a> {
    huffman: Option<HuffmanDecoder>,
    regenerated_size: usize,
    jump_table: Option<[usize; 3]>,
    data: &'a [u8],
}

const RAW_LITERALS_BLOCK: u8 = 0;
const RLE_LITERALS_BLOCK: u8 = 1;
const COMPRESSED_LITERALS_BLOCK: u8 = 2;
const TREELESS_LITERALS_BLOCK: u8 = 3;

impl<'a> LiteralsSection<'a> {
    /// Decompress the literals section. Update the Huffman decoder in
    /// `context` if appropriate (compressed literals block with a
    /// Huffman table inside).
    pub fn decode(self, context: &mut DecodingContext) -> Result<Vec<u8>> {
        match self {
            LiteralsSection::RawLiteralsBlock(block) => {
                let decoded = Vec::from(block.0);
                Ok(decoded)
            }

            LiteralsSection::RLELiteralsBlock(block) => {
                let decoded = vec![block.byte; block.repeat];
                Ok(decoded)
            }

            LiteralsSection::CompressedLiteralsBlock(block) => {
                let mut decoded = vec![];

                if let Some(huffman) = block.huffman {
                    context.huffman = Some(huffman);
                }

                let huffman = context.huffman.clone().ok_or(MissingHuffmanDecoder)?;

                match block.jump_table {
                    None => {
                        let mut bitstream = BackwardBitParser::new(block.data)?;

                        while bitstream.available_bits() > 0 {
                            decoded.push(huffman.decode(&mut bitstream)?);
                        }
                    }

                    Some([stream1_size, stream2_size, stream3_size]) => {
                        let idx2 = stream1_size;
                        let idx3 = idx2 + stream2_size;
                        let idx4 = idx3 + stream3_size;

                        let data = Arc::new(Vec::from(block.data));
                        let decoder = Arc::new(huffman);

                        fn process(
                            decoder: Arc<HuffmanDecoder>,
                            data: Arc<Vec<u8>>,
                            range: (usize, usize),
                        ) -> thread::JoinHandle<Result<Vec<u8>>> {
                            thread::spawn(move || -> Result<Vec<u8>> {
                                let mut decoded = vec![];
                                let mut stream =
                                    BackwardBitParser::new(&data.as_slice()[range.0..range.1])?;
                                while stream.available_bits() > 0 {
                                    decoded.push(decoder.decode(&mut stream)?)
                                }
                                Ok(decoded)
                            })
                        }

                        let r1 = (0, idx2);
                        let r2 = (idx2, idx3);
                        let r3 = (idx3, idx4);
                        let r4 = (idx4, data.len());

                        let h1 = process(Arc::clone(&decoder), Arc::clone(&data), r1);
                        let h2 = process(Arc::clone(&decoder), Arc::clone(&data), r2);
                        let h3 = process(Arc::clone(&decoder), Arc::clone(&data), r3);
                        let h4 = process(Arc::clone(&decoder), Arc::clone(&data), r4);

                        let stream1 = h1.join().map_err(|_| ParallelDecodingError)??;
                        let stream2 = h2.join().map_err(|_| ParallelDecodingError)??;
                        let stream3 = h3.join().map_err(|_| ParallelDecodingError)??;
                        let stream4 = h4.join().map_err(|_| ParallelDecodingError)??;

                        decoded.extend(stream1);
                        decoded.extend(stream2);
                        decoded.extend(stream3);
                        decoded.extend(stream4);
                    }
                }

                Ok(decoded)
            }
        }
    }

    pub fn parse(input: &mut ForwardByteParser<'a>) -> Result<Self> {
        let header = input.u8()?;
        let block_type = header & 0b0000_0011;
        let size_format = (header & 0b0000_1100) >> 2;

        match block_type {
            RAW_LITERALS_BLOCK | RLE_LITERALS_BLOCK => {
                let regenerated_size: usize = match size_format {
                    // use 5bits (8 - 3)
                    0b00 | 0b10 => (header >> 3).into(),
                    // use 12bits (8 + 4)
                    0b01 => header as usize >> 4 | (input.u8()? as usize) << 4,
                    // use 20bits (8 + 8 + 4)
                    0b11 => {
                        header as usize >> 4
                            | (input.u8()? as usize) << 4
                            | (input.u8()? as usize) << 12
                    }
                    _ => panic!("unexpected size_format {size_format}"),
                };

                match block_type {
                    RAW_LITERALS_BLOCK => Ok(LiteralsSection::RawLiteralsBlock(RawLiteralsBlock(
                        input.slice(regenerated_size)?,
                    ))),
                    RLE_LITERALS_BLOCK => Ok(LiteralsSection::RLELiteralsBlock(RLELiteralsBlock {
                        byte: input.u8()?,
                        repeat: regenerated_size,
                    })),
                    _ => panic!("unexpected block_type {block_type}"),
                }
            }

            COMPRESSED_LITERALS_BLOCK | TREELESS_LITERALS_BLOCK => {
                let header: usize = header.into();
                let streams = match size_format {
                    0b00 => 1,
                    0b01 | 0b10 | 0b11 => 4,
                    _ => panic!("unexpected size_format {size_format}"),
                };
                let (regenerated_size, compressed_size) = match size_format {
                    0b00 | 0b01 => {
                        let header1 = input.u8()? as usize;
                        let header2 = input.u8()? as usize;

                        // both size on 10bits
                        let re_size = header >> 4 | (header1 & 0b0011_1111) << 4;
                        let cp_size = header1 >> 6 | header2 << 2;

                        (re_size, cp_size)
                    }
                    0b10 => {
                        let header1 = input.u8()? as usize;
                        let header2 = input.u8()? as usize;
                        let header3 = input.u8()? as usize;

                        // both size on 14bits
                        let re_size = header >> 4 | header1 << 4 | (header2 & 0b0000_0011) << 12;
                        let cp_size = header2 >> 2 | header3 << 6;

                        (re_size, cp_size)
                    }
                    0b11 => {
                        let header1 = input.u8()? as usize;
                        let header2 = input.u8()? as usize;
                        let header3 = input.u8()? as usize;
                        let header4 = input.u8()? as usize;

                        // both size on 18bits
                        let re_size = header >> 4 | header1 << 4 | (header2 & 0b0011_1111) << 12;
                        let cp_size = header2 >> 6 | header3 << 2 | header4 << 10;

                        (re_size, cp_size)
                    }
                    _ => panic!("unexpected size_format {size_format}"),
                };

                let mut huffman = None;
                let mut huffman_description_size = 0;

                if block_type == COMPRESSED_LITERALS_BLOCK {
                    let size_before = input.len();
                    huffman = Some(HuffmanDecoder::parse(input)?);
                    let size_after = input.len();
                    assert!(size_before > size_after);
                    huffman_description_size = size_before - size_after;
                }

                // Actual total_streams_size depend on the number of streams.
                // If there are 4 streams, 6bytes are remove from the total size to store
                // the respective streams size.
                let mut total_streams_size: usize = compressed_size - huffman_description_size;

                let jump_table = match streams {
                    1 => None,
                    4 => {
                        total_streams_size -= 6;

                        let stream1_size = input.le(2)?;
                        let stream2_size = input.le(2)?;
                        let stream3_size = input.le(2)?;
                        let stream4_size =
                            total_streams_size - stream1_size - stream2_size - stream3_size;

                        if stream4_size < 1 {
                            return Err(CorruptedDataError);
                        }

                        Some([stream1_size, stream2_size, stream3_size])
                    }
                    _ => panic!("unexpected number of streams {streams}"),
                };

                let data = input.slice(total_streams_size)?;

                Ok(LiteralsSection::CompressedLiteralsBlock(
                    CompressedLiteralsBlock {
                        huffman,
                        regenerated_size,
                        jump_table,
                        data,
                    },
                ))
            }
            _ => panic!("unexpected block_type {block_type}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::parsing::ForwardByteParser;

    use super::*;

    #[test]
    fn test_parse_raw_literal() {
        let mut input = ForwardByteParser::new(&[0b0000_1000, 0xFF]);
        assert_eq!(
            LiteralsSection::parse(&mut input).unwrap(),
            LiteralsSection::RawLiteralsBlock(RawLiteralsBlock(&[0xFF]))
        );

        let mut input = ForwardByteParser::new(&[0b0000_0000]);
        assert_eq!(
            LiteralsSection::parse(&mut input).unwrap(),
            LiteralsSection::RawLiteralsBlock(RawLiteralsBlock(&[]))
        );

        let mut input = ForwardByteParser::new(&[0b0100_0100, 0x0000_0000, 0xAA, 0xBB, 0xCC, 0xDD]);
        assert_eq!(
            LiteralsSection::parse(&mut input).unwrap(),
            LiteralsSection::RawLiteralsBlock(RawLiteralsBlock(&[0xAA, 0xBB, 0xCC, 0xDD]))
        );

        let mut input = ForwardByteParser::new(&[0b0010_1100, 0x0, 0x0, 0xAA, 0xBB]);
        assert_eq!(
            LiteralsSection::parse(&mut input).unwrap(),
            LiteralsSection::RawLiteralsBlock(RawLiteralsBlock(&[0xAA, 0xBB]))
        );
    }

    #[test]
    fn test_parse_rle_literal() {
        let mut input = ForwardByteParser::new(&[0b0000_0001, 0xFF]);
        assert_eq!(
            LiteralsSection::parse(&mut input).unwrap(),
            LiteralsSection::RLELiteralsBlock(RLELiteralsBlock {
                byte: 0xFF,
                repeat: 0
            })
        );

        let mut input = ForwardByteParser::new(&[0b0000_1001, 0xFF]);
        assert_eq!(
            LiteralsSection::parse(&mut input).unwrap(),
            LiteralsSection::RLELiteralsBlock(RLELiteralsBlock {
                byte: 0xFF,
                repeat: 1
            })
        );

        let mut input = ForwardByteParser::new(&[0b0101_0101, 0b1101_0101, 0xFF]);
        assert_eq!(
            LiteralsSection::parse(&mut input).unwrap(),
            LiteralsSection::RLELiteralsBlock(RLELiteralsBlock {
                byte: 0xFF,
                repeat: 0b1101_0101_0101,
            })
        );

        let mut input = ForwardByteParser::new(&[0b0101_1101, 0b1101_0101, 0b1111_0000, 0xFF]);
        assert_eq!(
            LiteralsSection::parse(&mut input).unwrap(),
            LiteralsSection::RLELiteralsBlock(RLELiteralsBlock {
                byte: 0xFF,
                repeat: 0b1111_0000_1101_0101_0101,
            })
        );
    }

    #[test]
    fn test_parse_compressed_literal() {
        // TODO
    }

    #[test]
    fn test_parse_treeless_literal() {
        // TODO
    }
}

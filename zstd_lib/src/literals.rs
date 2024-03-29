use super::{BackwardBitParser, DecodingContext, Error, ForwardByteParser, HuffmanDecoder, Result};
use std::{
    sync::{Arc, Mutex},
    thread,
};

#[derive(Debug, thiserror::Error)]
pub enum LiteralsError {
    #[error("Missing huffman decoder")]
    MissingHuffmanDecoder,

    #[error("Data corrupted")]
    CorruptedDataError,

    #[error("Compressed size is invalid")]
    InvalidCompressedSize,

    #[error("Regenerated size error")]
    RegneratedSizeError,
}
use LiteralsError::*;

#[derive(Debug, PartialEq)]
pub enum LiteralsSection<'a> {
    Raw(RawLiteralsBlock<'a>),
    Rle(RLELiteralsBlock),
    Compressed(CompressedLiteralsBlock<'a>),
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

const MAX_LITERALS_SIZE: usize = 1024 * 128; // 128kb

impl<'a> LiteralsSection<'a> {
    /// Decompress the literals section. Update the Huffman decoder in
    /// `context` if appropriate (compressed literals block with a
    /// Huffman table inside).
    pub fn decode(self, shared_context: &Arc<Mutex<&mut DecodingContext>>) -> Result<Vec<u8>> {
        match self {
            LiteralsSection::Raw(block) => Ok(Vec::from(block.0)),
            LiteralsSection::Rle(block) => Ok(vec![block.byte; block.repeat]),
            LiteralsSection::Compressed(block) => match block.jump_table {
                None => decode_1_stream(shared_context, block),
                Some(jump_table) => decode_4_streams(jump_table, shared_context, block),
            },
        }
    }

    #[allow(clippy::too_many_lines)]
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

                if regenerated_size > MAX_LITERALS_SIZE {
                    return Err(Error::Literals(CorruptedDataError));
                }

                match block_type {
                    RAW_LITERALS_BLOCK => Ok(LiteralsSection::Raw(RawLiteralsBlock(
                        input.slice(regenerated_size)?,
                    ))),
                    RLE_LITERALS_BLOCK => Ok(LiteralsSection::Rle(RLELiteralsBlock {
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
                    #[allow(clippy::manual_range_patterns)]
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

                if regenerated_size > MAX_LITERALS_SIZE {
                    return Err(Error::Literals(CorruptedDataError));
                }

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
                // If there are 4 streams, 6bytes are removed from the total size to store
                // the respective streams size.
                if compressed_size < huffman_description_size {
                    return Err(Error::Literals(InvalidCompressedSize));
                }
                let mut total_streams_size: usize = compressed_size - huffman_description_size;

                let jump_table = match streams {
                    1 => None,
                    4 => {
                        let stream1_size = input.le(2)?;
                        let stream2_size = input.le(2)?;
                        let stream3_size = input.le(2)?;

                        if total_streams_size < stream1_size + stream2_size + stream3_size + 6 + 1 {
                            return Err(Error::Literals(CorruptedDataError));
                        }

                        total_streams_size -= 6;

                        Some([stream1_size, stream2_size, stream3_size])
                    }
                    _ => panic!("unexpected number of streams {streams}"),
                };

                let data = input.slice(total_streams_size)?;

                Ok(LiteralsSection::Compressed(CompressedLiteralsBlock {
                    huffman,
                    regenerated_size,
                    jump_table,
                    data,
                }))
            }
            _ => panic!("unexpected block_type {block_type}"),
        }
    }
}

fn update_decoder(
    shared_context: &Arc<Mutex<&mut DecodingContext>>,
    block_huffman: Option<HuffmanDecoder>,
) -> Result<HuffmanDecoder> {
    let mut ctx = shared_context.lock().unwrap();
    if let Some(huffman) = block_huffman {
        ctx.huffman = Some(huffman);
    }

    // We need to clone the decoder to send it to move it to threads
    let huffman = ctx.huffman.clone().ok_or(MissingHuffmanDecoder)?;
    Ok(huffman)
}

fn decode_1_stream(
    shared_context: &Arc<Mutex<&mut DecodingContext>>,
    block: CompressedLiteralsBlock,
) -> Result<Vec<u8>> {
    let mut decoded = vec![];
    let huffman = update_decoder(shared_context, block.huffman)?;
    let mut bitstream = BackwardBitParser::new(block.data)?;

    while bitstream.available_bits() > 0 {
        decoded.push(huffman.decode(&mut bitstream)?);
    }

    Ok(decoded)
}

fn decode_4_streams(
    jump_table: [usize; 3],
    shared_context: &Arc<Mutex<&mut DecodingContext>>,
    block: CompressedLiteralsBlock,
) -> Result<Vec<u8>> {
    let mut decoded = vec![];
    let huffman = update_decoder(shared_context, block.huffman)?;

    let idx2 = jump_table[0];
    let idx3 = idx2 + jump_table[1];
    let idx4 = idx3 + jump_table[2];
    assert!(idx4 > idx3 && idx3 > idx2);

    let ranges: [(usize, usize); 4] = [
        (0, idx2),
        (idx2, idx3),
        (idx3, idx4),
        (idx4, block.data.len()),
    ];

    let regenerated_stream_size = (block.regenerated_size + 3) / 4;
    let data = Arc::new(Vec::from(block.data));
    let huffman_decoder = Arc::new(huffman);

    let handles: Vec<_> = ranges
        .into_iter()
        .map(|r| {
            let data = Arc::clone(&data);
            let huffman_decoder = Arc::clone(&huffman_decoder);

            thread::spawn(move || -> Result<Vec<u8>> {
                let mut decoded = vec![];
                let mut stream = BackwardBitParser::new(&data[r.0..r.1])?;
                while stream.available_bits() > 0 {
                    decoded.push(huffman_decoder.decode(&mut stream)?);
                }

                Ok(decoded)
            })
        })
        .collect();

    assert!(handles.len() == 4);

    for (id, handle) in handles.into_iter().enumerate() {
        let stream = handle.join().map_err(|_| Error::ParallelDecodingError)??;

        if id < 3 && stream.len() != regenerated_stream_size {
            return Err(Error::Literals(RegneratedSizeError));
        }

        decoded.extend(stream);
    }

    Ok(decoded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_raw_literal() {
        let mut input = ForwardByteParser::new(&[0b0000_1000, 0xFF]);
        assert_eq!(
            LiteralsSection::parse(&mut input).unwrap(),
            LiteralsSection::Raw(RawLiteralsBlock(&[0xFF]))
        );

        let mut input = ForwardByteParser::new(&[0b0000_0000]);
        assert_eq!(
            LiteralsSection::parse(&mut input).unwrap(),
            LiteralsSection::Raw(RawLiteralsBlock(&[]))
        );

        let mut input = ForwardByteParser::new(&[0b0100_0100, 0x0000_0000, 0xAA, 0xBB, 0xCC, 0xDD]);
        assert_eq!(
            LiteralsSection::parse(&mut input).unwrap(),
            LiteralsSection::Raw(RawLiteralsBlock(&[0xAA, 0xBB, 0xCC, 0xDD]))
        );

        let mut input = ForwardByteParser::new(&[0b0010_1100, 0x0, 0x0, 0xAA, 0xBB]);
        assert_eq!(
            LiteralsSection::parse(&mut input).unwrap(),
            LiteralsSection::Raw(RawLiteralsBlock(&[0xAA, 0xBB]))
        );
    }

    #[test]
    fn test_parse_rle_literal() {
        let mut input = ForwardByteParser::new(&[0b0000_0001, 0xFF]);
        assert_eq!(
            LiteralsSection::parse(&mut input).unwrap(),
            LiteralsSection::Rle(RLELiteralsBlock {
                byte: 0xFF,
                repeat: 0
            })
        );

        let mut input = ForwardByteParser::new(&[0b0000_1001, 0xFF]);
        assert_eq!(
            LiteralsSection::parse(&mut input).unwrap(),
            LiteralsSection::Rle(RLELiteralsBlock {
                byte: 0xFF,
                repeat: 1
            })
        );

        let mut input = ForwardByteParser::new(&[0b0101_0101, 0b1101_0101, 0xFF]);
        assert_eq!(
            LiteralsSection::parse(&mut input).unwrap(),
            LiteralsSection::Rle(RLELiteralsBlock {
                byte: 0xFF,
                repeat: 0b1101_0101_0101,
            })
        );
    }
}

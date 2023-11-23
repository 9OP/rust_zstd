use crate::{decoders, parsing};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Frame parsing error: {0}")]
    ParsingError(#[from] parsing::Error),

    #[error("Decoder error: {0}")]
    DecoderError(#[from] decoders::Error),

    #[error("Data corrupted")]
    CorruptedError,
}
pub type Result<T, E = Error> = std::result::Result<T, E>;

pub enum LiteralsSection<'a> {
    RawLiteralsBlock(RawLiteralsBlock<'a>),
    RLELiteralsBlock(RLELiteralsBlock),
    CompressedLiteralsBlock(CompressedLiteralsBlock<'a>),
}

const RAW_LITERALS_BLOCK: u8 = 0;
const RLE_LITERALS_BLOCK: u8 = 1;
const COMPRESSED_LITERALS_BLOCK: u8 = 2;
const TREELESS_LITERALS_BLOCK: u8 = 3;

pub struct RawLiteralsBlock<'a>(&'a [u8]);

pub struct RLELiteralsBlock {
    byte: u8,
    repetition: usize,
}

pub struct CompressedLiteralsBlock<'a> {
    huffman: Option<decoders::HuffmanDecoder>,
    regenerated_size: usize,
    jump_table: Option<[u16; 3]>, // three 2-bytes long offsets
    data: &'a [u8],
}

impl<'a> LiteralsSection<'a> {
    /// Decompress the literals section. Update the Huffman decoder in
    /// `context` if appropriate (compressed literals block with a
    /// Huffman table inside).
    pub fn decode(self, context: &mut decoders::DecodingContext) -> Result<Vec<u8>> {
        todo!()
    }

    pub fn parse(input: &mut parsing::ForwardByteParser<'a>) -> Result<Self> {
        let header = input.u8()?;
        let block_type = header & 0b0000_0011;

        match block_type {
            RAW_LITERALS_BLOCK | RLE_LITERALS_BLOCK => {
                let size_format = (header & 0b0000_1100) >> 2;
                let regenerated_size: usize = match size_format {
                    // use 5bits (8 - 3)
                    0b00 | 0b10 => (header >> 3).into(),
                    // use 12bits (8 + 4)
                    0b01 => (header as usize | (input.u8()? as usize) << 8) >> 4,
                    // use 20bits (8 + 8 + 4)
                    0b11 => {
                        (header as usize
                            | (input.u8()? as usize) << 8
                            | (input.u8()? as usize) << 16)
                            >> 4
                    }
                    _ => panic!("unexpected size_format {size_format}"),
                };

                match block_type {
                    RAW_LITERALS_BLOCK => Ok(LiteralsSection::RawLiteralsBlock(RawLiteralsBlock(
                        input.slice(regenerated_size)?,
                    ))),
                    RLE_LITERALS_BLOCK => Ok(LiteralsSection::RLELiteralsBlock(RLELiteralsBlock {
                        byte: input.u8()?,
                        repetition: regenerated_size,
                    })),
                    _ => panic!("unexpected block_type {block_type}"),
                }
            }

            COMPRESSED_LITERALS_BLOCK | TREELESS_LITERALS_BLOCK => {
                let header: usize = header.into();
                let size_format = (header & 0b0000_1100) >> 2;
                let (regenerated_size, compressed_size, streams) = match size_format {
                    0b00 => {
                        let header1 = input.u8()? as usize;
                        let header2 = input.u8()? as usize;

                        // both size on 10bits
                        let re_size = header >> 4 | (header1 & 0b0011_1111) << 4;
                        let cp_size = header1 >> 6 | header2 << 2;

                        (re_size, cp_size, 1)
                    }
                    0b01 => {
                        let header1 = input.u8()? as usize;
                        let header2 = input.u8()? as usize;

                        // both size on 10bits
                        let re_size = header >> 4 | (header1 & 0b0011_1111) << 4;
                        let cp_size = header1 >> 6 | header2 << 2;

                        (re_size, cp_size, 4)
                    }
                    0b10 => {
                        let header1 = input.u8()? as usize;
                        let header2 = input.u8()? as usize;
                        let header3 = input.u8()? as usize;

                        // both size on 14bits
                        let re_size = header >> 4 | header1 << 4 | (header2 & 0b0000_0011) << 12;
                        let cp_size = header2 >> 2 | header3 << 6;

                        (re_size, cp_size, 4)
                    }
                    0b11 => {
                        let header1 = input.u8()? as usize;
                        let header2 = input.u8()? as usize;
                        let header3 = input.u8()? as usize;
                        let header4 = input.u8()? as usize;

                        // both size on 18bits
                        let re_size = header >> 4 | header1 << 4 | (header2 & 0b0011_1111) << 12;
                        let cp_size = header2 >> 6 | header3 << 2 | header4 << 10;

                        (re_size, cp_size, 4)
                    }
                    _ => panic!("unexpected size_format {size_format}"),
                };

                let mut huffman = None;
                let mut huffman_description_size = 0;

                if block_type == COMPRESSED_LITERALS_BLOCK {
                    let size_before = input.len();
                    huffman = Some(decoders::HuffmanDecoder::parse(input)?);
                    let size_after = input.len();
                    assert!(size_before > size_after);
                    huffman_description_size = size_before - size_after;
                }

                let total_streams_size = compressed_size - huffman_description_size;

                let jump_table = match streams {
                    1 => None,
                    4 => {
                        // Decompressed size of the first 3 streams
                        let decompressed_size = (regenerated_size + 3) / 4;
                        // size of the last stream is: total_streams_size-3*decompressed_size
                        assert!(3 * decompressed_size < total_streams_size);
                        // TODO: assert the conversion usize->u16 do not overflow
                        Some([
                            decompressed_size as u16,
                            2 * decompressed_size as u16,
                            3 * decompressed_size as u16,
                        ])
                    }
                    _ => panic!("unexpected streams {streams}"),
                };

                Ok(LiteralsSection::CompressedLiteralsBlock(
                    CompressedLiteralsBlock {
                        huffman,
                        regenerated_size,
                        jump_table,
                        data: input.slice(total_streams_size)?,
                    },
                ))
            }
            _ => panic!("unexpected block_type {block_type}"),
        }
    }
}

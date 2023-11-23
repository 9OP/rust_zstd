use crate::{decoders::HuffmanDecoder, parsing};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Frame parsing error: {0}")]
    ParsingError(#[from] parsing::Error),
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
    huffman: Option<HuffmanDecoder>,
    regenerated_size: usize,
    jump_table: [u8; 3],
    data: &'a [u8],
}

impl<'a> LiteralsSection<'a> {
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
                let (regenerated_size, compressed_size) = match size_format {
                    0b00 => {
                        let header1 = input.u8()? as usize;
                        let header2 = input.u8()? as usize;

                        // both size on 10bits
                        let re_size = header >> 4 | (header1 & 0b0011_1111) << 4;
                        let cp_size = header1 >> 6 | header2 << 2;

                        (re_size, cp_size)
                    }
                    0b01 => {
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

                // Ok(LiteralsSection::CompressedLiteralsBlock(
                //     CompressedLiteralsBlock{input.slice(regenerated_size)?},
                // ));

                Ok(LiteralsSection::RawLiteralsBlock(RawLiteralsBlock(
                    input.slice(regenerated_size)?,
                )))
            }

            _ => panic!("unexpected block_type {block_type}"),
        }
    }
}

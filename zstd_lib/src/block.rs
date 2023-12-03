use super::{DecodingContext, Error, ForwardByteParser, LiteralsSection, Result, Sequences};

#[derive(Debug, thiserror::Error)]
pub enum BlockError {
    #[error("Reserved block type")]
    ReservedBlockType,
}
use BlockError::*;

#[derive(Debug)]
pub enum Block<'a> {
    Raw(&'a [u8]),
    Rle {
        byte: u8,
        repeat: usize,
    },
    Compressed {
        literals: LiteralsSection<'a>,
        sequences: Sequences<'a>,
    },
}

const RAW_BLOCK_FLAG: u8 = 0;
const RLE_BLOCK_FLAG: u8 = 1;
const COMPRESSED_BLOCK_FLAG: u8 = 2;
const RESERVED_BLOCK_FLAG: u8 = 3;

const BLOCK_SIZE_MAX: usize = 1024 * 128; // 128kb

impl<'a> Block<'a> {
    pub fn parse(
        input: &mut ForwardByteParser<'a>,
        window_size: usize,
    ) -> Result<(Block<'a>, bool)> {
        let header = input.slice(3)?;

        // Parse header with bit-mask and bit-shifts:
        //  last_block is LSB bit0
        //  block_type is bits1-2
        //  block_size is bits3-23 (need to Rshift by 3)
        let last_block = (header[0] & 0b0000_0001) != 0;
        let block_type = (header[0] & 0b0000_0110) >> 1;
        let block_size =
            ((header[2] as usize) << 16 | (header[1] as usize) << 8 | (header[0] as usize)) >> 3;

        match block_type {
            RAW_BLOCK_FLAG => {
                let raw_data = input.slice(block_size)?;
                let block = Block::Raw(raw_data);
                Ok((block, last_block))
            }

            RLE_BLOCK_FLAG => {
                let byte = input.u8()?;
                let block = Block::Rle {
                    repeat: block_size,
                    byte,
                };
                Ok((block, last_block))
            }

            COMPRESSED_BLOCK_FLAG => {
                // The size of Block_Content is limited by the smallest of:
                // window_size or 128 KB
                let max_block_size = std::cmp::min(BLOCK_SIZE_MAX, window_size);
                let block_size = std::cmp::min(block_size, max_block_size);

                let compressed_data = input.slice(block_size)?;
                let mut parser = ForwardByteParser::new(compressed_data);

                let literals = LiteralsSection::parse(&mut parser)?;
                let sequences = Sequences::parse(&mut parser)?;

                let block = Block::Compressed {
                    literals,
                    sequences,
                };

                Ok((block, last_block))
            }

            RESERVED_BLOCK_FLAG => Err(Error::Block(ReservedBlockType)),

            _ => panic!("unexpected block_type {block_type}"),
        }
    }

    pub fn decode(self, context: &mut DecodingContext) -> Result<()> {
        match self {
            Block::Raw(v) => {
                let decoded = Vec::from(v);
                context.decoded.extend(decoded);
            }
            Block::Rle { byte, repeat } => {
                let decoded = vec![byte; repeat];
                context.decoded.extend(decoded);
            }
            Block::Compressed {
                literals,
                sequences,
            } => {
                let literals = literals.decode(context)?;
                let sequences = sequences.decode(context)?;

                context.execute_sequences(sequences, literals.as_slice())?;
            }
        };

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{super::ParsingError, *};

    mod parse {
        use super::*;

        #[test]
        fn test_parse_raw_block_last() {
            let mut parser = ForwardByteParser::new(&[
                0b0010_0001,
                0x0,
                0x0, // raw, last, len 4
                0x10,
                0x20,
                0x30,
                0x40, // content
                0x50, // +extra byte
            ]);
            let (block, last) = Block::parse(&mut parser, 1024).unwrap();
            assert!(last);
            assert!(matches!(block, Block::Raw(&[0x10, 0x20, 0x30, 0x40])));
            assert_eq!(parser.len(), 1);
        }

        #[test]
        fn test_parse_rle_block_not_last() {
            let mut parser = ForwardByteParser::new(&[
                0x22, 0x0, 0x18, // rle, not last, repeat  0x30004
                0x42, // content
                0x50, // +extra byte
            ]);
            let (block, last) = Block::parse(&mut parser, 1024).unwrap();
            assert!(!last);
            assert!(matches!(
                block,
                Block::Rle {
                    byte: 0x42,
                    repeat: 196612
                }
            ));
            assert_eq!(parser.len(), 1);
        }

        #[test]
        fn test_parse_reserved() {
            let mut parser = ForwardByteParser::new(&[
                0b0000_0110,
                0x0,
                0x0, // reserved
            ]);
            assert!(matches!(
                Block::parse(&mut parser, 1024),
                Err(Error::Block(ReservedBlockType))
            ));
        }

        #[test]
        fn test_parse_not_enough_byte() {
            let mut parser = ForwardByteParser::new(&[0x0, 0x0]);
            assert!(matches!(
                Block::parse(&mut parser, 1024),
                Err(Error::Parsing(ParsingError::NotEnoughBytes {
                    requested: 3,
                    available: 2
                }))
            ));

            assert_eq!(parser.len(), 2);
        }

        #[test]
        fn test_parse_rle_not_enough_byte() {
            let mut parser = ForwardByteParser::new(&[
                0b0000_0010,
                0x0,
                0x0, // RLE not last
            ]);
            assert!(matches!(
                Block::parse(&mut parser, 1024),
                Err(Error::Parsing(ParsingError::NotEnoughBytes {
                    requested: 1,
                    available: 0
                }))
            ));
            assert_eq!(parser.len(), 0);
        }

        #[test]
        fn test_parse_raw_block_not_enough_size() {
            let mut parser = ForwardByteParser::new(&[
                // Raw block, not last, len 8, content len 3
                0b0010_0000,
                0x0,
                0x0, // raw, not last, len 4
                0x10,
                0x20,
                0x30, // content
            ]);
            assert!(matches!(
                Block::parse(&mut parser, 1024),
                Err(Error::Parsing(ParsingError::NotEnoughBytes {
                    requested: 4,
                    available: 3
                }))
            ));
            assert_eq!(parser.len(), 3);
        }
    }

    mod decode {
        use super::*;

        #[test]
        fn test_decode_raw() {
            let mut ctx = DecodingContext::new(0).unwrap();
            let block = Block::Raw(&[0x10, 0x20, 0x30, 0x40]);
            block.decode(&mut ctx).unwrap();
            assert_eq!(ctx.decoded, vec![0x10, 0x20, 0x30, 0x40]);
        }

        #[test]
        fn test_decode_rle() {
            let mut ctx = DecodingContext::new(0).unwrap();
            let block = Block::Rle {
                byte: 0x42,
                repeat: 196612,
            };
            block.decode(&mut ctx).unwrap();
            assert_eq!(196612, ctx.decoded.len());
            assert!(ctx.decoded.into_iter().all(|b| b == 0x42));
        }

        #[test]
        fn test_decode_compressed() {
            // bitstream obtained via the reference implementation

            let mut ctx = DecodingContext::new(1000).unwrap();
            let bitstream = [
                189, 1, 0, 228, 2, 35, 35, 10, 35, 32, 87, 101, 108, 99, 111, 109, 101, 32, 116,
                111, 32, 84, 101, 108, 101, 99, 111, 109, 32, 80, 97, 114, 105, 115, 32, 122, 115,
                116, 100, 32, 101, 120, 97, 109, 112, 108, 101, 32, 35, 10, 35, 2, 0, 12, 202, 162,
                4, 109, 63, 5, 217, 139,
            ];
            let mut parser = ForwardByteParser::new(&bitstream);
            let (block, _) = Block::parse(&mut parser, 1024).unwrap();
            block.decode(&mut ctx).unwrap();
            let decoded = String::from_utf8(ctx.decoded).unwrap();

            let expected = r##"
#########################################
# Welcome to Telecom Paris zstd example #
#########################################
            "##;

            assert_eq!(expected.trim(), decoded);
        }
    }
}

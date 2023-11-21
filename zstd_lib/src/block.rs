use crate::parsing;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Block parsing error: {0}")]
    ParsingError(#[from] parsing::Error),

    #[error("Reserved block type")]
    ReservedBlockType,
}
use Error::*;
type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug)]
pub enum Block<'a> {
    Raw(&'a [u8]),
    RLE { byte: u8, repeat: usize },
}

const BLOCK_HEADER_LEN: usize = 3;
const RAW_BLOCK_FLAG: u8 = 0;
const RLE_BLOCK_FLAG: u8 = 1;
const COMPRESSED_BLOCK_FLAG: u8 = 2;
const RESERVED_BLOCK_FLAG: u8 = 3;

impl<'a> Block<'a> {
    pub fn parse(input: &mut parsing::ForwardByteParser<'a>) -> Result<(Block<'a>, bool)> {
        // TrustMe™ unwrap is safe, we know the len
        let header: &[u8; BLOCK_HEADER_LEN] = input.slice(BLOCK_HEADER_LEN)?.try_into().unwrap();

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
                let block = Block::RLE {
                    repeat: block_size,
                    byte,
                };
                Ok((block, last_block))
            }

            COMPRESSED_BLOCK_FLAG => unimplemented!(),

            RESERVED_BLOCK_FLAG => Err(ReservedBlockType),

            _ => panic!("unexpected block_type {block_type}"),
        }
    }

    pub fn decode(self) -> Vec<u8> {
        match self {
            Block::Raw(v) => Vec::from(v),
            Block::RLE { byte, repeat } => vec![byte; repeat],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[rustfmt::skip]
    mod parse {
        use super::*;

        #[test]
        fn test_parse_raw_block_last() {
            let mut parser = parsing::ForwardByteParser::new(&[
                0b0010_0001, 0x0, 0x0, // raw, last, len 4
                0x10, 0x20, 0x30, 0x40, // content
                0x50, // +extra byte
            ]);
            let (block, last) = Block::parse(&mut parser).unwrap();
            assert!(last);
            assert!(matches!(block, Block::Raw(&[0x10, 0x20, 0x30, 0x40])));
            assert_eq!(parser.len(), 1);
        }

        #[test]
        fn test_parse_rle_block_not_last() {
            let mut parser = parsing::ForwardByteParser::new(&[
                0x22, 0x0, 0x18, // rle, not last, repeat  0x30004
                0x42, // content
                0x50, // +extra byte
            ]);
            let (block, last) = Block::parse(&mut parser).unwrap();
            assert!(!last);
            assert!(matches!(
                block,
                Block::RLE {
                    byte: 0x42,
                    repeat: 196612
                }
            ));
            assert_eq!(parser.len(), 1);
        }

        #[test]
        fn test_parse_reserved() {
            let mut parser = parsing::ForwardByteParser::new(&[
                0b0000_0110, 0x0, 0x0, // reserved
            ]);
            assert!(matches!(Block::parse(&mut parser), Err(ReservedBlockType)));
        }

        #[test]
        fn test_parse_not_enough_byte() {
            let mut parser = parsing::ForwardByteParser::new(&[0x0, 0x0]);
            assert!(matches!(
                Block::parse(&mut parser),
                Err(ParsingError(parsing::Error::NotEnoughBytes {
                    requested: 3,
                    available: 2
                }))
            ));
            assert_eq!(parser.len(), 2);
        }

        #[test]
        fn test_parse_rle_not_enough_byte() {
            let mut parser = parsing::ForwardByteParser::new(&[
                0b0000_0010, 0x0, 0x0, // RLE not last
            ]);
            assert!(matches!(
                Block::parse(&mut parser),
                Err(ParsingError(parsing::Error::NotEnoughBytes {
                    requested: 1,
                    available: 0
                }))
            ));
            assert_eq!(parser.len(), 0);
        }

        #[test]
        fn test_parse_raw_block_not_enough_size() {
            let mut parser = parsing::ForwardByteParser::new(&[
                // Raw block, not last, len 8, content len 3
                0b0010_0000, 0x0, 0x0, // raw, not last, len 4
                0x10, 0x20, 0x30, // content
            ]);
            assert!(matches!(
                Block::parse(&mut parser),
                Err(ParsingError(parsing::Error::NotEnoughBytes {
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
            let block = Block::Raw(&[0x10, 0x20, 0x30, 0x40]);
            assert_eq!(block.decode(), vec![0x10, 0x20, 0x30, 0x40]);
        }

        #[test]
        fn test_decode_rle() {
            let block = Block::RLE {
                byte: 0x42,
                repeat: 196612,
            };
            let decoded = block.decode();
            assert_eq!(196612, decoded.len());
            assert!(decoded.into_iter().all(|b| b == 0x42));
        }
    }
}

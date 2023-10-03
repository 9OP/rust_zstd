use super::parsing;

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

            _ => Err(ReservedBlockType),
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

    #[test]
    fn test_decode_raw_block_last() {
        let mut parser = parsing::ForwardByteParser::new(&[
            // Raw block, last block, len 4, content 0x10, 0x20, 0x30, 0x40,
            // and an extra 0x50 at the end.
            0x21, 0x0, 0x0, 0x10, 0x20, 0x30, 0x40, 0x50,
        ]);
        let (block, last) = Block::parse(&mut parser).unwrap();
        assert!(last);
        assert!(matches!(block, Block::Raw(&[0x10, 0x20, 0x30, 0x40])));
        assert_eq!(1, parser.len());
        let decoded = block.decode();
        assert_eq!(vec![0x10, 0x20, 0x30, 0x40], decoded);
    }

    #[test]
    fn test_decode_rle_block_not_last() {
        let mut parser = parsing::ForwardByteParser::new(&[
            // RLE block, not last, byte 0x42 and repeat 0x30004,
            // and an extra 0x50 at the end.
            0x22, 0x0, 0x18, 0x42, 0x50,
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
        assert_eq!(1, parser.len());
        let decoded = block.decode();
        assert_eq!(196612, decoded.len());
        assert!(decoded.into_iter().all(|b| b == 0x42));
    }

    #[test]
    fn test_parse_reserved() {
        let mut parser = parsing::ForwardByteParser::new(&[
            // Reserved block
            0x06, 0x0, 0x0,
        ]);
        assert!(matches!(Block::parse(&mut parser), Err(ReservedBlockType)));
    }

    #[test]
    fn test_parse_raw_block_not_enough_size() {
        let mut parser = parsing::ForwardByteParser::new(&[
            // Raw block, not last, len 8, content len 3
            0x40, 0x0, 0x0, 0x10, 0x20, 0x30,
        ]);
        assert!(matches!(
            Block::parse(&mut parser),
            Err(ParsingError(parsing::Error::NotEnoughBytes {
                requested: 8,
                available: 3
            }))
        ));
        assert_eq!(parser.len(), 3);
    }

    #[test]
    fn test_parse_rle_not_enough_byte() {
        let mut parser = parsing::ForwardByteParser::new(&[
            // RLE block, not last,
            0x02, 0x0, 0x0,
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
    fn test_parse_header_not_enough_byte() {
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
}

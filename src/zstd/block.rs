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

pub enum Block<'a> {
    Raw(&'a [u8]),
    RLE { byte: u8, repeat: usize },
}
const BLOCK_HEADER_LEN: usize = 3;

impl<'a> Block<'a> {
    pub fn parse(input: &mut parsing::ForwardByteParser<'a>) -> Result<(Block<'a>, bool)> {
        // TrustMe™ unwrap is safe, we know the len
        let header: &[u8; BLOCK_HEADER_LEN] = input.slice(BLOCK_HEADER_LEN)?.try_into().unwrap();

        // Parse header:
        //  last_block is bit0
        //  block_type is bits1-2
        //  block_size is bits3-23
        let last_block = (header[0] & 0b0000_0001) != 0;
        let block_type = (header[0] & 0b0000_0110) >> 1;
        let block_size = (header[2] as usize) << (16 - BLOCK_HEADER_LEN)
            | (header[1] as usize) << (8 - BLOCK_HEADER_LEN)
            | (header[0] as usize) >> BLOCK_HEADER_LEN;

        match block_type {
            // Raw Block
            0 => {
                let raw_data = input.slice(block_size)?;
                Ok((Block::Raw(raw_data), last_block))
            }

            // RLE Block
            1 => {
                // TODO return error when input.len != 1
                // Blockformat error

                let byte = input.u8()?; // comsume first byte
                Ok((
                    Block::RLE {
                        repeat: block_size,
                        byte,
                    },
                    last_block,
                ))
            }

            // Compressed Block
            2 => unimplemented!(),

            // Reserved Block
            3 => Err(ReservedBlockType),
            _ => Err(ReservedBlockType),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_raw_block_last() {
        let mut parser = parsing::ForwardByteParser::new(&[
            // Raw block, last block, len 4, content 0x10, 0x20, 0x30, 0x40,
            // and an extra 0x50 at the end.
            0x21, 0x0, 0x0, 0x10, 0x20, 0x30, 0x40, 0x50,
        ]);
        let (block, last) = Block::parse(&mut parser).unwrap();
        dbg!(last);
        assert!(last);
        assert!(matches!(block, Block::Raw(&[0x10, 0x20, 0x30, 0x40])));
        assert_eq!(1, parser.len());
        //
        // let decoded = block.decode().unwrap();
        // assert_eq!(vec![0x10, 0x20, 0x30, 0x40], decoded);
    }

    #[test]
    fn decode_rle_block_not_last() {
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
        // let decoded = block.decode().unwrap();
        // assert_eq!(196612, decoded.len());
        // assert!(decoded.into_iter().all(|b| b == 0x42));
    }
}

use super::{Block, DecodingContext, Error, ForwardByteParser, Result};
use xxhash_rust::xxh64::xxh64;

#[derive(Debug, thiserror::Error)]
pub enum FrameError {
    #[error("Unrecognized magic number: {0}")]
    UnrecognizedMagic(u32),

    #[error("Dictionnary not supported: id {id}")]
    DictNotSupported { id: usize },

    #[error("Frame header reserved bit must be 0")]
    InvalidReservedBit,

    #[error("Corrupted frame, checksum mismatch")]
    ChecksumMismatch,
}
use FrameError::*;

#[derive(Debug)]
pub enum Frame<'a> {
    ZstandardFrame(ZstandardFrame<'a>),
    SkippableFrame(SkippableFrame<'a>),
}

const STANDARD_MAGIC_NUMBER: u32 = 0xFD2FB528;
const SKIPPABLE_MAGIC_NUMBER: u32 = 0x184D2A5; // last 4bits: 0x0 to 0xF

#[derive(Debug)]
pub struct ZstandardFrame<'a> {
    frame_header: FrameHeader,
    blocks: Vec<Block<'a>>,
    checksum: Option<u32>,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct SkippableFrame<'a> {
    magic: u32,
    data: &'a [u8],
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct FrameHeader {
    window_size: usize,
    window_descriptor: u8,
    frame_content_size: usize,
    content_checksum_flag: bool,
}

impl<'a> Frame<'a> {
    pub fn parse(input: &mut ForwardByteParser<'a>) -> Result<Self> {
        let magic = input.le_u32()?;

        // Note: if more magic numbers to check use match case instead
        if magic == STANDARD_MAGIC_NUMBER {
            Ok(Self::ZstandardFrame(ZstandardFrame::parse(input)?))
        } else {
            if magic >> 4 == SKIPPABLE_MAGIC_NUMBER {
                let len = input.le_u32()?;
                let data = input.slice(len as usize)?;
                return Ok(Self::SkippableFrame(SkippableFrame { magic, data }));
            }
            Err(Error::Frame(UnrecognizedMagic(magic)))
        }
    }

    pub fn decode(self) -> Result<Vec<u8>> {
        match self {
            Frame::SkippableFrame(_) => Ok(Vec::new()),
            Frame::ZstandardFrame(mut frame) => {
                let mut context = DecodingContext::new(frame.frame_header.window_size)?;

                // hint: decode consume self, but we need to replace blocks, so that it does not borrow self
                // too soon and let us call frame.verify_checksum.
                // `take` let us replace frame.blocks with an empty vec.
                let blocks = std::mem::take(&mut frame.blocks);
                for block in blocks {
                    block.decode(&mut context)?;
                }

                if !frame.verify_checksum(&context.decoded)? {
                    return Err(Error::Frame(ChecksumMismatch));
                }

                Ok(context.decoded)
            }
        }
    }
}

impl<'a> ZstandardFrame<'a> {
    pub fn parse(input: &mut ForwardByteParser<'a>) -> Result<Self> {
        let frame_header = FrameHeader::parse(input)?;
        let mut blocks: Vec<Block> = Vec::new();

        loop {
            let (block, is_last) = Block::parse(input, frame_header.window_size)?;
            blocks.push(block);
            if is_last {
                break;
            }
        }

        let checksum = if frame_header.content_checksum_flag {
            Some(input.le_u32()?)
        } else {
            None
        };

        Ok(ZstandardFrame {
            frame_header,
            blocks,
            checksum,
        })
    }

    pub fn verify_checksum(&self, decoded: &[u8]) -> Result<bool> {
        if !self.frame_header.content_checksum_flag {
            return Ok(true);
        }

        let checksum = (xxh64(decoded, 0) & 0xFFFF_FFFF) as u32;
        let content_checksum = self.checksum.ok_or(ChecksumMismatch)?;

        Ok(checksum == content_checksum)
    }
}

impl FrameHeader {
    pub fn parse(input: &mut ForwardByteParser) -> Result<Self> {
        // Frame_Header_Descriptor 	    1 byte
        // [Window_Descriptor] 	        0-1 byte
        // [Dictionary_ID] 	            0-4 bytes
        // [Frame_Content_Size] 	    0-8 bytes
        let frame_header_descriptor = input.u8()?;

        let frame_content_size_flag = (frame_header_descriptor & 0b1100_0000) >> 6;
        let single_segment_flag = (frame_header_descriptor & 0b0010_0000) >> 5 == 1;
        let reserved_bit = (frame_header_descriptor & 0b0000_1000) >> 3;
        let content_checksum_flag = (frame_header_descriptor & 0b0000_0100) >> 2 == 1;
        let dictionary_id_flag = frame_header_descriptor & 0b0000_0011;
        let window_descriptor: u8 = if single_segment_flag { 0 } else { input.u8()? };

        if reserved_bit != 0 {
            return Err(Error::Frame(InvalidReservedBit));
        }

        // dictionnary is not implemented yet, but we still have to consume its bytes
        let dictionary_id = match dictionary_id_flag {
            0 => input.le(0)?,
            1 => input.le(1)?,
            2 => input.le(2)?,
            3 => input.le(4)?,
            _ => panic!("unexpected dictionary_id_flag {dictionary_id_flag}"),
        };
        if dictionary_id != 0 {
            return Err(Error::Frame(DictNotSupported { id: dictionary_id }));
        }

        let frame_content_size = match frame_content_size_flag {
            0 => input.le(usize::from(single_segment_flag))?,
            1 => input.le(2)? + 256,
            2 => input.le(4)?,
            3 => input.le(8)?,
            _ => panic!("unexpected frame_content_size_flag {frame_content_size_flag}"),
        };

        let mut window_size = frame_content_size;
        if !single_segment_flag {
            let exponent: usize = ((window_descriptor & 0b1111_1000) >> 3).into();
            let mantissa: usize = (window_descriptor & 0b0000_0111).into();

            let window_base = 1_usize << (10 + exponent);
            let window_add = (window_base / 8) * mantissa;
            window_size = window_base + window_add;
        }

        Ok(FrameHeader {
            window_size,
            window_descriptor,
            frame_content_size,
            content_checksum_flag,
        })
    }
}

pub struct FrameIterator<'a> {
    parser: ForwardByteParser<'a>,
}

impl<'a> FrameIterator<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            parser: ForwardByteParser::new(data),
        }
    }
}

impl<'a> Iterator for FrameIterator<'a> {
    type Item = Result<Frame<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.parser.is_empty() {
            return None;
        }
        Some(Frame::parse(&mut self.parser))
    }
}

#[cfg(test)]
mod tests {
    use super::{super::ParsingError, *};

    mod frame {
        use super::*;

        mod parse {
            use super::*;

            #[test]
            fn test_parse_empty() {
                let mut parser = ForwardByteParser::new(&[]);
                assert!(matches!(
                    Frame::parse(&mut parser),
                    Err(Error::Parsing(ParsingError::NotEnoughBytes {
                        requested: 4,
                        available: 0
                    }))
                ))
            }

            #[test]
            fn test_parse_skippable_frame() {
                let mut parser = ForwardByteParser::new(&[
                    // Skippable frame:
                    0x53, 0x2a, 0x4d, 0x18, // magic:   0x184d2a53
                    0x03, 0x00, 0x00, 0x00, // length:  3
                    0x10, 0x20, 0x30, // content: 0x10 0x20 0x30
                    0x40, // + extra byte
                ]);
                let Frame::SkippableFrame(skippable) = Frame::parse(&mut parser).unwrap() else {
                    panic!("unexpected frame type")
                };
                assert_eq!(skippable.magic, 0x184d2a53);
                assert_eq!(skippable.data, &[0x10, 0x20, 0x30]);
                assert_eq!(parser.len(), 1);
            }

            #[test]
            fn test_parse_truncated_skippable_frame() {
                let mut parser = ForwardByteParser::new(&[
                    // Skippable frame:
                    0x50, 0x2a, 0x4d, 0x18, // magic:   0x184d2a50
                    0x03, 0x00, 0x00, 0x00, // length:  3
                    0x10, 0x20, // content: 0x10 0x20
                ]);
                assert!(matches!(
                    Frame::parse(&mut parser),
                    Err(Error::Parsing(ParsingError::NotEnoughBytes {
                        requested: 3,
                        available: 2
                    }))
                ));
            }

            #[test]
            fn test_parse_magic_only_skippable_frame() {
                let mut parser = ForwardByteParser::new(&[
                    // Skippable frame:
                    0x50, 0x2a, 0x4d, 0x18, // magic:   0x184d2a50
                ]);
                assert!(matches!(
                    Frame::parse(&mut parser),
                    Err(Error::Parsing(ParsingError::NotEnoughBytes {
                        requested: 4,
                        available: 0
                    }))
                ));
            }

            #[test]
            fn test_parse_unknown_magic_number() {
                let mut parser = ForwardByteParser::new(&[
                    // Unknown frame: (similar to STANDARD_MAGIC_NUMBER with only last 4bits changing)
                    0x20, 0xB5, 0x2F, 0xFD, // magic:   0xFD2FB520
                ]);
                assert!(matches!(
                    Frame::parse(&mut parser),
                    Err(Error::Frame(FrameError::UnrecognizedMagic(0xFD2FB520)))
                ));
            }

            #[test]
            fn test_parse_standard_frame() {
                let mut parser = ForwardByteParser::new(&[
                    // Standard frame:
                    0x28, 0xB5, 0x2F, 0xFD, // magic:   0xFD2FB528
                    0x4, 0x0, // header + checksum flag
                    0x1, 0x0, 0x0, // block
                    0x12, 0x34, 0x56, 0x78, // checksum
                ]);
                let Frame::ZstandardFrame(standard) = Frame::parse(&mut parser).unwrap() else {
                    panic!("unexpected frame type")
                };
                assert_eq!(standard.checksum, Some(0x78563412));
            }
        }

        mod decode {
            use super::*;

            #[test]
            fn test_decode_skippable() {
                let frame = Frame::SkippableFrame(SkippableFrame {
                    magic: 0,
                    data: &[],
                });
                assert_eq!(frame.decode().unwrap(), Vec::new());
            }

            #[test]
            fn test_decode_standard() {
                let frame = Frame::ZstandardFrame(ZstandardFrame {
                    frame_header: FrameHeader {
                        window_size: 0,
                        window_descriptor: 0,
                        frame_content_size: 0,
                        content_checksum_flag: false,
                    },
                    blocks: vec![
                        Block::Rle {
                            byte: 0xAA,
                            repeat: 2,
                        },
                        Block::Raw(&[0xCA, 0xFE]),
                        Block::Rle {
                            byte: 0xBA,
                            repeat: 1,
                        },
                        Block::Raw(&[0xBE]),
                    ],
                    checksum: None,
                });
                assert_eq!(
                    frame.decode().unwrap(),
                    vec![0xAA, 0xAA, 0xCA, 0xFE, 0xBA, 0xBE]
                );
            }
        }
    }

    mod frame_header {
        use super::*;

        #[rustfmt::skip]
        mod parse {
            use super::*;

            #[test]
            fn test_decode_null_frame_header() {
                let mut parser = ForwardByteParser::new(&[0x0, 0xFF]);
                let frame_header = FrameHeader::parse(&mut parser).unwrap();
                assert_eq!(frame_header.content_checksum_flag, false);
                assert_eq!(frame_header.window_descriptor, 0xFF);
            }

            #[test]
            fn test_empty_frame_header() {
                let mut parser = ForwardByteParser::new(&[]);
                assert!(matches!(
                    FrameHeader::parse(&mut parser),
                    Err(Error::Parsing(ParsingError::NotEnoughBytes {
                        requested: 1,
                        available: 0
                    }))
                ))
            }

            #[test]
            fn test_parse_frame_header() {
                let mut parser = ForwardByteParser::new(&[
                    0b1010_0110,            // FCS 4bytes, no window descriptor, 2byte dict id, checksum
                    0x0, 0x0,               // dict id
                    0x10, 0x20, 0x30, 0x40, // FCS
                    0x42,                   // +extra byte
                ]);
                let frame_header = FrameHeader::parse(&mut parser).unwrap();
                assert_eq!(frame_header.content_checksum_flag, true);
                assert_eq!(frame_header.window_descriptor, 0);
                assert_eq!(frame_header.frame_content_size, 0x40_30_20_10);
                assert_eq!(parser.len(), 1);
            }

            #[test]
            fn test_parse_single_segment_flag_true() {
                let mut parser = ForwardByteParser::new(
                    &[
                        0b0010_0000, // SSF true
                        0xAD,        // FCS (SSF)
                        0x01,        // +extra byte
                    ],
                );
                let frame_header = FrameHeader::parse(&mut parser).unwrap();
                assert_eq!(frame_header.content_checksum_flag, false);
                assert_eq!(frame_header.window_descriptor, 0);
                assert_eq!(frame_header.frame_content_size, 0xAD);
                assert_eq!(parser.len(), 1);
            }

            #[test]
            fn test_parse_single_segment_flag_false() {
                let mut parser = ForwardByteParser::new(
                    &[
                        0b0000_0000, // SSF false
                        0xAD,        // window descriptor (SSF)
                        0x01,        // +extra byte
                    ],
                );
                let frame_header = FrameHeader::parse(&mut parser).unwrap();
                assert_eq!(frame_header.content_checksum_flag, false);
                assert_eq!(frame_header.window_descriptor, 0xAD);
                assert_eq!(frame_header.frame_content_size, 0);
                assert_eq!(parser.len(), 1);
            }
        }
    }

    mod frame_iterator {
        use core::panic;

        use super::*;

        #[test]
        fn test_iterator_empty() {
            let mut iterator = FrameIterator::new(&[]);
            assert!(iterator.next().is_none());
        }

        #[test]
        fn test_iterator() {
            let mut iterator = FrameIterator::new(&[
                // Skippable frame:
                0x53, 0x2a, 0x4d, 0x18, // magic:   0x184d2a53
                0x03, 0x00, 0x00, 0x00, // length:  3
                0x10, 0x20, 0x30, // content: 0x10 0x20 0x30
                // Standard frame:
                0x28, 0xB5, 0x2F, 0xFD, // magic:   0xFD2FB528
                0x4, 0x0, // header + checksum flag
                0x1, 0x0, 0x0, // block
                0x12, 0x34, 0x56, 0x78, // checksum
            ]);

            let Frame::SkippableFrame(frame) = iterator.next().unwrap().unwrap() else {
                panic!("unexpected frame type")
            };
            assert_eq!(frame.magic, 0x184d2a53);
            assert_eq!(frame.data, &[0x10, 0x20, 0x30]);

            let Frame::ZstandardFrame(frame) = iterator.next().unwrap().unwrap() else {
                panic!("unexpected frame type")
            };
            assert_eq!(frame.checksum, Some(0x78563412));

            assert!(iterator.next().is_none());
        }
    }
}

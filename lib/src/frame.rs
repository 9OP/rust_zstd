#![allow(dead_code)]

use super::block;
use super::parsing;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Frame parsing error: {0}")]
    ParsingError(#[from] parsing::Error),

    #[error(transparent)]
    BlockError(#[from] block::Error),

    #[error("Unrecognized magic number: {0}")]
    UnrecognizedMagic(u32),

    #[error("Invalid frame header")]
    InvalidFrameHeader,
}
use Error::*;
type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug)]
pub enum Frame<'a> {
    ZstandardFrame(ZstandardFrame<'a>),
    SkippableFrame(SkippableFrame<'a>),
}

const STANDARD_MAGIC_NUMBER: u32 = 0xFD2FB528;
const SKIPPABLE_MAGIC_NUMBER: u32 = 0x184D2A5; // last 4bits: 0x0 to 0xF

#[derive(Debug)]
pub struct SkippableFrame<'a> {
    magic: u32,
    data: &'a [u8],
}

#[derive(Debug)]
pub struct ZstandardFrame<'a> {
    frame_header: FrameHeader<'a>,
    blocks: Vec<block::Block<'a>>,
    checksum: Option<u32>,
}

#[derive(Debug)]
pub struct FrameHeader<'a> {
    frame_header_descriptor: u8,
    window_descriptor: u8,
    dictionary_id: &'a [u8],      // 0-4bytes
    frame_content_size: &'a [u8], // 0-8bytes
    content_checksum_flag: bool,
}

impl<'a> Frame<'a> {
    pub fn parse(input: &mut parsing::ForwardByteParser<'a>) -> Result<Self> {
        let magic = input.le_u32()?;
        match magic {
            STANDARD_MAGIC_NUMBER => Ok(Self::ZstandardFrame(ZstandardFrame::parse(input)?)),
            _ => {
                if magic >> 4 == SKIPPABLE_MAGIC_NUMBER {
                    let len = input.le_u32()? as usize;
                    let data = input.slice(len)?;
                    return Ok(Self::SkippableFrame(SkippableFrame { magic, data }));
                }
                Err(UnrecognizedMagic(magic))
            }
        }
    }

    pub fn decode(self) -> Vec<u8> {
        match self {
            Frame::SkippableFrame(_) => Vec::new(),
            Frame::ZstandardFrame(f) => f
                .blocks
                .into_iter()
                .flat_map(|block| block.decode())
                .collect(),
        }
    }
}

impl<'a> ZstandardFrame<'a> {
    pub fn parse(input: &mut parsing::ForwardByteParser<'a>) -> Result<Self> {
        let frame_header = FrameHeader::parse(input)?;
        let mut blocks: Vec<block::Block> = Vec::new();

        let mut last = false;
        while !last {
            let (block, is_last) = block::Block::parse(input)?;
            last = is_last;
            blocks.push(block);
        }

        let checksum = match frame_header.content_checksum_flag {
            true => Some(input.le_u32()?),
            false => None,
        };

        Ok(ZstandardFrame {
            frame_header,
            blocks,
            checksum,
        })
    }
}

impl<'a> FrameHeader<'a> {
    pub fn parse(input: &mut parsing::ForwardByteParser<'a>) -> Result<Self> {
        // Frame_Header_Descriptor 	    1 byte
        // [Window_Descriptor] 	        0-1 byte
        // [Dictionary_ID] 	            0-4 bytes
        // [Frame_Content_Size] 	    0-8 bytes
        let frame_header_descriptor = input.u8()?;

        // https://www.rfc-editor.org/rfc/rfc8878#section-3.1.1.1.1
        let frame_content_size_flag = (frame_header_descriptor & 0b1100_0000) >> 6;
        let single_segment_flag = (frame_header_descriptor & 0b0010_0000) >> 5 == 1;
        let content_checksum_flag = (frame_header_descriptor & 0b0000_0100) >> 2 == 1;
        let dictionary_id_flag = frame_header_descriptor & 0b0000_0011;

        // https://www.rfc-editor.org/rfc/rfc8878#section-3.1.1.1.1.2
        let window_descriptor: u8 = if single_segment_flag { 0 } else { input.u8()? };

        // https://www.rfc-editor.org/rfc/rfc8878#section-3.1.1.1.1.6
        let dictionary_id = match dictionary_id_flag {
            0 => input.slice(0)?,
            1 => input.slice(1)?,
            2 => input.slice(2)?,
            3 => input.slice(4)?,
            _ => return Err(InvalidFrameHeader),
        };

        // https://www.rfc-editor.org/rfc/rfc8878#section-3.1.1.1.1.1
        let frame_content_size = match frame_content_size_flag {
            0 => input.slice(if single_segment_flag { 1 } else { 0 })?,
            1 => input.slice(2)?,
            2 => input.slice(4)?,
            3 => input.slice(8)?,
            _ => return Err(InvalidFrameHeader),
        };

        Ok(FrameHeader {
            frame_header_descriptor,
            window_descriptor,
            dictionary_id,
            frame_content_size,
            content_checksum_flag,
        })
    }
}

pub struct FrameIterator<'a> {
    parser: parsing::ForwardByteParser<'a>,
}

impl<'a> FrameIterator<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            parser: parsing::ForwardByteParser::new(data),
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
    use super::*;

    #[test]
    fn test_parse_skippable_frame() {
        let mut parser = parsing::ForwardByteParser::new(&[
            // Skippable frame with magic 0x184d2a53, length 3, content 0x10 0x20 0x30
            // and an extra byte at the end.
            0x53, 0x2a, 0x4d, 0x18, 0x03, 0x00, 0x00, 0x00, 0x10, 0x20, 0x30, 0x40,
        ]);
        let Frame::SkippableFrame(skippable) = Frame::parse(&mut parser).unwrap() else {
            panic!("unexpected frame type")
        };
        assert_eq!(0x184d2a53, skippable.magic);
        assert_eq!(&[0x10, 0x20, 0x30], skippable.data);
        assert_eq!(1, parser.len());
    }

    #[test]
    fn test_parse_truncated_skippable_frame() {
        let mut parser = parsing::ForwardByteParser::new(&[
            0x50, 0x2a, 0x4d, 0x18, 0x03, 0x00, 0x00, 0x00, 0x10, 0x20,
        ]);
        assert!(matches!(
            Frame::parse(&mut parser),
            Err(ParsingError(parsing::Error::NotEnoughBytes {
                requested: 3,
                available: 2
            }))
        ));
    }

    #[test]
    fn test_error_on_unknown_frame() {
        let mut parser = parsing::ForwardByteParser::new(&[0x10, 0x20, 0x30, 0x40]);
        assert!(matches!(
            Frame::parse(&mut parser),
            Err(UnrecognizedMagic(0x40302010))
        ));
    }

    // #[test]
    // fn test_decode_frame() {
    //     todo!()
    // }

    // #[test]
    // fn test_iterate_frame_iterator() {
    //     todo!()
    // }

    #[test]
    fn test_no_frame_header() {
        let mut parser = parsing::ForwardByteParser::new(&[]);
        assert!(matches!(
            FrameHeader::parse(&mut parser),
            Err(ParsingError(parsing::Error::NotEnoughBytes {
                requested: 1,
                available: 0
            }))
        ))
    }

    #[test]
    fn test_parse_frame_header() {
        let mut parser = parsing::ForwardByteParser::new(
            // 4bytes FCS, no window descriptor, 2bytes dictionary id, checksum flag
            &[0xA6, 0xDE, 0xAD, 0x10, 0x20, 0x30, 0x40, 0x42],
        );
        let frame_header = FrameHeader::parse(&mut parser).unwrap();
        assert_eq!(frame_header.frame_header_descriptor, 0xA6);
        assert_eq!(frame_header.content_checksum_flag, true);
        assert_eq!(frame_header.window_descriptor, 0);
        assert_eq!(frame_header.dictionary_id, &[0xDE, 0xAD]);
        assert_eq!(frame_header.frame_content_size, &[0x10, 0x20, 0x30, 0x40]);
        assert_eq!(parser.len(), 1);
    }

    #[test]
    fn test_parse_single_segment_flag() {
        // SSF True
        let mut parser = parsing::ForwardByteParser::new(
            // 0bytes FCS, no window descriptor, no dictionary id, no checksum
            &[0x20, 0xAD, 0x01],
        );
        let frame_header = FrameHeader::parse(&mut parser).unwrap();
        assert_eq!(frame_header.frame_header_descriptor, 0x20);
        assert_eq!(frame_header.content_checksum_flag, false);
        assert_eq!(frame_header.window_descriptor, 0);
        assert_eq!(frame_header.dictionary_id.len(), 0);
        assert_eq!(frame_header.frame_content_size, &[0xAD]);
        assert_eq!(parser.len(), 1);

        // SSF False
        let mut parser = parsing::ForwardByteParser::new(
            // 0bytes FCS, window descriptor, no dictionary id, no checksum
            &[0x0, 0xAD, 0x01],
        );
        let frame_header = FrameHeader::parse(&mut parser).unwrap();
        assert_eq!(frame_header.frame_header_descriptor, 0x0);
        assert_eq!(frame_header.content_checksum_flag, false);
        assert_eq!(frame_header.window_descriptor, 0xAD);
        assert_eq!(frame_header.dictionary_id.len(), 0);
        assert_eq!(frame_header.frame_content_size.len(), 0);
        assert_eq!(parser.len(), 1);
    }
}

use crate::{
    decoders::{self},
    parsing::{self},
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Frame parsing error: {0}")]
    ParsingError(#[from] parsing::Error),

    #[error("Invalid reserved bits value")]
    InvalidDataError,
}
use Error::*;
pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug)]
pub struct SequenceSection {
    //
}

#[derive(Debug)]
pub struct SequenceSectionHeader {
    pub num_sequences: usize,
    pub literals_lengths_mode: CompressionMode,
    pub offsets_mode: CompressionMode,
    pub match_lengths_mode: CompressionMode,
}

#[derive(Debug)]
pub enum CompressionMode {
    Predefined = 0,
    RLE,
    FSECompressed,
    Repeat,
}

const LITERALS_LENGTH_DEFAULT_DISTRIBUTION: [i8; 36] = [
    4, 3, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 3, 2, 1, 1, 1, 1, 1,
    -1, -1, -1, -1,
];
const MATCH_LENGTH_DEFAULT_DISTRIBUTION: [i8; 53] = [
    1, 4, 3, 2, 2, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, -1, -1, -1, -1, -1, -1, -1,
];
const OFFSET_CODE_DEFAULT_DISTRIBUTION: [i8; 29] = [
    1, 1, 1, 1, 1, 1, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, -1, -1, -1, -1, -1,
];

impl CompressionMode {
    fn parse(mode: u8) -> Self {
        match mode {
            0 => Self::Predefined,
            1 => Self::RLE,
            2 => Self::FSECompressed,
            3 => Self::Repeat,
            _ => panic!("unexpected compression mode value {mode}"),
        }
    }
}

impl SequenceSectionHeader {
    pub fn parse(input: &mut parsing::ForwardByteParser) -> Result<Self> {
        let byte_0 = input.u8()? as usize;

        let num_sequences = match byte_0 {
            v if byte_0 < 128 => v,
            v if byte_0 < 255 => ((v - 0x80) << 8) + input.u8()? as usize,
            _ if byte_0 == 255 => input.u8()? as usize + ((input.u8()? as usize) << 8) + 0x7F00,
            _ => panic!("unexpected byte value {byte_0}"),
        };

        let modes = input.u8()?;

        let literals_lengths_mode = CompressionMode::parse((modes & 0b1100_0000) >> 6);
        let offsets_mode = CompressionMode::parse((modes & 0b0011_0000) >> 4);
        let match_lengths_mode = CompressionMode::parse((modes & 0b0000_1100) >> 2);

        let reserved = modes & 0b11;
        if reserved != 0 {
            return Err(InvalidDataError);
        }

        Ok(SequenceSectionHeader {
            num_sequences,
            literals_lengths_mode,
            offsets_mode,
            match_lengths_mode,
        })
    }
}

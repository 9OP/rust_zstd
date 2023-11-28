use crate::{
    decoders::{self, DecodingContext, FseTable},
    parsing::{self, ForwardBitParser, ForwardByteParser},
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Parsing error: {0}")]
    ParsingError(#[from] parsing::Error),

    #[error("Decoding error: {0}")]
    DecodingError(#[from] decoders::Error),

    #[error("Invalid reserved bits value")]
    InvalidDataError,
}
use Error::*;
pub type Result<T, E = Error> = std::result::Result<T, E>;

pub struct Sequences<'a> {
    pub number_of_sequences: usize,
    pub literal_lengths_mode: SymbolCompressionMode,
    pub offsets_mode: SymbolCompressionMode,
    pub match_lengths_mode: SymbolCompressionMode,
    pub bitstream: &'a [u8],
}

pub enum SymbolCompressionMode {
    PredefinedMode,
    RLEMode(u8),
    FseCompressedMode(FseTable),
    RepeatMode,
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

impl SymbolCompressionMode {
    pub fn parse(mode: u8, input: &mut ForwardByteParser) -> Result<Self> {
        match mode {
            0 => Ok(Self::PredefinedMode),
            1 => Ok(Self::RLEMode(input.u8()?)),
            2 => {
                // TODO: use RLE when only one symbol is present
                // TODO: there is magic for converting ByteParser->BitParser... implement from/into trait
                let bitstream = input.slice(input.len())?;
                let mut parser = ForwardBitParser::new(bitstream);
                let fse_table = FseTable::parse(&mut parser)?;
                *input = ForwardByteParser::new(&bitstream[(bitstream.len() - parser.len())..]);
                Ok(Self::FseCompressedMode(fse_table))
            }
            3 => Ok(Self::RepeatMode),
            _ => panic!("unexpected compression mode value {mode}"),
        }
    }
}

impl<'a> Sequences<'a> {
    /// Parse the sequences data from the stream
    /// TODO: &mut
    pub fn parse(input: &mut ForwardByteParser<'a>) -> Result<Self> {
        let byte_0 = input.u8()? as usize;

        let number_of_sequences = match byte_0 {
            v if byte_0 < 128 => v,
            v if byte_0 < 255 => ((v - 0x80) << 8) + input.u8()? as usize,
            _ if byte_0 == 255 => input.u8()? as usize + ((input.u8()? as usize) << 8) + 0x7F00,
            _ => panic!("unexpected byte value {byte_0}"),
        };

        let modes = input.u8()?;

        // Parse SymbolCompression mode in this order: [literal][offset][match]
        // Recall sequence layout: header,[literal_table],[offset_table],[match_table], bitstream
        let literal_lengths_mode = SymbolCompressionMode::parse((modes & 0b1100_0000) >> 6, input)?;
        let offsets_mode = SymbolCompressionMode::parse((modes & 0b0011_0000) >> 4, input)?;
        let match_lengths_mode = SymbolCompressionMode::parse((modes & 0b0000_1100) >> 2, input)?;

        let reserved = modes & 0b11;
        if reserved != 0 {
            return Err(InvalidDataError);
        }

        Ok(Sequences {
            number_of_sequences,
            literal_lengths_mode,
            offsets_mode,
            match_lengths_mode,
            bitstream: &[],
        })
    }

    /// Return vector of (literals length, offset value, match length) and update the
    /// decoding context with the tables if appropriate.
    pub fn decode(self, context: &mut DecodingContext) -> Result<Vec<(usize, usize, usize)>> {
        todo!()
    }
}

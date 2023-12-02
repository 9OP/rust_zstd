use core::panic;

use crate::{
    decoders::{
        self, BitDecoder, DecodingContext, FseDecoder, FseTable, RLEDecoder, SequenceDecoder,
        SymbolDecoder,
    },
    parsing::{self, BackwardBitParser, ForwardBitParser, ForwardByteParser},
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Parsing error: {0}")]
    ParsingError(#[from] parsing::Error),

    #[error("Decoding error: {0}")]
    DecodingError(#[from] decoders::Error),

    #[error("Invalid reserved bits value")]
    InvalidDataError,

    #[error("Missing sequence decoder")]
    MissingSequenceDecoder,
}
use Error::*;
pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug)]
pub struct Sequences<'a> {
    pub number_of_sequences: usize,
    pub literal_lengths_mode: SymbolCompressionMode,
    pub offsets_mode: SymbolCompressionMode,
    pub match_lengths_mode: SymbolCompressionMode,
    pub bitstream: &'a [u8],
}

#[derive(Debug)]
pub enum SymbolCompressionMode {
    PredefinedMode,
    RLEMode(u8),
    FseCompressedMode(FseTable),
    RepeatMode,
}
use SymbolCompressionMode::*;

const LITERALS_LENGTH_DEFAULT_DISTRIBUTION: (u8, [i16; 36]) = (
    6,
    [
        4, 3, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 3, 2, 1, 1, 1,
        1, 1, -1, -1, -1, -1,
    ],
);
const MATCH_LENGTH_DEFAULT_DISTRIBUTION: (u8, [i16; 53]) = (
    6,
    [
        1, 4, 3, 2, 2, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, -1, -1, -1, -1, -1, -1, -1,
    ],
);
const OFFSET_CODE_DEFAULT_DISTRIBUTION: (u8, [i16; 29]) = (
    5,
    [
        1, 1, 1, 1, 1, 1, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, -1, -1, -1, -1, -1,
    ],
);

impl SymbolCompressionMode {
    pub fn parse(mode: u8, input: &mut ForwardByteParser) -> Result<Self> {
        match mode {
            0 => Ok(Self::PredefinedMode),
            1 => Ok(Self::RLEMode(input.u8()?)),
            2 => {
                let mut parser = ForwardBitParser::from(*input);
                let fse_table = FseTable::parse(&mut parser)?;
                if fse_table.states.len() == 1 {
                    return Ok(Self::PredefinedMode);
                }
                *input = ForwardByteParser::from(parser);
                Ok(Self::FseCompressedMode(fse_table))
            }
            3 => Ok(Self::RepeatMode),
            _ => panic!("unexpected compression mode value {mode}"),
        }
    }
}

impl<'a> Sequences<'a> {
    /// Parse the sequences data from the stream
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

        let bitstream = <&[u8]>::from(*input);

        Ok(Sequences {
            number_of_sequences,
            literal_lengths_mode,
            offsets_mode,
            match_lengths_mode,
            bitstream,
        })
    }

    // TODO: factorize RLEmode, FSECompressedMode parsing and move it to SymbolCompressionMode instead
    fn parse_literals_lengths_decoder(
        &self,
        parser: &mut BackwardBitParser,
    ) -> Result<Option<Box<SymbolDecoder>>> {
        let decoder = match &self.literal_lengths_mode {
            PredefinedMode => {
                let (acc_log, distribution) = LITERALS_LENGTH_DEFAULT_DISTRIBUTION;
                let fse_table = FseTable::from_distribution(acc_log, &distribution);
                let mut fse_decoder = FseDecoder::new(fse_table);
                fse_decoder.initialize(parser)?;
                Some(Box::new(fse_decoder) as Box<SymbolDecoder>)
            }
            RLEMode(byte) => {
                let rle_decoder = RLEDecoder {
                    symbol: *byte as u16,
                };
                Some(Box::new(rle_decoder) as Box<SymbolDecoder>)
            }
            FseCompressedMode(fse_table) => {
                let mut fse_decoder = FseDecoder::new(fse_table.clone());
                fse_decoder.initialize(parser)?;
                Some(Box::new(fse_decoder) as Box<SymbolDecoder>)
            }
            RepeatMode => None,
        };

        Ok(decoder)
    }

    fn parse_offsets_decoder(
        &self,
        parser: &mut BackwardBitParser,
    ) -> Result<Option<Box<SymbolDecoder>>> {
        let decoder = match &self.offsets_mode {
            PredefinedMode => {
                let (acc_log, distribution) = OFFSET_CODE_DEFAULT_DISTRIBUTION;
                let fse_table = FseTable::from_distribution(acc_log, &distribution);
                let mut fse_decoder = FseDecoder::new(fse_table);
                fse_decoder.initialize(parser)?;
                Some(Box::new(fse_decoder) as Box<SymbolDecoder>)
            }
            RLEMode(byte) => {
                let rle_decoder = RLEDecoder {
                    symbol: *byte as u16,
                };
                Some(Box::new(rle_decoder) as Box<SymbolDecoder>)
            }
            FseCompressedMode(fse_table) => {
                let mut fse_decoder = FseDecoder::new(fse_table.clone());
                fse_decoder.initialize(parser)?;
                Some(Box::new(fse_decoder) as Box<SymbolDecoder>)
            }
            RepeatMode => None,
        };

        Ok(decoder)
    }

    fn parse_match_lengths_decoder(
        &self,
        parser: &mut BackwardBitParser,
    ) -> Result<Option<Box<SymbolDecoder>>> {
        let decoder = match &self.match_lengths_mode {
            PredefinedMode => {
                let (acc_log, distribution) = MATCH_LENGTH_DEFAULT_DISTRIBUTION;
                let fse_table = FseTable::from_distribution(acc_log, &distribution);
                let mut fse_decoder = FseDecoder::new(fse_table);
                fse_decoder.initialize(parser)?;
                Some(Box::new(fse_decoder) as Box<SymbolDecoder>)
            }
            RLEMode(byte) => {
                let rle_decoder = RLEDecoder {
                    symbol: *byte as u16,
                };
                Some(Box::new(rle_decoder) as Box<SymbolDecoder>)
            }
            FseCompressedMode(fse_table) => {
                let mut fse_decoder = FseDecoder::new(fse_table.clone());
                fse_decoder.initialize(parser)?;
                Some(Box::new(fse_decoder) as Box<SymbolDecoder>)
            }
            RepeatMode => None,
        };

        Ok(decoder)
    }

    /// Parse the symbol decoders and update the context
    fn parse_symbol_decoders(
        &self,
        parser: &mut BackwardBitParser,
        context: &mut DecodingContext,
    ) -> Result<()> {
        // initialize order: literals > offsets > match
        if let Some(decoder) = self.parse_literals_lengths_decoder(parser)? {
            context.literals_lengths_decoder = Some(decoder);
        }
        if let Some(decoder) = self.parse_offsets_decoder(parser)? {
            context.offsets_decoder = Some(decoder);
        }
        if let Some(decoder) = self.parse_match_lengths_decoder(parser)? {
            context.match_lengths_decoder = Some(decoder);
        }

        Ok(())
    }

    /// Extract the symbol decoders from the context and return a SequenceDecoder instance.
    /// Return `MissingSequenceDecoder` when a symbol decoder is `None`.
    fn get_sequence_decoder(context: &mut DecodingContext) -> Result<SequenceDecoder> {
        let literals_lengths_decoder = context
            .literals_lengths_decoder
            .as_mut()
            .ok_or(MissingSequenceDecoder)?;
        let offsets_decoder = context
            .offsets_decoder
            .as_mut()
            .ok_or(MissingSequenceDecoder)?;
        let match_lengths_decoder = context
            .match_lengths_decoder
            .as_mut()
            .ok_or(MissingSequenceDecoder)?;

        Ok(SequenceDecoder::new(
            literals_lengths_decoder,
            offsets_decoder,
            match_lengths_decoder,
        ))
    }

    /// Return vector of (literals length, offset value, match length) and update the
    /// decoding context with the tables if appropriate.
    pub fn decode(self, context: &mut DecodingContext) -> Result<Vec<(usize, usize, usize)>> {
        let mut decoded_sequences = Vec::<(usize, usize, usize)>::new();
        let mut parser = BackwardBitParser::new(self.bitstream)?;

        self.parse_symbol_decoders(&mut parser, context)?;
        let mut sequence_decoder = Self::get_sequence_decoder(context)?;

        for i in 0..self.number_of_sequences {
            // decode order: offset > match > literals
            let (literals_symbol, offset_symbol, match_symbol) = sequence_decoder.symbol();

            // offset
            let offset_code =
                ((1 as u64) << offset_symbol) + parser.take(offset_symbol as usize)?;

            // TODO: check the offset_symbol greater bound
            // if of_code >= 32 {
            //     return Err(DecodeSequenceError::UnsupportedOffset {
            //         offset_code: of_code,
            //     });
            // }

            // match
            let (match_value, match_num_bits) = match_lengths_code_lookup(match_symbol);
            let match_code = match_value + parser.take(match_num_bits)? as usize;

            // literals
            let (literals_value, literals_num_bits) = literals_lengths_code_lookup(literals_symbol);
            let literals_code = literals_value + parser.take(literals_num_bits)? as usize;

            decoded_sequences.push((literals_code, offset_code as usize, match_code));

            // update bits if it is not the last sequence
            if i != self.number_of_sequences - 1 {
                sequence_decoder.update_bits(&mut parser)?;
            }
        }

        Ok(decoded_sequences)
    }
}

fn literals_lengths_code_lookup(symbol: u16) -> (usize, usize) {
    match symbol {
        0..=15 => (symbol as usize, 0),
        16 => (16, 1),
        17 => (18, 1),
        18 => (20, 1),
        19 => (22, 1),
        20 => (24, 2),
        21 => (28, 2),
        22 => (32, 3),
        23 => (40, 3),
        24 => (48, 4),
        25 => (64, 6),
        26 => (128, 7),
        27 => (256, 8),
        28 => (512, 9),
        29 => (1024, 10),
        30 => (2048, 11),
        31 => (4096, 12),
        32 => (8192, 13),
        33 => (16384, 14),
        34 => (32768, 15),
        35 => (65536, 16),
        _ => panic!("unexpected symbol value {symbol}"), // TODO: return error instead SymbolOutOfRange
    }
}

fn match_lengths_code_lookup(symbol: u16) -> (usize, usize) {
    match symbol {
        0..=31 => (symbol as usize + 3, 0),
        32 => (35, 1),
        33 => (37, 1),
        34 => (39, 1),
        35 => (41, 1),
        36 => (43, 2),
        37 => (47, 2),
        38 => (51, 3),
        39 => (59, 3),
        40 => (67, 4),
        41 => (83, 4),
        42 => (99, 5),
        43 => (131, 7),
        44 => (259, 8),
        45 => (515, 9),
        46 => (1027, 10),
        47 => (2051, 11),
        48 => (4099, 12),
        49 => (8195, 13),
        50 => (16387, 14),
        51 => (32771, 15),
        52 => (65539, 16),
        _ => panic!("unexpected symbol value {symbol}"), // TODO: return error instead SymbolOutOfRange
    }
}

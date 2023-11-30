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
            bitstream: input.slice(input.len())?, // TODO: create a function to get the bitstream
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
                let mut rle_decoder = RLEDecoder {
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
                let mut rle_decoder = RLEDecoder {
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

    /// Return vector of (literals length, offset value, match length) and update the
    /// decoding context with the tables if appropriate.
    pub fn decode(self, context: &mut DecodingContext) -> Result<Vec<(usize, usize, usize)>> {
        let mut decoded_sequences = Vec::<(usize, usize, usize)>::new();
        let mut parser = BackwardBitParser::new(self.bitstream)?;

        // TODO: move to function parse_decoders()
        // initialize order: literals > offsets > match
        if let Some(decoder) = self.parse_literals_lengths_decoder(&mut parser)? {
            context.literals_lengths_decoder = Some(decoder);
        }
        if let Some(decoder) = self.parse_offsets_decoder(&mut parser)? {
            context.offsets_decoder = Some(decoder);
        }
        if let Some(decoder) = self.parse_match_lengths_decoder(&mut parser)? {
            context.match_lengths_decoder = Some(decoder);
        }

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

        let mut sequence_decoder = SequenceDecoder::new(
            literals_lengths_decoder,
            offsets_decoder,
            match_lengths_decoder,
        );

        let mut stop = false;
        loop {
            // seuqnce decoder symbol, then convert to code
            // decode order:
            // the offset
            // the match length
            // the literals length
            let (literals_symbol, offset_symbol, match_symbol) = sequence_decoder.symbol();

            decoded_sequences.push((
                literals_symbol as usize,
                offset_symbol as usize,
                match_symbol as usize,
            ));

            if stop {
                break;
            }
            stop |= sequence_decoder.update_bits(&mut parser)?;
        }

        Ok(decoded_sequences)
    }
}

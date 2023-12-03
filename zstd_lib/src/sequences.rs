use super::{
    BackwardBitParser, BitDecoder, DecodingContext, Error, ForwardBitParser, ForwardByteParser,
    FseDecoder, FseTable, RLEDecoder, Result, SequenceDecoder, SymbolDecoder,
};

#[derive(Debug, thiserror::Error)]
pub enum SequencesError {
    #[error("Invalid reserved bits value")]
    InvalidDataError,

    #[error("Missing sequence decoder")]
    MissingSequenceDecoder,

    #[error("Symbol code unknown")]
    SymbolCodeUnknown,

    #[error("FSE AL is too large")]
    ALTooLarge,
}
use SequencesError::*;

#[derive(Debug)]
pub struct Sequences<'a> {
    number_of_sequences: usize,
    literal_lengths_mode: SymbolCompressionMode,
    offsets_mode: SymbolCompressionMode,
    match_lengths_mode: SymbolCompressionMode,
    bitstream: &'a [u8],
}

pub struct SequenceCommand {
    pub literal_length: usize,
    pub match_length: usize,
    pub offset: usize,
}

#[derive(Debug)]
enum SymbolCompressionMode {
    Predefined,
    Rle(u8),
    FseCompressed(FseTable),
    Repeat,
}

#[derive(Debug)]
enum SymbolType {
    LiteralsLengths,
    MatchLength,
    Offset,
}

struct DefaultDistribution<'a> {
    accuracy_log: u8,
    distribution: &'a [i16],
}

const LITERALS_LENGTH_DEFAULT_DISTRIBUTION: DefaultDistribution<'_> = DefaultDistribution {
    accuracy_log: 6,
    distribution: &[
        4, 3, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 3, 2, 1, 1, 1,
        1, 1, -1, -1, -1, -1,
    ],
};
const MATCH_LENGTH_DEFAULT_DISTRIBUTION: DefaultDistribution<'_> = DefaultDistribution {
    accuracy_log: 6,
    distribution: &[
        1, 4, 3, 2, 2, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, -1, -1, -1, -1, -1, -1, -1,
    ],
};
const OFFSET_CODE_DEFAULT_DISTRIBUTION: DefaultDistribution<'_> = DefaultDistribution {
    accuracy_log: 5,
    distribution: &[
        1, 1, 1, 1, 1, 1, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, -1, -1, -1, -1, -1,
    ],
};

impl SymbolCompressionMode {
    /// Parse the compression mode
    fn parse(mode: u8, input: &mut ForwardByteParser, symbol_type: SymbolType) -> Result<Self> {
        match mode {
            0 => Ok(Self::Predefined),
            1 => Ok(Self::Rle(input.u8()?)),
            2 => {
                let mut parser = ForwardBitParser::from(*input);
                let fse_table = FseTable::parse(&mut parser)?;

                let max_al = match symbol_type {
                    SymbolType::MatchLength | SymbolType::LiteralsLengths => 9,
                    SymbolType::Offset => 8,
                };
                if fse_table.accuracy_log() > max_al {
                    return Err(Error::Sequences(ALTooLarge));
                }

                // not sure, see: https://www.rfc-editor.org/rfc/rfc8878#name-sequences_section_header
                if fse_table.accuracy_log() == 0 {
                    return Ok(Self::Predefined);
                }

                *input = ForwardByteParser::from(parser);
                Ok(Self::FseCompressed(fse_table))
            }
            3 => Ok(Self::Repeat),
            _ => panic!("unexpected compression mode value {mode}"),
        }
    }

    /// Parse the compression mode respective decoder
    fn parse_symbol_decoder(
        &self,
        symbol_type: SymbolType,
        parser: &mut BackwardBitParser,
    ) -> Result<Option<Box<SymbolDecoder>>> {
        let decoder = match &self {
            SymbolCompressionMode::Predefined => {
                let def = match symbol_type {
                    SymbolType::LiteralsLengths => LITERALS_LENGTH_DEFAULT_DISTRIBUTION,
                    SymbolType::MatchLength => MATCH_LENGTH_DEFAULT_DISTRIBUTION,
                    SymbolType::Offset => OFFSET_CODE_DEFAULT_DISTRIBUTION,
                };

                let fse_table = FseTable::from_distribution(def.accuracy_log, def.distribution)?;
                let mut fse_decoder = FseDecoder::new(fse_table);
                fse_decoder.initialize(parser)?;
                Some(Box::new(fse_decoder) as Box<SymbolDecoder>)
            }
            SymbolCompressionMode::Rle(byte) => {
                let rle_decoder = RLEDecoder::new(*byte as u16);
                Some(Box::new(rle_decoder) as Box<SymbolDecoder>)
            }
            SymbolCompressionMode::FseCompressed(fse_table) => {
                let mut fse_decoder = FseDecoder::new(fse_table.clone());
                fse_decoder.initialize(parser)?;
                Some(Box::new(fse_decoder) as Box<SymbolDecoder>)
            }
            SymbolCompressionMode::Repeat => None,
        };

        Ok(decoder)
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

        // Parse order: [literal][offset][match]
        let literal_lengths_mode = SymbolCompressionMode::parse(
            (modes & 0b1100_0000) >> 6,
            input,
            SymbolType::LiteralsLengths,
        )?;
        let offsets_mode =
            SymbolCompressionMode::parse((modes & 0b0011_0000) >> 4, input, SymbolType::Offset)?;
        let match_lengths_mode = SymbolCompressionMode::parse(
            (modes & 0b0000_1100) >> 2,
            input,
            SymbolType::MatchLength,
        )?;

        let reserved = modes & 0b11;
        if reserved != 0 {
            return Err(Error::Sequences(InvalidDataError));
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

    /// Parse the symbol decoders and update the context
    fn parse_sequence_decoder(
        &'a self,
        parser: &mut BackwardBitParser,
        context: &'a mut DecodingContext,
    ) -> Result<SequenceDecoder<'_>> {
        // initialize order: literals > offsets > match
        let ll_decoder = self
            .literal_lengths_mode
            .parse_symbol_decoder(SymbolType::LiteralsLengths, parser)?;

        let of_decoder = self
            .offsets_mode
            .parse_symbol_decoder(SymbolType::Offset, parser)?;

        let ml_decoder = self
            .match_lengths_mode
            .parse_symbol_decoder(SymbolType::MatchLength, parser)?;

        // TODO: move the code below in a function in decoding_context
        // Update the context decoders or reset them
        if ll_decoder.is_some() {
            context.literals_lengths_decoder = ll_decoder;
        } else {
            context
                .literals_lengths_decoder
                .as_mut()
                .ok_or(MissingSequenceDecoder)?
                .reset();
        }

        if of_decoder.is_some() {
            context.offsets_decoder = of_decoder;
        } else {
            context
                .offsets_decoder
                .as_mut()
                .ok_or(MissingSequenceDecoder)?
                .reset();
        }

        if ml_decoder.is_some() {
            context.match_lengths_decoder = ml_decoder;
        } else {
            context
                .match_lengths_decoder
                .as_mut()
                .ok_or(MissingSequenceDecoder)?
                .reset();
        }

        Ok(SequenceDecoder::new(
            context.literals_lengths_decoder.as_mut().unwrap(),
            context.offsets_decoder.as_mut().unwrap(),
            context.match_lengths_decoder.as_mut().unwrap(),
        ))
    }

    /// Return vector of (literals length, offset value, match length) and update the
    /// decoding context with the tables if appropriate.
    pub fn decode(self, context: &mut DecodingContext) -> Result<Vec<SequenceCommand>> {
        let mut decoded_sequences = Vec::<SequenceCommand>::new();
        let mut parser = BackwardBitParser::new(self.bitstream)?;

        let mut sequence_decoder = self.parse_sequence_decoder(&mut parser, context)?;

        for i in 0..self.number_of_sequences {
            // decode order: offset > match > literals
            let (literals_symbol, offset_symbol, match_symbol) = sequence_decoder.symbol();

            if offset_symbol > 31 {
                // >31: from reference implementation
                return Err(Error::Sequences(SymbolCodeUnknown));
            }

            // offset
            let offset_code = (1_u64 << offset_symbol) + parser.take(offset_symbol.into())?;

            // match
            let (value, num_bits) = match_lengths_code_lookup(match_symbol)?;
            let match_code = value + parser.take(num_bits)? as usize;

            // literals
            let (value, num_bits) = literals_lengths_code_lookup(literals_symbol)?;
            let literals_code = value + parser.take(num_bits)? as usize;

            decoded_sequences.push(SequenceCommand {
                literal_length: literals_code,
                match_length: match_code,
                offset: offset_code as usize,
            });

            // update bits if it is not the last sequence
            if i != self.number_of_sequences - 1 {
                sequence_decoder.update_bits(&mut parser)?;
            }
            // sequence_decoder.update_bits(&mut parser)?;
        }

        Ok(decoded_sequences)
    }
}

fn literals_lengths_code_lookup(symbol: u16) -> Result<(usize, usize)> {
    let lookup = match symbol {
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
        _ => return Err(Error::Sequences(SymbolCodeUnknown)),
    };
    Ok(lookup)
}

fn match_lengths_code_lookup(symbol: u16) -> Result<(usize, usize)> {
    let lookup = match symbol {
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
        _ => return Err(Error::Sequences(SymbolCodeUnknown)),
    };
    Ok(lookup)
}

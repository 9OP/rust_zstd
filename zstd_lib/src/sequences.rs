use super::{
    BackwardBitParser, BitDecoder, DecodingContext, Error, ForwardBitParser, ForwardByteParser,
    FseDecoder, FseTable, RLEDecoder, Result, SequenceDecoder, SymbolDecoder,
};

#[derive(Debug, thiserror::Error)]
pub enum SequencesError {
    #[error("Invalid reserved bits value")]
    InvalidDataError,

    #[error("Missing decoder: {0:?}")]
    MissingDecoder(SymbolType),

    #[error("Symbol code unknown")]
    SymbolCodeUnknown,

    #[error("FSE AL is too large")]
    ALTooLarge,
}
use SequencesError::*;

#[derive(Debug)]
pub struct Sequences<'a> {
    number_of_sequences: usize,
    literal_lengths_mode: SymbolCompressor,
    offsets_mode: SymbolCompressor,
    match_lengths_mode: SymbolCompressor,
    bitstream: &'a [u8],
}

#[derive(Debug)]
struct SymbolCompressor {
    compression_mode: CompressionMode,
    symbol_type: SymbolType,
}

#[derive(Debug)]
enum CompressionMode {
    Predefined,
    Rle(u8),
    FseCompressed(FseTable),
    Repeat,
}
use CompressionMode::*;

#[derive(Debug)]
pub enum SymbolType {
    LiteralsLength,
    MatchLength,
    Offset,
}
use SymbolType::*;

#[derive(Debug)]
pub struct SequenceCommand {
    pub literal_length: usize,
    pub match_length: usize,
    pub offset: usize,
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

impl SymbolCompressor {
    /// Parse the compression mode
    fn parse(mode: u8, symbol_type: SymbolType, input: &mut ForwardByteParser) -> Result<Self> {
        let compression_mode = match mode {
            0 => Predefined,
            1 => Rle(input.u8()?),
            2 => {
                let mut parser = ForwardBitParser::from(*input);
                let fse_table = FseTable::parse(&mut parser)?;
                *input = ForwardByteParser::from(parser);

                // Not sure about this part, from the doc:
                //      Note that the maximum allowed accuracy log for literals length code and match length code tables is 9,
                //      and the maximum accuracy log for the offset code table is 8.
                //      This mode must not be used when only one symbol is present;
                //      RLE_Mode should be used instead (although any other mode will work).
                let max_al = match symbol_type {
                    MatchLength | LiteralsLength => 9,
                    Offset => 8,
                };
                if fse_table.accuracy_log() > max_al {
                    return Err(Error::Sequences(ALTooLarge));
                }

                if fse_table.accuracy_log() == 0 {
                    Predefined
                } else {
                    FseCompressed(fse_table)
                }
            }
            3 => Repeat,
            _ => panic!("unexpected compression mode value {mode}"),
        };

        Ok(Self {
            compression_mode,
            symbol_type,
        })
    }

    /// Parse the compression mode respective decoder
    fn parse_decoder(
        &self,
        context: &mut DecodingContext,
        parser: &mut BackwardBitParser,
    ) -> Result<Box<SymbolDecoder>> {
        let decoder = match &self.compression_mode {
            Predefined => {
                let DefaultDistribution {
                    accuracy_log,
                    distribution,
                } = match &self.symbol_type {
                    LiteralsLength => LITERALS_LENGTH_DEFAULT_DISTRIBUTION,
                    MatchLength => MATCH_LENGTH_DEFAULT_DISTRIBUTION,
                    Offset => OFFSET_CODE_DEFAULT_DISTRIBUTION,
                };

                let fse_table = FseTable::from_distribution(accuracy_log, distribution)?;
                let mut fse_decoder = FseDecoder::new(fse_table);
                fse_decoder.initialize(parser)?;
                Box::new(fse_decoder) as Box<SymbolDecoder>
            }
            Rle(byte) => {
                let rle_decoder = RLEDecoder::new(*byte as u16);
                Box::new(rle_decoder) as Box<SymbolDecoder>
            }
            FseCompressed(fse_table) => {
                let mut fse_decoder = FseDecoder::new(fse_table.clone());
                fse_decoder.initialize(parser)?;
                Box::new(fse_decoder) as Box<SymbolDecoder>
            }
            Repeat => {
                let mut repeat_decoder = match &self.symbol_type {
                    LiteralsLength => context
                        .literals_lengths_decoder
                        .take()
                        .ok_or(MissingDecoder(LiteralsLength))?,
                    MatchLength => context
                        .match_lengths_decoder
                        .take()
                        .ok_or(MissingDecoder(MatchLength))?,
                    Offset => context
                        .offsets_decoder
                        .take()
                        .ok_or(MissingDecoder(Offset))?,
                };

                repeat_decoder.reset();
                repeat_decoder.initialize(parser)?;
                // repeat_decoder.debug();
                repeat_decoder
            }
        };

        Ok(decoder)
    }
}

impl<'a> Sequences<'a> {
    fn parse_number_of_sequences(input: &mut ForwardByteParser) -> Result<usize> {
        let byte_0 = input.u8()? as usize;

        let number_of_sequences = match byte_0 {
            v if byte_0 < 128 => v,
            v if byte_0 < 255 => ((v - 0x80) << 8) + input.u8()? as usize,
            _ if byte_0 == 255 => input.u8()? as usize + ((input.u8()? as usize) << 8) + 0x7F00,
            _ => panic!("unexpected byte value {byte_0}"),
        };

        Ok(number_of_sequences)
    }

    fn parse_compression_modes(
        input: &mut ForwardByteParser,
    ) -> Result<(SymbolCompressor, SymbolCompressor, SymbolCompressor)> {
        let modes = input.u8()?;

        let ll_mode = (modes & 0b1100_0000) >> 6;
        let of_mode = (modes & 0b0011_0000) >> 4;
        let ml_mode = (modes & 0b0000_1100) >> 2;

        // Parse order: [literal][offset][match]
        let ll = SymbolCompressor::parse(ll_mode, LiteralsLength, input)?;
        let of = SymbolCompressor::parse(of_mode, Offset, input)?;
        let ml = SymbolCompressor::parse(ml_mode, MatchLength, input)?;

        let reserved = modes & 0b11;
        if reserved != 0 {
            return Err(Error::Sequences(InvalidDataError));
        }

        Ok((ll, of, ml))
    }

    /// Parse the sequences data from the stream
    pub fn parse(input: &mut ForwardByteParser<'a>) -> Result<Self> {
        let number_of_sequences = Self::parse_number_of_sequences(input)?;
        if number_of_sequences == 0 {
            return Ok(Sequences {
                number_of_sequences: 0,
                literal_lengths_mode: SymbolCompressor {
                    compression_mode: Predefined,
                    symbol_type: LiteralsLength,
                },
                offsets_mode: SymbolCompressor {
                    compression_mode: Predefined,
                    symbol_type: Offset,
                },
                match_lengths_mode: SymbolCompressor {
                    compression_mode: Predefined,
                    symbol_type: MatchLength,
                },
                bitstream: &[],
            });
        }

        let (ll, of, ml) = Self::parse_compression_modes(input)?;

        let bitstream = <&[u8]>::from(*input);

        Ok(Sequences {
            number_of_sequences,
            literal_lengths_mode: ll,
            offsets_mode: of,
            match_lengths_mode: ml,
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
        let ll_decoder = self.literal_lengths_mode.parse_decoder(context, parser)?;
        let of_decoder = self.offsets_mode.parse_decoder(context, parser)?;
        let ml_decoder = self.match_lengths_mode.parse_decoder(context, parser)?;

        context.literals_lengths_decoder = Some(ll_decoder);
        context.offsets_decoder = Some(of_decoder);
        context.match_lengths_decoder = Some(ml_decoder);

        Ok(context.get_sequence_decoder()?)
    }

    fn decode_sequence(
        decoder: &mut SequenceDecoder,
        input: &mut BackwardBitParser,
        is_last: bool,
        _i: usize,
    ) -> Result<SequenceCommand> {
        // decode order: offset > match > literals
        let (literals_symbol, offset_symbol, match_symbol) = decoder.symbol();
        // println!("{literals_symbol} {offset_symbol} {match_symbol} {_i}");

        if offset_symbol > 31 {
            // >31: from reference implementation
            return Err(Error::Sequences(SymbolCodeUnknown));
        }

        // offset
        let offset_code = (1_u64 << offset_symbol) + input.take(offset_symbol.into())?;

        // match
        let (value, num_bits) = match_lengths_code_lookup(match_symbol)?;
        let match_code = value + input.take(num_bits)? as usize;

        // literals
        let (value, num_bits) = literals_lengths_code_lookup(literals_symbol)?;
        let literals_code = value + input.take(num_bits)? as usize;

        // update bits if it is not the last sequence
        if !is_last {
            decoder.update_bits(input)?;
        }

        Ok(SequenceCommand {
            literal_length: literals_code,
            match_length: match_code,
            offset: offset_code as usize,
        })
        // sequence_decoder.update_bits(&mut parser)?;
    }

    /// Return vector of (literals length, offset value, match length) and update the
    /// decoding context with the tables if appropriate.
    pub fn decode(self, context: &mut DecodingContext) -> Result<Vec<SequenceCommand>> {
        if self.number_of_sequences == 0 {
            return Ok(vec![]);
        }

        let mut decoded_sequences = Vec::<SequenceCommand>::new();
        let mut parser = BackwardBitParser::new(self.bitstream)?;
        let mut sequence_decoder = self.parse_sequence_decoder(&mut parser, context)?;

        for i in 0..self.number_of_sequences {
            let is_last = i == self.number_of_sequences - 1;
            let command = Self::decode_sequence(&mut sequence_decoder, &mut parser, is_last, i)?;
            decoded_sequences.push(command);
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

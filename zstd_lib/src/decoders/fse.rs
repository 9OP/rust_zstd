use std::collections::HashSet;

use super::{BackwardBitParser, BitDecoder, Error, ForwardBitParser, Result};

#[derive(Debug, thiserror::Error)]
pub enum FseError {
    #[error("Missing FSE state")]
    MissingState,

    #[error("FSE accuracy log: {log} greater than allowed maximum: {max}")]
    ALTooLarge { log: u8, max: u8 },

    #[error("FSE distribution is corrupted")]
    DistributionCorrupted,
}
use FseError::*;

#[derive(Debug, Clone)]
pub struct FseTable {
    states: Vec<FseState>,
}

type Symbol = u16;
type Probability = i16;

#[derive(Debug, Default, Clone, Copy)]
pub struct FseState {
    symbol: Symbol,
    base_line: usize,
    num_bits: usize,
}

const ACC_LOG_OFFSET: u8 = 5;
const ACC_LOG_MAX: u8 = 9;

impl FseTable {
    pub fn accuracy_log(&self) -> u32 {
        // by design: 1 << AL == states.len()
        assert!(
            self.states.len().is_power_of_two(),
            "unexpected FSE states count not power of 2"
        );
        usize::BITS - self.states.len().leading_zeros() - 1
    }

    fn get(&self, index: usize) -> Result<&FseState> {
        self.states.get(index).ok_or(Error::Fse(MissingState))
    }

    pub fn parse(parser: &mut ForwardBitParser) -> Result<Self> {
        let (al, dist) = parse_fse_table(parser)?;
        Self::from_distribution(al, dist.as_slice())
    }

    pub fn from_distribution(accuracy_log: u8, distribution: &[Probability]) -> Result<Self> {
        let table_length = 1 << accuracy_log;
        let mut states = vec![FseState::default(); table_length];
        let mut set_index = HashSet::<usize>::new();

        let distribution: Vec<(Symbol, Probability)> = distribution
            .iter()
            .enumerate()
            .map(|(symbol, &probability)| (symbol as Symbol, probability))
            .collect();

        // Symbols with "less than 1" probabilities
        let mut less_than_one: Vec<Symbol> = distribution
            .iter()
            .filter(|&e| e.1 == -1)
            .map(|&e| e.0)
            .collect();

        // sort symbols based on lowest value first
        less_than_one.sort();
        for (i, symbol) in less_than_one.iter().enumerate() {
            let index = table_length - 1 - i;
            let state = FseState {
                symbol: *symbol,
                base_line: 0,
                num_bits: accuracy_log as usize,
            };
            states[index] = state;
            set_index.insert(index);
        }

        // closure iterator that generates next state index
        let mut state_index = std::iter::successors(Some(0_usize), |s| {
            let new_state =
                (s + (table_length >> 1) + (table_length >> 3) + 3) & (table_length - 1);
            if new_state == 0 {
                return None;
            }
            Some(new_state)
        })
        .filter(|&index| !set_index.contains(&index));

        // Symbols with positive probabilities
        let positives: Result<Vec<(Symbol, Probability, Vec<usize>)>> = distribution
            .iter()
            .filter(|&e| e.1 > 0)
            .map(|(symbol, probability)| {
                let proba = *probability as usize;
                let mut symbol_states: Vec<usize> = state_index.by_ref().take(proba).collect();

                symbol_states.sort();

                // invariant
                if symbol_states.len() != proba {
                    return Err(Error::Fse(DistributionCorrupted));
                }

                Ok((*symbol, *probability, symbol_states))
            })
            .collect();

        let positives = positives?;

        for (symbol, probability, symbol_states) in positives {
            let p = (probability as usize).next_power_of_two();
            let b = (table_length / p).trailing_zeros() as usize; // log2(R/p)
            let e = p - probability as usize;

            let mut base_line = 0;
            for (i, &index) in symbol_states.iter().cycle().skip(e).enumerate() {
                if i == symbol_states.len() {
                    break;
                }

                let i = (i + e) % symbol_states.len();
                let state = &mut states[index];
                state.symbol = symbol;
                state.num_bits = if i < e { b + 1 } else { b };
                state.base_line = base_line;
                base_line += 1 << state.num_bits;
            }
        }

        Ok(Self { states })
    }
}

fn parse_fse_table(parser: &mut ForwardBitParser) -> Result<(u8, Vec<Probability>)> {
    let accuracy_log = parser.take(4)? as u8 + ACC_LOG_OFFSET; // accuracy log
    if accuracy_log > ACC_LOG_MAX {
        return Err(Error::Fse(ALTooLarge {
            log: accuracy_log,
            max: ACC_LOG_MAX,
        }));
    }

    let probability_sum: u32 = 1 << accuracy_log;
    let mut probability_counter: u32 = 0;
    let mut probabilities: Vec<i16> = Vec::new();

    while probability_counter < probability_sum {
        let max_remaining_value: u32 = probability_sum + 1 - probability_counter;
        let bits_to_read = u32::BITS - max_remaining_value.leading_zeros();

        // Value is either encoded in: bits_to_read or bits_to_read-1
        let small_value = parser.take((bits_to_read - 1) as usize)? as u32;

        // The MSB peeked (not consumed) because value is in: bits_to_read or bits_to_read-1
        let unchecked_value = ((parser.peek()? as u32) << (bits_to_read - 1)) | small_value;

        // Threshold above wich value is encoded in bits_to_read, below which encoded in bits_to_read-1
        let low_threshold = ((1 << bits_to_read) - 1) - (max_remaining_value);

        // Used to divide in two halves the range of values encoded in bits_to_read
        let mask = (1 << (bits_to_read - 1)) - 1;

        let decoded_value = match small_value < low_threshold {
            true => small_value,
            false => {
                // consumme MSB peeked bit in unchecked_value
                let _ = parser.take(1)?;
                if unchecked_value > mask {
                    unchecked_value - low_threshold
                } else {
                    unchecked_value
                }
            }
        };

        let probability = (decoded_value as i16) - 1;

        probability_counter += probability.unsigned_abs() as u32;
        probabilities.push(probability);

        if probability == 0 {
            loop {
                let num_zeroes = parser.take(2)?;
                probabilities.extend_from_slice(&vec![0; num_zeroes as usize]);
                if num_zeroes != 0b11 {
                    break;
                }
            }
        }
    }

    // invariant
    if probability_counter != probability_sum {
        return Err(Error::Fse(DistributionCorrupted));
    }

    Ok((accuracy_log, probabilities))
}

pub struct FseDecoder {
    initialized: bool,
    table: FseTable,
    base_line: usize,
    num_bits: usize,
    symbol: Option<Symbol>,
}

impl FseDecoder {
    pub fn new(table: FseTable) -> Self {
        Self {
            table,
            initialized: false,
            base_line: 0,
            num_bits: 0,
            symbol: None,
        }
    }
}

// Refactor it, use initialized boolean var
impl BitDecoder<Symbol, Error> for FseDecoder {
    fn initialize(&mut self, bitstream: &mut BackwardBitParser) -> Result<(), Error> {
        assert!(!self.initialized, "already initialized");
        assert!(!self.table.states.is_empty(), "empty");

        self.initialized = true;

        let index = bitstream.take(self.table.accuracy_log() as usize)?;
        let state = self.table.get(index as usize)?;

        self.symbol = Some(state.symbol);
        self.num_bits = state.num_bits;
        self.base_line = state.base_line;

        Ok(())
    }

    fn expected_bits(&self) -> usize {
        assert!(self.initialized, "not initialized");
        self.num_bits
    }

    fn symbol(&mut self) -> Symbol {
        assert!(self.initialized, "not initialized");
        assert!(self.symbol.is_some(), "no symbol to consume");
        self.symbol.take().unwrap()
    }

    fn update_bits(&mut self, bitstream: &mut BackwardBitParser) -> Result<bool, Error> {
        assert!(self.initialized, "not initialized");
        assert!(self.symbol.is_none(), "symbol to consume");

        let available_bits = bitstream.available_bits();
        let expected_bits = self.expected_bits();

        let (index, zeroes) = match expected_bits <= available_bits {
            true => {
                let index = bitstream.take(expected_bits)?;
                (index + self.base_line as u64, false)
            }
            false => {
                let diff = expected_bits - available_bits;
                let index = bitstream.take(available_bits)? << diff;
                (index + self.base_line as u64, true)
            }
        };

        let state = self.table.get(index as usize)?;
        self.symbol = Some(state.symbol);
        self.num_bits = state.num_bits;
        self.base_line = state.base_line;

        Ok(zeroes)
    }

    fn reset(&mut self) {
        self.initialized = false;
        self.symbol = None;
        self.num_bits = 0;
        self.base_line = 0;
    }
}

// #[cfg(test)]
impl std::fmt::Display for FseTable {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(fmt, "State,Sym,BL,NB").ok();
        for (i, state) in self.states.iter().enumerate() {
            writeln!(
                fmt,
                "0x{:02x},s{},0x{:02x},{}",
                i, state.symbol, state.base_line, state.num_bits
            )
            .ok();
        }
        write!(fmt, "")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod fse_decoder {
        use super::*;

        #[test]
        fn test_decoder() {
            let mut bitstream = BackwardBitParser::new(&[0b0011_1100, 0b0001_0111]).unwrap();
            let mut parser = ForwardBitParser::new(&[0x30, 0x6f, 0x9b, 0x03]);
            let fse_table = FseTable::parse(&mut parser).unwrap();
            let mut decoder = FseDecoder::new(fse_table);
            decoder.initialize(&mut bitstream).unwrap();
        }
    }

    mod fse_table {
        use super::*;

        #[test]
        fn test_parse_distribution() {
            let mut parser = ForwardBitParser::new(&[0x30, 0x6f, 0x9b, 0x03]);
            let (accuracy_log, table) = parse_fse_table(&mut parser).unwrap();
            assert_eq!(5, accuracy_log);
            assert_eq!(&[18, 6, 2, 2, 2, 1, 1][..], &table);
            assert_eq!(parser.available_bits(), 6);
            assert_eq!(parser.len(), 0);
        }

        #[test]
        fn test_parse() {
            let mut parser = ForwardBitParser::new(&[0x30, 0x6f, 0x9b, 0x03]);
            let state = FseTable::parse(&mut parser).unwrap();
            // This is not a robust test as it relies on the Debug trait implementation.
            // However it is most likely to fail because of formatting rather than `parse` logic
            // so I'm fine with it. I dont really expect the Debug trait implementation to change in the future.
            let expected = r#"
State,Sym,BL,NB
0x00,s0,0x04,1
0x01,s0,0x06,1
0x02,s0,0x08,1
0x03,s1,0x10,3
0x04,s4,0x00,4
0x05,s0,0x0a,1
0x06,s0,0x0c,1
0x07,s0,0x0e,1
0x08,s2,0x00,4
0x09,s6,0x00,5
0x0a,s0,0x10,1
0x0b,s0,0x12,1
0x0c,s1,0x18,3
0x0d,s3,0x00,4
0x0e,s0,0x14,1
0x0f,s0,0x16,1
0x10,s0,0x18,1
0x11,s1,0x00,2
0x12,s5,0x00,5
0x13,s0,0x1a,1
0x14,s0,0x1c,1
0x15,s1,0x04,2
0x16,s3,0x10,4
0x17,s0,0x1e,1
0x18,s0,0x00,0
0x19,s0,0x01,0
0x1a,s1,0x08,2
0x1b,s4,0x10,4
0x1c,s0,0x02,0
0x1d,s0,0x03,0
0x1e,s1,0x0c,2
0x1f,s2,0x10,4
"#;
            assert_eq!(expected.trim(), format!("{}", state).trim());

            let mut parser = ForwardBitParser::new(&[
                0x21, 0x9d, 0x51, 0xcc, 0x18, 0x42, 0x44, 0x81, 0x8c, 0x94, 0xb4, 0x50, 0x1e,
            ]);
            let state = FseTable::parse(&mut parser).unwrap();
            // Same remark as above. Example is also taken from Nigel Tao's examples.
            let expected = r#"
State,Sym,BL,NB
0x00,s0,0x04,2
0x01,s0,0x08,2
0x02,s0,0x0c,2
0x03,s0,0x10,2
0x04,s0,0x14,2
0x05,s0,0x18,2
0x06,s1,0x20,4
0x07,s1,0x30,4
0x08,s2,0x00,5
0x09,s3,0x00,4
0x0a,s4,0x10,4
0x0b,s4,0x20,4
0x0c,s6,0x00,5
0x0d,s8,0x20,5
0x0e,s9,0x20,5
0x0f,s10,0x20,5
0x10,s12,0x00,6
0x11,s14,0x00,6
0x12,s15,0x00,4
0x13,s17,0x00,6
0x14,s20,0x00,6
0x15,s24,0x20,5
0x16,s0,0x1c,2
0x17,s0,0x20,2
0x18,s0,0x24,2
0x19,s0,0x28,2
0x1a,s0,0x2c,2
0x1b,s1,0x00,3
0x1c,s1,0x08,3
0x1d,s2,0x20,5
0x1e,s3,0x10,4
0x1f,s4,0x30,4
0x20,s4,0x00,3
0x21,s5,0x00,5
0x22,s7,0x00,6
0x23,s8,0x00,4
0x24,s9,0x00,4
0x25,s10,0x00,4
0x26,s13,0x00,5
0x27,s15,0x10,4
0x28,s16,0x00,6
0x29,s18,0x00,5
0x2a,s24,0x00,4
0x2b,s0,0x30,2
0x2c,s0,0x34,2
0x2d,s0,0x38,2
0x2e,s0,0x3c,2
0x2f,s0,0x00,1
0x30,s0,0x02,1
0x31,s1,0x10,3
0x32,s1,0x18,3
0x33,s3,0x20,4
0x34,s3,0x30,4
0x35,s4,0x08,3
0x36,s5,0x20,5
0x37,s6,0x20,5
0x38,s8,0x10,4
0x39,s9,0x10,4
0x3a,s10,0x10,4
0x3b,s13,0x20,5
0x3c,s15,0x20,4
0x3d,s15,0x30,4
0x3e,s18,0x20,5
0x3f,s24,0x10,4        
"#;
            assert_eq!(expected.trim(), format!("{}", state).trim());
        }

        #[test]
        fn test_from_distribution_cross_check() {
            // Cross check the predefined FSE distribution used by sequence decoder:
            // https://github.com/facebook/zstd/blob/dev/doc/zstd_compression_format.md#appendix-a---decoding-tables-for-predefined-codes

            let literals_distribution = [
                4, 3, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 3, 2, 1,
                1, 1, 1, 1, -1, -1, -1, -1,
            ];
            let state = FseTable::from_distribution(6, &literals_distribution).unwrap();
            let expected = r#"
State,Sym,BL,NB
0x00,s0,0x00,4
0x01,s0,0x10,4
0x02,s1,0x20,5
0x03,s3,0x00,5
0x04,s4,0x00,5
0x05,s6,0x00,5
0x06,s7,0x00,5
0x07,s9,0x00,5
0x08,s10,0x00,5
0x09,s12,0x00,5
0x0a,s14,0x00,6
0x0b,s16,0x00,5
0x0c,s18,0x00,5
0x0d,s19,0x00,5
0x0e,s21,0x00,5
0x0f,s22,0x00,5
0x10,s24,0x00,5
0x11,s25,0x20,5
0x12,s26,0x00,5
0x13,s27,0x00,6
0x14,s29,0x00,6
0x15,s31,0x00,6
0x16,s0,0x20,4
0x17,s1,0x00,4
0x18,s2,0x00,5
0x19,s4,0x20,5
0x1a,s5,0x00,5
0x1b,s7,0x20,5
0x1c,s8,0x00,5
0x1d,s10,0x20,5
0x1e,s11,0x00,5
0x1f,s13,0x00,6
0x20,s16,0x20,5
0x21,s17,0x00,5
0x22,s19,0x20,5
0x23,s20,0x00,5
0x24,s22,0x20,5
0x25,s23,0x00,5
0x26,s25,0x00,4
0x27,s25,0x10,4
0x28,s26,0x20,5
0x29,s28,0x00,6
0x2a,s30,0x00,6
0x2b,s0,0x30,4
0x2c,s1,0x10,4
0x2d,s2,0x20,5
0x2e,s3,0x20,5
0x2f,s5,0x20,5
0x30,s6,0x20,5
0x31,s8,0x20,5
0x32,s9,0x20,5
0x33,s11,0x20,5
0x34,s12,0x20,5
0x35,s15,0x00,6
0x36,s17,0x20,5
0x37,s18,0x20,5
0x38,s20,0x20,5
0x39,s21,0x20,5
0x3a,s23,0x20,5
0x3b,s24,0x20,5
0x3c,s35,0x00,6
0x3d,s34,0x00,6
0x3e,s33,0x00,6
0x3f,s32,0x00,6
"#;
            assert_eq!(expected.trim(), format!("{}", state).trim());

            let match_distribution = [
                1, 4, 3, 2, 2, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
                1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, -1, -1, -1, -1, -1, -1, -1,
            ];
            let state = FseTable::from_distribution(6, &match_distribution).unwrap();
            let expected = r#"
State,Sym,BL,NB
0x00,s0,0x00,6
0x01,s1,0x00,4
0x02,s2,0x20,5
0x03,s3,0x00,5
0x04,s5,0x00,5
0x05,s6,0x00,5
0x06,s8,0x00,5
0x07,s10,0x00,6
0x08,s13,0x00,6
0x09,s16,0x00,6
0x0a,s19,0x00,6
0x0b,s22,0x00,6
0x0c,s25,0x00,6
0x0d,s28,0x00,6
0x0e,s31,0x00,6
0x0f,s33,0x00,6
0x10,s35,0x00,6
0x11,s37,0x00,6
0x12,s39,0x00,6
0x13,s41,0x00,6
0x14,s43,0x00,6
0x15,s45,0x00,6
0x16,s1,0x10,4
0x17,s2,0x00,4
0x18,s3,0x20,5
0x19,s4,0x00,5
0x1a,s6,0x20,5
0x1b,s7,0x00,5
0x1c,s9,0x00,6
0x1d,s12,0x00,6
0x1e,s15,0x00,6
0x1f,s18,0x00,6
0x20,s21,0x00,6
0x21,s24,0x00,6
0x22,s27,0x00,6
0x23,s30,0x00,6
0x24,s32,0x00,6
0x25,s34,0x00,6
0x26,s36,0x00,6
0x27,s38,0x00,6
0x28,s40,0x00,6
0x29,s42,0x00,6
0x2a,s44,0x00,6
0x2b,s1,0x20,4
0x2c,s1,0x30,4
0x2d,s2,0x10,4
0x2e,s4,0x20,5
0x2f,s5,0x20,5
0x30,s7,0x20,5
0x31,s8,0x20,5
0x32,s11,0x00,6
0x33,s14,0x00,6
0x34,s17,0x00,6
0x35,s20,0x00,6
0x36,s23,0x00,6
0x37,s26,0x00,6
0x38,s29,0x00,6
0x39,s52,0x00,6
0x3a,s51,0x00,6
0x3b,s50,0x00,6
0x3c,s49,0x00,6
0x3d,s48,0x00,6
0x3e,s47,0x00,6
0x3f,s46,0x00,6
"#;
            assert_eq!(expected.trim(), format!("{}", state).trim());

            let offset_distribution = [
                1, 1, 1, 1, 1, 1, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, -1, -1, -1,
                -1, -1,
            ];
            let state = FseTable::from_distribution(5, &offset_distribution).unwrap();
            let expected = r#"
State,Sym,BL,NB
0x00,s0,0x00,5
0x01,s6,0x00,4
0x02,s9,0x00,5
0x03,s15,0x00,5
0x04,s21,0x00,5
0x05,s3,0x00,5
0x06,s7,0x00,4
0x07,s12,0x00,5
0x08,s18,0x00,5
0x09,s23,0x00,5
0x0a,s5,0x00,5
0x0b,s8,0x00,4
0x0c,s14,0x00,5
0x0d,s20,0x00,5
0x0e,s2,0x00,5
0x0f,s7,0x10,4
0x10,s11,0x00,5
0x11,s17,0x00,5
0x12,s22,0x00,5
0x13,s4,0x00,5
0x14,s8,0x10,4
0x15,s13,0x00,5
0x16,s19,0x00,5
0x17,s1,0x00,5
0x18,s6,0x10,4
0x19,s10,0x00,5
0x1a,s16,0x00,5
0x1b,s28,0x00,5
0x1c,s27,0x00,5
0x1d,s26,0x00,5
0x1e,s25,0x00,5
0x1f,s24,0x00,5
"#;
            assert_eq!(expected.trim(), format!("{}", state).trim());
        }
    }
}

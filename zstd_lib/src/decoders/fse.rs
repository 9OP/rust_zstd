use super::{
    BitDecoder,
    Error::{self, *},
    Result,
};
use crate::parsing::*;
use std::fmt;

#[derive(Clone)]
pub struct FseTable {
    pub states: Vec<FseState>,
    pub accuracy_log: u8,
}

impl fmt::Debug for FseTable {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
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

// Aliased types for better code clarity
pub type Symbol = u16;
type Probability = i16;

#[derive(Debug, Default, Clone, Copy)]
pub struct FseState {
    pub symbol: Symbol,
    pub base_line: u16, // could be usize
    pub num_bits: u16,  // could be usize
}

const ACC_LOG_OFFSET: u8 = 5;
const ACC_LOG_MAX: u8 = 9;

impl FseTable {
    fn get(&self, index: usize) -> Result<&FseState> {
        self.states.get(index).ok_or(MissingSymbol)
    }

    pub fn parse(parser: &mut ForwardBitParser) -> Result<Self> {
        let (accuracy_log, distribution) = parse_fse_table(parser)?;
        Ok(Self::from_distribution(
            accuracy_log,
            distribution.as_slice(),
        ))
    }

    pub fn from_distribution(accuracy_log: u8, distribution: &[Probability]) -> Self {
        let table_length = 1 << accuracy_log;
        let mut states = vec![FseState::default(); table_length];

        // Filter out symbols with 0 probability
        let distribution: Vec<(Symbol, Probability)> = distribution
            .iter()
            .enumerate()
            .filter(|(_, &probability)| probability != 0)
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
            let state = &mut states[table_length - 1 - i];
            state.base_line = 0;
            state.num_bits = accuracy_log as u16;
            state.symbol = *symbol;
        }

        // closure iterator that generates next state index
        let mut state_index = std::iter::successors(Some(0_usize), |s| {
            let new_state = (s + (table_length >> 1) + (table_length >> 3) + 3) % table_length;
            if new_state == 0 {
                return None;
            }
            Some(new_state)
        })
        .filter(|&index| states[index].symbol == 0);

        // Symbols with positive probabilities
        let positives: Vec<(Symbol, Probability, Vec<usize>)> = distribution
            .iter()
            .filter(|&e| e.1 > 0)
            .map(|(symbol, probability)| {
                let mut symbol_states: Vec<usize> =
                    state_index.by_ref().take(*probability as usize).collect();

                symbol_states.sort();

                // check invariant, TODO: create panic message
                assert!(symbol_states.len() == *probability as usize);

                (*symbol, *probability, symbol_states)
            })
            .collect();

        for (symbol, probability, symbol_states) in positives {
            let p = (probability as usize).next_power_of_two();
            let b = (table_length / p).trailing_zeros() as u16; // log2(R/p)
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

        Self {
            states,
            accuracy_log,
        }
    }
}

pub fn parse_fse_table(parser: &mut ForwardBitParser) -> Result<(u8, Vec<Probability>)> {
    let accuracy_log = parser.take(4)? as u8 + ACC_LOG_OFFSET; // accuracy log
    if accuracy_log > ACC_LOG_MAX {
        return Err(AccLogTooBig {
            log: accuracy_log,
            max: ACC_LOG_MAX,
        });
    }
    let probability_sum = 1 << accuracy_log;
    let mut probability_counter: u32 = 0;
    let mut probabilities: Vec<i16> = Vec::new();

    while probability_counter < probability_sum {
        let max_remaining_value = probability_sum - probability_counter + 1;
        let bits_to_read = u32::BITS - max_remaining_value.leading_zeros();

        // Value is either encoded in bits_to_read of bits_to_read-1
        let small_value = parser.take((bits_to_read - 1) as usize)? as u32;
        // The MSB is not consummed but peeked as we dont know yet if the value is encoded in bits_to_read or bits_to_read-1
        let unchecked_value = (parser.peek()? << (bits_to_read - 1)) as u32 | small_value;
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

        let probability = decoded_value as i16 - 1;

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

    // Check invariant
    if probability_counter != probability_sum {
        return Err(DistributionCorrupted);
    }

    Ok((accuracy_log, probabilities))
}

pub struct FseDecoder {
    table: FseTable,
    state: Option<FseState>,
}

impl FseDecoder {
    pub fn new(table: FseTable) -> Self {
        Self { table, state: None }
    }
}

impl BitDecoder<Error, Symbol> for FseDecoder {
    fn initialize(&mut self, bitstream: &mut BackwardBitParser) -> Result<(), Error> {
        assert!(
            !self.state.is_none(),
            "FseDecoder instance is already initialized"
        );
        assert!(
            !self.table.states.is_empty(),
            "FseDecoder states table is empty"
        );

        // read |accuracy_log| bits to get the initial state
        let initial_state_index = bitstream.take(self.table.accuracy_log as usize)?;
        let initial_state = self.table.get(initial_state_index as usize)?;
        self.state = Some(*initial_state);
        Ok(())
    }

    fn expected_bits(&self) -> usize {
        if let Some(state) = self.state {
            assert!(state.num_bits > 0, "No bits expected");
            return state.num_bits as usize;
        }
        panic!("FseDecoder instance not initialized");
    }

    fn symbol(&mut self) -> Symbol {
        if let Some(state) = self.state {
            let symbol = state.symbol;
            self.state = None;
            return symbol;
        }
        panic!("FseDecoder instance not initialized");
    }

    fn update_bits(&mut self, bitstream: &mut BackwardBitParser) -> Result<bool, Error> {
        if self.state.is_none() {
            panic!("FseDecoder instance not initialized");
        }
        let state = self.state.unwrap();
        let available_bits = bitstream.available_bits();
        let expected_bits = self.expected_bits();

        let (state_index, completing_with_zeros) = match expected_bits <= available_bits {
            true => {
                let index = bitstream.take(expected_bits)?;
                (index + state.base_line as u64, false)
            }
            false => {
                let diff = expected_bits - available_bits;
                let index = bitstream.take(available_bits)? << diff;
                (index + state.base_line as u64, true)
            }
        };
        let state = self.table.get(state_index as usize)?;
        self.state = Some(*state);
        Ok(completing_with_zeros)
    }

    fn reset(&mut self) {
        self.state = None;
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

            assert_eq!(bitstream.available_bits(), 8);
            assert_eq!(decoder.update_bits(&mut bitstream).unwrap(), false);
            assert_eq!(decoder.symbol(), 0);

            assert_eq!(bitstream.available_bits(), 7);
            assert_eq!(decoder.update_bits(&mut bitstream).unwrap(), false);
            assert_eq!(decoder.symbol(), 0);

            assert_eq!(bitstream.available_bits(), 6);
            assert_eq!(decoder.update_bits(&mut bitstream).unwrap(), false);
            assert_eq!(decoder.symbol(), 0);
        }
    }

    mod fse_table {
        use super::*;

        #[test]
        fn test_parse_fse_table() {
            let mut parser = ForwardBitParser::new(&[0x30, 0x6f, 0x9b, 0x03]);
            let (accuracy_log, table) = parse_fse_table(&mut parser).unwrap();
            assert_eq!(5, accuracy_log);
            assert_eq!(&[18, 6, 2, 2, 2, 1, 1][..], &table);
            assert_eq!(parser.available_bits(), 6);
            assert_eq!(parser.len(), 1);
        }

        #[test]
        fn test_from_distribution() {
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
            assert_eq!(expected.trim(), format!("{:?}", state).trim());

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
            assert_eq!(expected.trim(), format!("{:?}", state).trim());
        }
    }
}

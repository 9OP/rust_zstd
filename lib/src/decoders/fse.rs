use super::error::{Error::*, Result};
use crate::parsing::*;

const ACC_LOG_OFFSET: u8 = 5;
const ACC_LOG_MAX: u8 = 9;

pub struct FseTable {
    pub states: Vec<State>,
}

#[derive(Debug, Default, Clone)]
pub struct State {
    pub base_line: u16,
    pub symbol: u16,
    pub num_bits: u8,
    // check that state is unmodified
    marked: bool,
}

impl FseTable {
    pub fn parse(parser: &mut ForwardBitParser) -> Result<Self> {
        let (accuracy_log, distribution) = parse_fse_table(parser)?;
        Ok(Self::from_distribution(
            accuracy_log,
            distribution.as_slice(),
        ))
    }

    pub fn from_distribution(accuracy_log: u8, distribution: &[i16]) -> Self {
        let length = 1 << accuracy_log;
        let mut states = vec![State::default(); length];

        // Filter out symbols with 0 probability
        let distribution: Vec<(u16, i16)> = distribution
            .iter()
            .enumerate()
            .filter(|(_, &probability)| probability != 0)
            .map(|(symbol, &probability)| (symbol as u16, probability))
            .collect();

        // Symbols with "less than 1" probabilities
        let mut less_than_one: Vec<u16> = distribution
            .iter()
            .filter(|&e| e.1 == -1)
            .map(|&e| e.0)
            .collect();

        // sort symbols based on lowest value first
        less_than_one.sort();
        for (i, symbol) in less_than_one.iter().enumerate() {
            let state = &mut states[length - 1 - i];
            assert!(!state.marked);
            state.marked = true;
            state.base_line = 0;
            state.num_bits = accuracy_log;
            state.symbol = *symbol;
        }

        Self { states }
    }
}

pub fn parse_fse_table(parser: &mut ForwardBitParser) -> Result<(u8, Vec<i16>)> {
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

#[cfg(test)]
mod tests {
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
}

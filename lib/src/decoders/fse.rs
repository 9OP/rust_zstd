use super::error::{Error::*, Result};
use crate::parsing::*;

const ACC_LOG_OFFSET: u8 = 5;
const ACC_LOG_MAX: u8 = 9;

fn highest_bit_set(x: u32) -> u32 {
    assert!(x > 0);
    u32::BITS - x.leading_zeros()
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
        let bits_to_read = highest_bit_set(max_remaining_value);
        // Value is encoded in bits_to_read or bits_to_read-1
        // the MSB is not consummed but peeked

        // let small_value =

        let unchecked_value = parser.take((bits_to_read - 1) as usize)? as u32
            | (parser.peek()? << (bits_to_read - 1)) as u32;

        let low_threshold = ((1 << bits_to_read) - 1) - (max_remaining_value);
        let mask = (1 << (bits_to_read - 1)) - 1;
        let small_value = unchecked_value & mask;

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
                if num_zeroes != 3 {
                    break;
                }
            }
        }
    }

    // Check invariant
    if probability_counter != probability_sum {
        return Err(CounterMismatch {
            counter: probability_counter,
            expected_sum: probability_sum,
        });
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
    }
}

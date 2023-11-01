use super::error::{Error::*, Result};
use crate::parsing::*;

fn num_bits_needed(value: u32) -> u32 {
    if value == 0 {
        return 1;
    }
    (value as f64).log2().ceil() as u32
}

pub fn parse_fse_table(parser: &mut ForwardBitParser) -> Result<(u8, Vec<i16>)> {
    let al = parser.take(4)? as u8 + 5; // accuracy log
    if al > 9 {
        panic!("unexpected accuracy log: {al} > 9")
    }
    let mut distribution: Vec<i16> = Vec::new();
    let r = 0b0000_0001 << al; // 2^al=R range

    let mut total_probabilities: u32 = 0;
    while total_probabilities < r {
        let max = r - total_probabilities;
        let nbits = num_bits_needed(max + 1) - 1;
        let mut value = parser.take(nbits as usize)? as u32;

        let peek = parser.peek()? as u32;
        if ((peek << nbits) + value) <= max as u32 {
            value += (parser.take(1)? << nbits) as u32;
        }

        let probability = value as i16 - 1;

        if (total_probabilities + probability.abs() as u32) > r {
            return Err(ComputeFseCoefficient);
        }

        total_probabilities += if probability != 0 {
            probability.abs() as u32
        } else {
            1
        };
        // total_probabilities += probability.abs() as u32;
        distribution.push(probability);

        if probability == 0 {
            loop {
                let num_zeroes = parser.take(2)?;
                distribution.extend_from_slice(&vec![0; num_zeroes as usize]);
                if num_zeroes != 3 {
                    break;
                }
            }
        }
    }

    return Ok((al, distribution));
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
        // assert_eq!(parser.available_bits(), 6);
    }
}

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

    dbg!(&distribution, &r, &al);

    let mut sum = 0; // sum of symbols
    while sum < r {
        let m = r - sum; // max value of symbol
        let b = num_bits_needed(m + 1) as usize; // number of bits to encode [0,m+1] values

        let mut value = parser.take(b - 1)?;

        if ((value << 1) + 1) <= m as u64 {
            value = (value << 1) + parser.take(1)?;
        }

        dbg!(&distribution, &m, &b, &value, &sum);
        println!("======");

        let coefficient = value - 1;
        if (sum as u64 + coefficient) > r as u64 {
            return Err(ComputeFseCoefficient);
        }
        sum += coefficient as u32;
        distribution.push(coefficient as i16);

        if coefficient == 0 {
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
        assert_eq!(parser.len(), 6);
    }
}

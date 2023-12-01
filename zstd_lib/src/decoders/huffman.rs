use super::{BitDecoder, Error::*, Result};
use crate::{
    decoders::{AlternatingDecoder, FseTable},
    parsing::{BackwardBitParser, ForwardBitParser, ForwardByteParser},
};
use std::fmt;

// TODO:Create huffman error type

#[derive(PartialEq)]
pub enum HuffmanDecoder {
    Absent,
    Symbol(u8),
    Tree(Box<HuffmanDecoder>, Box<HuffmanDecoder>),
}
use HuffmanDecoder::*;

const MAX_NUM_BITS: u32 = 11;

impl<'a> HuffmanDecoder {
    fn from_number_of_bits(widths: Vec<u8>) -> Self {
        assert!(widths.len() <= 255, "unexpected number of symbols");

        // Build a list of symbols and their widths
        let mut symbols: Vec<(u8, u8)> = widths
            .iter()
            .enumerate()
            .filter(|(_, &width)| width > 0)
            .map(|(symbol, &width)| (symbol as u8, width))
            .collect();

        // Sort symbols based on highest width and lowest symbol value
        symbols.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

        let mut tree = HuffmanDecoder::Absent;
        for (symbol, width) in symbols {
            tree.insert(symbol, width);
        }
        tree
    }

    // Return the last weight and the maximum width
    fn compute_last_weight(weights_sum: u32) -> Result<(u8, u8)> {
        // max_width is the log2 of the sum 2^(w−1) for all weights.
        // we dont know the last_weight yet, but we now that the sum
        // should be a power of two. We deduce the max_width as the greater
        // nearest power of two of weights_sum.
        let max_width = u32::BITS - weights_sum.leading_zeros();

        // safety check: max_width is bounded
        if max_width > MAX_NUM_BITS {
            return Err(WeightTooBig {
                weight: max_width,
                max: MAX_NUM_BITS,
            });
        }

        // since: weights_sum + 2^(last_weigth-1) = 2^max_width
        // last_weight = log2(2^max_width - weight_sum) + 1
        let left_over = (1 << max_width) - weights_sum;

        // safety check: left_over is a clean power of 2
        if !left_over.is_power_of_two() {
            return Err(ComputeMissingWeight);
        }

        // left_over is a clean power of 2 (ie. only one bit is set)
        // the log2 is the number of leading zeroes minus 1.
        let last_weight = (u32::BITS - left_over.leading_zeros() - 1) + 1;

        // safety check: no 2^(w-1) is greater that the sum of others
        if last_weight > weights_sum {
            return Err(ComputeMissingWeight);
        }

        Ok((last_weight as u8, max_width as u8))
    }

    pub fn from_weights(weights: Vec<u8>) -> Result<Self> {
        let mut weights = weights.clone();

        let mut weights_sum: u32 = 0;
        for w in &weights {
            if *w as u32 > MAX_NUM_BITS {
                return Err(WeightTooBig {
                    weight: *w as u32,
                    max: MAX_NUM_BITS,
                });
            }
            weights_sum += if *w > 0 { 1_u32 << (*w - 1) } else { 0 };
        }

        if weights_sum == 0 {
            return Err(ComputeMissingWeight);
        }

        // TODO: ensure the properties:
        // - If no literal has a Weight of 1, then the data is considered corrupted.
        // - If there are not at least two literals with non-zero Weight, then the data is considered corrupted.

        let (missing_weight, max_width) = Self::compute_last_weight(weights_sum)?;
        weights.push(missing_weight);

        let widths = weights
            .iter()
            .map(|w| if *w > 0 { max_width + 1 - *w } else { 0 })
            .collect();

        Ok(Self::from_number_of_bits(widths))
    }

    pub fn insert(&mut self, symbol: u8, width: u8) -> bool {
        if width == 0 {
            if let Absent = self {
                *self = Symbol(symbol);
                return true;
            }
            return false;
        }

        match self {
            Symbol(_) => panic!("invalid Huffman tree decoder"),
            Tree(lhs, rhs) => {
                if lhs.insert(symbol, width - 1) {
                    return true;
                }
                rhs.insert(symbol, width - 1)
            }
            Absent => {
                *self = Tree(Box::new(Absent), Box::new(Absent));
                self.insert(symbol, width)
            }
        }
    }

    pub fn decode(&self, parser: &mut BackwardBitParser) -> Result<u8> {
        match self {
            Absent => Err(MissingSymbol),
            Symbol(s) => Ok(*s),
            Tree(lhs, rhs) => match parser.take(1)? {
                0 => lhs.decode(parser),
                1 => rhs.decode(parser),
                _ => panic!("unexpected bit value"),
            },
        }
    }

    pub fn iter(&'a self) -> HuffmanDecoderIterator<'a> {
        HuffmanDecoderIterator::new(self)
    }

    /// Build a Huffman table from the given stream. Only the bytes needed to
    /// build the table are consumed from the stream.
    pub fn parse(input: &mut ForwardByteParser) -> Result<Self> {
        let header = input.u8()?;
        let weights = if header < 128 {
            Self::parse_fse(input, header)?
        } else {
            Self::parse_direct(input, header as usize - 127)?
        };
        // TODO: return error when weight.len > 255
        assert!(weights.len() <= 255, "return error TooManyWeights");
        println!("weights: {weights:?}, len: {}", weights.len());
        Self::from_weights(weights)
    }

    /// Parse the Huffman table weights directly from the stream, 4
    /// bits per weights. If there are an odd number of weights, the
    /// last four bits are lost. `number_of_weights/2` bytes (rounded
    /// up) will be consumed from the `input` stream.
    fn parse_direct(input: &mut ForwardByteParser, number_of_weights: usize) -> Result<Vec<u8>> {
        assert!(
            number_of_weights <= 128,
            "expected number_of_weights <= 128"
        );

        let mut weights = Vec::<u8>::new();
        let mut number_of_weights = number_of_weights;

        'outer: loop {
            let byte = input.u8()?;

            for shift in &[4, 0] {
                let weight = (byte >> shift) & 0b0000_1111;
                weights.push(weight);
                number_of_weights -= 1;

                if number_of_weights == 0 {
                    break 'outer;
                }
            }
        }

        Ok(weights)
    }

    /// Decode a FSE table and use an alternating FSE decoder to parse
    /// the Huffman table weights. `compressed_size` bytes will be
    /// consumed from the `input` stream.
    fn parse_fse(input: &mut ForwardByteParser, compressed_size: u8) -> Result<Vec<u8>> {
        let mut weights = Vec::<u8>::new();

        let bitstream = input.slice(compressed_size as usize)?;

        let mut forward_bit_parser = ForwardBitParser::new(&bitstream);
        let fse_table = FseTable::parse(&mut forward_bit_parser)?;
        let mut decoder = AlternatingDecoder::new(&fse_table);

        // TODO: create error
        assert!(compressed_size as usize > forward_bit_parser.len());
        let index = compressed_size as usize - forward_bit_parser.len();
        let huffman_coeffs = &bitstream[index..];

        let mut backward_bit_parser = BackwardBitParser::new(huffman_coeffs)?;
        decoder.initialize(&mut backward_bit_parser)?;

        loop {
            // TODO: remove unwrap
            // Consume alternating decoder
            weights.push(decoder.symbol().try_into().unwrap());
            if decoder.update_bits(&mut backward_bit_parser)? {
                // Consume the alternate decoder a last time
                weights.push(decoder.symbol().try_into().unwrap());
                break;
            }
        }

        // TODO: return error when weights.lent()>255
        assert!(weights.len() < 255);
        // if weights.len() > 255 {
        //     return Err(err::TooManyWeights {
        //         got: self.weights.len(),
        //     });
        // }

        Ok(weights)
    }
}

pub struct HuffmanDecoderIterator<'a> {
    nodes: Vec<(&'a HuffmanDecoder, String)>,
}
impl<'a> HuffmanDecoderIterator<'a> {
    pub fn new(tree: &'a HuffmanDecoder) -> Self {
        Self {
            nodes: vec![(tree, String::from(""))],
        }
    }
}
impl<'a> Iterator for HuffmanDecoderIterator<'a> {
    type Item = (String, u8);

    fn next(&mut self) -> Option<Self::Item> {
        let (decoder, prefix) = self.nodes.pop()?;
        match decoder {
            Absent => None,
            Symbol(s) => Some((prefix, *s)),
            Tree(lhs, rhs) => {
                self.nodes.push((lhs, prefix.clone() + "0"));
                self.nodes.push((rhs, prefix.clone() + "1"));
                self.next()
            }
        }
    }
}

impl fmt::Debug for HuffmanDecoder {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut res = fmt.debug_struct("HuffmanDecoder");
        for (prefix, symbol) in self.iter() {
            res.field(&prefix, &symbol);
        }
        res.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_tree() -> HuffmanDecoder {
        let mut tree = HuffmanDecoder::Absent;
        tree.insert(b'A', 2);
        tree.insert(b'C', 2);
        tree.insert(b'B', 1);
        tree
    }

    #[test]
    fn test_insert() {
        let mut tree = HuffmanDecoder::Absent;
        assert!(tree.insert(b'A', 2));
        assert!(tree.insert(b'C', 2));
        assert!(tree.insert(b'B', 1));
    }

    #[test]
    fn test_debug() {
        let tree = fixture_tree();
        assert_eq!(
            format!("{:?}", tree),
            "HuffmanDecoder { 1: 66, 01: 67, 00: 65 }"
        );
    }

    #[test]
    fn test_iterator() {
        let tree = fixture_tree();
        let mut iter = tree.iter();
        assert_eq!(iter.next(), Some((String::from("1"), b'B')));
        assert_eq!(iter.next(), Some((String::from("01"), b'C')));
        assert_eq!(iter.next(), Some((String::from("00"), b'A')));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_from_number_of_bits() {
        let widths: Vec<u8> = std::iter::repeat(0).take(65).chain([2, 1, 2]).collect();
        let tree = HuffmanDecoder::from_number_of_bits(widths);
        assert_eq!(
            format!("{:?}", tree),
            "HuffmanDecoder { 1: 66, 01: 67, 00: 65 }"
        );
    }

    #[test]
    fn test_compute_last_weight() {
        let weight = HuffmanDecoder::compute_last_weight(3).unwrap();
        assert_eq!(weight, (1, 2));

        let weight = HuffmanDecoder::compute_last_weight(4).unwrap();
        assert_eq!(weight, (3, 3));

        let weight = HuffmanDecoder::compute_last_weight(24).unwrap();
        assert_eq!(weight, (4, 5));

        assert!(matches!(
            HuffmanDecoder::compute_last_weight(5),
            Err(ComputeMissingWeight)
        ));
    }

    #[test]
    fn test_from_weights() {
        let weights: Vec<_> = std::iter::repeat(0).take(65).chain([1, 2]).collect();
        let tree = HuffmanDecoder::from_weights(weights).unwrap();
        assert_eq!(
            format!("{:?}", tree),
            "HuffmanDecoder { 1: 66, 01: 67, 00: 65 }"
        );
    }

    #[test]
    fn test_decode() {
        // 0 repeated 65 times, 1, 2
        let weights: Vec<_> = std::iter::repeat(0).take(65).chain([1, 2]).collect();
        let decoder = HuffmanDecoder::from_weights(weights).unwrap();
        let mut parser = BackwardBitParser::new(&[0x97, 0x01]).unwrap();
        let mut result = String::new();
        while !parser.is_empty() {
            let decoded = decoder.decode(&mut parser).unwrap();
            result.push(decoded as char); // We know they are valid A, B, or C char
        }
        assert_eq!(result, "BABCBB");
    }
}

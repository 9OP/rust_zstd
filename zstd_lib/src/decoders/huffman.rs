use super::{BitDecoder, Error::*, Result};
use crate::{
    decoders::{AlternatingDecoder, FseTable},
    parsing::{BackwardBitParser, ForwardBitParser, ForwardByteParser},
};
use std::fmt;

pub enum HuffmanDecoder {
    Absent,
    Symbol(u8),
    Tree(Box<HuffmanDecoder>, Box<HuffmanDecoder>),
}
use HuffmanDecoder::*;

impl<'a> HuffmanDecoder {
    fn from_number_of_bits(widths: Vec<u8>) -> Self {
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

    fn compute_missing_weight(weights_sum: u16) -> Result<(u8, u8)> {
        let mut missing_weight: u8 = 0;

        while missing_weight <= 16 {
            missing_weight += 1; // missing weight is at least >= 1
            let prefix = 1 << (missing_weight - 1);

            // opportunistically break out
            if prefix > weights_sum {
                return Err(ComputeMissingWeight);
            }

            // stop when first strictly greater power of 2
            if (weights_sum + prefix).is_power_of_two() {
                let max_width = (weights_sum + prefix).trailing_zeros() as u8;
                return Ok((missing_weight, max_width));
            }
        }

        Err(ComputeMissingWeight)
    }

    pub fn from_weights(weights: Vec<u8>) -> Result<Self> {
        // Prevent mutating the input
        let mut weights = weights.clone();

        let weights_sum = weights
            .iter() // do not consume weights
            .filter(|w| **w != 0) // remove 0 weights
            .map(|w| 1 << (w - 1)) // apply 2**(w-1)
            .sum();

        let (missing_weight, max_width) = Self::compute_missing_weight(weights_sum)?;
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
        Self::from_weights(weights)
    }

    /// Parse the Huffman table weights directly from the stream, 4
    /// bits per weights. If there are an odd number of weights, the
    /// last four bits are lost. `number_of_weights/2` bytes (rounded
    /// up) will be consumed from the `input` stream.
    fn parse_direct(input: &mut ForwardByteParser, number_of_weights: usize) -> Result<Vec<u8>> {
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

        let mut bitstream = Vec::<u8>::new();
        for _ in 0..compressed_size {
            bitstream.push(input.u8()?);
        }
        let bitstream_copy = bitstream.clone();

        let mut forward_bit_parser = ForwardBitParser::new(bitstream.as_slice());
        let fse_table = FseTable::parse(&mut forward_bit_parser)?;
        let mut decoder = AlternatingDecoder::new(&fse_table);

        assert!(compressed_size as usize > forward_bit_parser.len());
        let index = compressed_size as usize - forward_bit_parser.len();
        let huffman_coeffs = &bitstream_copy[index..];

        let mut backward_bit_parser = BackwardBitParser::new(huffman_coeffs)?;
        decoder.initialize(&mut backward_bit_parser)?;

        loop {
            weights.push(decoder.symbol().try_into().unwrap());
            let stop = decoder.update_bits(&mut backward_bit_parser)?;
            if stop {
                break;
            }
        }

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
    fn test_compute_missing_weight() {
        let weight = HuffmanDecoder::compute_missing_weight(3).unwrap();
        assert_eq!(weight, (1, 2));

        let weight = HuffmanDecoder::compute_missing_weight(4).unwrap();
        assert_eq!(weight, (3, 3));

        let weight = HuffmanDecoder::compute_missing_weight(24).unwrap();
        assert_eq!(weight, (4, 5));

        assert!(matches!(
            HuffmanDecoder::compute_missing_weight(5),
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

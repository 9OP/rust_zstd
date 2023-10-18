use std::fmt;

pub enum HuffmanDecoder {
    Absent,
    Symbol(u8),
    Tree(Box<HuffmanDecoder>, Box<HuffmanDecoder>),
}

use HuffmanDecoder::*;

impl<'a> HuffmanDecoder {
    pub fn from_number_of_bits(widths: Vec<u8>) -> Self {
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
            Tree(lb, rb) => {
                if lb.insert(symbol, width - 1) {
                    return true;
                }
                rb.insert(symbol, width - 1)
            }
            Absent => {
                *self = Tree(Box::new(Absent), Box::new(Absent));
                self.insert(symbol, width)
            }
        }
    }

    pub fn iter(&'a self) -> HuffmanDecoderIterator<'a> {
        HuffmanDecoderIterator::new(self)
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
            Tree(rb, lb) => {
                self.nodes.push((rb, prefix.clone() + "0"));
                self.nodes.push((lb, prefix.clone() + "1"));
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
}

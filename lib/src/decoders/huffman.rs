use std::fmt;

#[derive(PartialEq)]
pub enum HuffmanDecoder {
    Absent,
    Symbol(u8),
    Tree(Box<HuffmanDecoder>, Box<HuffmanDecoder>),
}

use HuffmanDecoder::*;

impl<'a> HuffmanDecoder {
    pub fn iter(&'a self) -> HuffmanDecoderIterator<'a> {
        HuffmanDecoderIterator::new(self)
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
            Symbol(_) => panic!("huffman tree top is symbol!"),
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
        let val = self.nodes.pop()?;
        match val.0 {
            Absent => None,
            Symbol(s) => Some((val.1, *s)),
            Tree(rb, lb) => {
                self.nodes.push((rb, val.1.clone() + "0"));
                self.nodes.push((lb, val.1.clone() + "1"));
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

    #[test]
    fn test_insert() {
        let mut tree = HuffmanDecoder::Absent;
        assert!(tree.insert(b'A', 2));
        assert!(tree.insert(b'C', 2));
        assert!(tree.insert(b'B', 1));
        println!("{tree:?}");
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ParsingError {
    // Rename Parsing error and move to parsing file
    #[error("Not enough bytes: {requested:#06x} requested out of {available:#06x} available")]
    NotEnoughBytes { requested: usize, available: usize },
}

type Result<T, E = ParsingError> = std::result::Result<T, E>;

pub struct ForwardByteParser<'a>(&'a [u8]);

impl<'a> ForwardByteParser<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self(data)
    }

    /// Pop and return (side-effect)
    pub fn u8(&mut self) -> Result<u8> {
        // let (first, rest) = match self.0.split_first() {
        //     Some(v) => v,
        //     None => {
        //         return Result::Err(Error::NotEnoughBytes {
        //             requested: 1,
        //             available: 0,
        //         })
        //     }
        // };
        // equivalent
        let (first, rest) = self.0.split_first().ok_or(ParsingError::NotEnoughBytes {
            requested: 1,
            available: 0,
        })?;
        self.0 = rest;
        Ok(*first)
    }

    /// Return the number of bytes still unparsed
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Check if the input is exhausted
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Extract `len` bytes as a slice
    pub fn slice(&mut self, len: usize) -> Result<&'a [u8]> {
        match len <= self.len() {
            true => Ok(&self.0[0..len]),
            false => Err(ParsingError::NotEnoughBytes {
                requested: len,
                available: self.len(),
            }),
        }
    }

    /// Consume and return a u32 in little-endian format
    pub fn le_u32(&mut self) -> Result<u32> {
        let mut result = 0u32;
        for i in 0..4 as u8 {
            // OR + LeftShift bitwise
            // each iteration:
            //  - OR bitwise, which "copy" self.u8() to the 8LSB of result
            //  - Byte leftshift, which shift bits to the left by 8bits. 8LSB become 0x0
            //
            // little-endian: LSB is first in byte order, at i=0 no leftshift, at i=1 8bits leftshit, at i=2 16bits left shift
            result |= (self.u8()? as u32) << (i * 8)
        }
        Ok(result.to_le())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_u8() {
        let mut parser = ForwardByteParser::new(&[0x12, 0x23, 0x34]);
        assert_eq!(0x12, parser.u8().unwrap());
        assert_eq!(0x23, parser.u8().unwrap());
        assert_eq!(0x34, parser.u8().unwrap());
        assert!(matches!(
            parser.u8(),
            Err(ParsingError::NotEnoughBytes {
                requested: 1,
                available: 0,
            })
        ));
    }

    #[test]
    fn test_len() {
        let parser = ForwardByteParser::new(&[0x12, 0x23, 0x34]);
        assert_eq!(3, parser.len());
        let parser = ForwardByteParser::new(&[]);
        assert_eq!(0, parser.len());
    }

    #[test]
    fn test_is_empty() {
        let parser: ForwardByteParser<'_> = ForwardByteParser::new(&[0x12, 0x23, 0x34]);
        assert_eq!(false, parser.is_empty());
        let parser = ForwardByteParser::new(&[]);
        assert_eq!(true, parser.is_empty());
    }

    #[test]
    fn test_slice() {
        let mut parser: ForwardByteParser<'_> = ForwardByteParser::new(&[0x12, 0x23, 0x34]);
        assert_eq!(&[0x12, 0x23], parser.slice(2).unwrap());
        assert_eq!(&[0x12, 0x23, 0x34], parser.slice(3).unwrap());
        assert!(matches!(
            parser.slice(4),
            Err(ParsingError::NotEnoughBytes {
                requested: 4,
                available: 3,
            })
        ));
    }

    #[test]
    fn test_le_u32() {
        let mut parser: ForwardByteParser<'_> = ForwardByteParser::new(&[0x12, 0x34, 0x56, 0x78]);
        assert_eq!(0x78563412, parser.le_u32().unwrap());
        assert!(matches!(
            parser.le_u32(),
            Err(ParsingError::NotEnoughBytes {
                requested: 1,
                available: 0,
            })
        ));
    }
}

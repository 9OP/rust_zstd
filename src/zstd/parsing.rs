use super::errors::{Error, Result};
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
        let (first, rest) = self.0.split_first().ok_or(Error::NotEnoughBytes {
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
            false => Result::Err(Error::NotEnoughBytes {
                requested: len,
                available: self.len(),
            }),
        }
    }

    /// Consume and return a u32 in little-endian format
    pub fn le_u32(&mut self) -> Result<u32> {
        let u = self.u8()? as u32;
        Ok(u.to_le())
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
            Err(Error::NotEnoughBytes {
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
            Err(Error::NotEnoughBytes {
                requested: 4,
                available: 3,
            })
        ));
    }

    #[test]
    fn test_le_u32() {
        let mut parser: ForwardByteParser<'_> = ForwardByteParser::new(&[0x12, 0x23, 0x34]);
        assert_eq!(0x12 as u32, parser.le_u32().unwrap());
        assert_eq!(0x23 as u32, parser.le_u32().unwrap());
        assert_eq!(0x34 as u32, parser.le_u32().unwrap());
        assert!(matches!(
            parser.le_u32(),
            Err(Error::NotEnoughBytes {
                requested: 1,
                available: 0,
            })
        ));
    }
}

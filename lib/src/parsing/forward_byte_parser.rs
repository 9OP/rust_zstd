use super::error::{Error::*, Result};

pub struct ForwardByteParser<'a>(&'a [u8]);

impl<'a> ForwardByteParser<'a> {
    /// Create a new ForwardByteParse instance from a byte slice
    pub fn new(data: &'a [u8]) -> Self {
        Self(data)
    }

    /// Consume and return u8 from the byte slice
    /// or `NotEnoughByte` error when the byte slice is empty.
    /// # Example
    /// ```
    /// # use zstd_lib::parsing::{ForwardByteParser, Error};
    /// let mut parser = ForwardByteParser::new(&[0x01, 0x02, 0x03]);
    /// assert_eq!(parser.u8()?, 0x01);
    /// assert_eq!(parser.u8()?, 0x02);
    /// assert_eq!(parser.u8()?, 0x03);
    /// # Ok::<(), Error>(())
    /// ```
    pub fn u8(&mut self) -> Result<u8> {
        let (first, rest) = self.0.split_first().ok_or(NotEnoughBytes {
            requested: 1,
            available: 0,
        })?;
        self.0 = rest;
        Ok(*first)
    }

    /// Return the number of bytes still unparsed
    /// # Example
    /// ```
    /// # use zstd_lib::parsing::{ForwardByteParser, Error};
    /// let mut parser = ForwardByteParser::new(&[0x01, 0x02, 0x03]);
    /// assert_eq!(parser.len(), 3);
    /// parser.u8();
    /// assert_eq!(parser.len(), 2);
    /// ```
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Return `true` if the byte slice is exhausted
    /// # Example
    /// ```
    /// # use zstd_lib::parsing::{ForwardByteParser, Error};
    /// let mut parser = ForwardByteParser::new(&[0x01]);
    /// assert_eq!(parser.is_empty(), false);
    /// parser.u8();
    /// assert_eq!(parser.is_empty(), true);
    /// ```
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Return `len` bytes as a sub slice
    pub fn slice(&mut self, len: usize) -> Result<&'a [u8]> {
        match len <= self.len() {
            false => Err(NotEnoughBytes {
                requested: len,
                available: self.len(),
            }),
            true => {
                let (slice, rest) = self.0.split_at(len);
                self.0 = rest;
                Ok(slice)
            }
        }
    }

    /// Consume and return a u32 in little-endian format
    pub fn le_u32(&mut self) -> Result<u32> {
        // Consume 4bytes or Err. Do not consume partially !
        let byte_array = self.slice(4)?;
        let result = (byte_array[3] as u32) << 24
            | (byte_array[2] as u32) << 16
            | (byte_array[1] as u32) << 8
            | (byte_array[0] as u32);
        Ok(result.to_le())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_u8() {
        let mut parser = ForwardByteParser::new(&[0x12, 0x23, 0x34]);
        assert_eq!(parser.0.len(), 3);
        assert_eq!(parser.u8().unwrap(), 0x12);
        assert_eq!(parser.0.len(), 2);
        assert_eq!(parser.u8().unwrap(), 0x23);
        assert_eq!(parser.0.len(), 1);
        assert_eq!(parser.u8().unwrap(), 0x34);
        assert_eq!(parser.0.len(), 0);
        assert!(matches!(
            parser.u8(),
            Err(NotEnoughBytes {
                requested: 1,
                available: 0,
            })
        ));
    }

    #[test]
    fn test_len() {
        let parser = ForwardByteParser::new(&[0x12, 0x23, 0x34]);
        assert_eq!(parser.len(), 3);
        let parser = ForwardByteParser::new(&[0x12]);
        assert_eq!(parser.len(), 1);
        let parser = ForwardByteParser::new(&[]);
        assert_eq!(parser.len(), 0);
    }

    #[test]
    fn test_is_empty() {
        let parser = ForwardByteParser::new(&[0x12, 0x23, 0x34]);
        assert_eq!(false, parser.is_empty());
        let parser = ForwardByteParser::new(&[]);
        assert_eq!(true, parser.is_empty());
    }

    #[test]
    fn test_slice() {
        let mut parser = ForwardByteParser::new(&[0x12, 0x23, 0x34]);
        assert_eq!(&[] as &[u8], parser.slice(0).unwrap());
        assert_eq!(&[0x12, 0x23], parser.slice(2).unwrap());
        assert_eq!(1, parser.0.len());
        assert_eq!(&[0x34], parser.slice(1).unwrap());
        assert!(matches!(
            parser.slice(1),
            Err(NotEnoughBytes {
                requested: 1,
                available: 0,
            })
        ));
        let mut parser = ForwardByteParser::new(&[0x12, 0x23, 0x34]);
        assert!(matches!(
            parser.slice(4),
            Err(NotEnoughBytes {
                requested: 4,
                available: 3,
            })
        ));
        assert_eq!(3, parser.0.len());
        assert_eq!(&[0x12, 0x23, 0x34], parser.slice(3).unwrap());
        assert_eq!(0, parser.0.len());
    }

    #[test]
    fn test_le_u32() {
        let mut parser = ForwardByteParser::new(&[0x12, 0x34, 0x56, 0x78, 0xFF]);
        assert_eq!(5, parser.0.len());
        assert_eq!(0x78563412, parser.le_u32().unwrap());
        assert_eq!(1, parser.0.len());

        // Do not consume u8 when Error
        assert!(matches!(
            parser.le_u32(),
            Err(NotEnoughBytes {
                requested: 4,
                available: 1,
            })
        ));
        assert_eq!(1, parser.0.len());
    }
}

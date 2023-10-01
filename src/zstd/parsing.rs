use super::errors::{Error, Result};
pub struct ForwardByteParser<'a>(&'a [u8]);

impl<'a> ForwardByteParser<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self(data)
    }

    // returns parsed u8
    pub fn u8(&mut self) -> Result<u8> {
        let (first, rest) = self.0.split_first().ok_or(Error::NotEnoughBytes {
            requested: 1,
            available: 0,
        })?;
        self.0 = rest;
        return Ok(*first);
    }

    // pub fn u8(&mut self) -> Option<u8> {
    //     let (first, rest) = self.0.split_first()?;
    //     self.0 = rest;
    //     Ok(*first)
    // }

    /// Return the number of bytes still unparsed
    pub fn len(&self) -> usize {
        todo!()
    }

    /// Check if the input is exhausted
    pub fn is_empty(&self) -> bool {
        todo!()
    }

    /// Extract `len` bytes as a slice
    pub fn slice(&mut self, len: usize) -> Result<&'a [u8]> {
        todo!()
    }

    /// Consume and return a u32 in little-endian format
    pub fn le_u32(&mut self) -> Result<u32> {
        todo!()
    }
}

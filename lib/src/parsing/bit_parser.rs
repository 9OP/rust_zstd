use super::error::Result;

#[derive(Debug)]
pub struct Bitstream<'a> {
    pub data: &'a [u8],
    pub position: usize,
}

pub trait BitParser<'a> {
    fn new(bitstream: &'a [u8]) -> Result<Self>
    where
        Self: Sized;
    fn take(&mut self, len: usize) -> Result<u64>;

    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn available_bits(&mut self) -> usize;
}

impl<'a> BitParser<'a> for Bitstream<'a> {
    fn new(bitstream: &'a [u8]) -> Result<Self>
    where
        Self: Sized,
    {
        unimplemented!();
    }

    fn take(&mut self, len: usize) -> Result<u64> {
        unimplemented!();
    }

    fn len(&self) -> usize {
        self.bitstream.len()
    }

    fn available_bits(&mut self) -> usize {
        if self.is_empty() {
            return 0;
        }
        8 * (self.len() - 1) + self.position + 1
    }
}

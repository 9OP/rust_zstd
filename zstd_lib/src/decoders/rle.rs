use super::{BackwardBitParser, BitDecoder, Error};

#[derive(Debug)]
pub struct RLEDecoder {
    symbol: u16,
}

impl RLEDecoder {
    pub fn new(symbol: u16) -> Self {
        Self { symbol }
    }
}

impl BitDecoder<u16, Error> for RLEDecoder {
    fn initialize(&mut self, _: &mut BackwardBitParser) -> Result<(), Error> {
        Ok(())
    }

    fn expected_bits(&self) -> usize {
        unimplemented!("expected_bits not supported for RLEDecoder")
    }

    fn symbol(&mut self) -> u16 {
        self.symbol
    }

    fn update_bits(&mut self, _: &mut BackwardBitParser) -> Result<bool, Error> {
        Ok(false)
    }

    fn reset(&mut self) {}
}

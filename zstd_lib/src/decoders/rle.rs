use super::{BitDecoder, Error};

pub struct RLEDecoder {
    pub symbol: u16,
}

impl BitDecoder<u16, Error> for RLEDecoder {
    fn initialize(
        &mut self,
        _bitstream: &mut crate::parsing::BackwardBitParser,
    ) -> Result<(), Error> {
        unimplemented!("initialize not supported for RLEDecoder")
    }

    fn expected_bits(&self) -> usize {
        unimplemented!("expected_bits not supported for RLEDecoder")
    }

    fn symbol(&mut self) -> u16 {
        self.symbol
    }

    fn update_bits(
        &mut self,
        _bitstream: &mut crate::parsing::BackwardBitParser,
    ) -> Result<bool, Error> {
        unimplemented!("update_bits not supported for RLEDecoder")
    }

    fn reset(&mut self) {
        unimplemented!("reset not supported for RLEDecoder")
    }
}

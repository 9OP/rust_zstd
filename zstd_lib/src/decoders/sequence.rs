use super::{BitDecoder, Error};

pub struct SequenceDecoder {
    pub literals_lengths_decoder: Box<dyn BitDecoder<u16, Error>>,
    pub offsets_decoder: Box<dyn BitDecoder<u16, Error>>,
    pub match_lengths_decoder: Box<dyn BitDecoder<u16, Error>>,
}

impl BitDecoder<(u16, u16, u16), Error> for SequenceDecoder {
    fn initialize(
        &mut self,
        _bitstream: &mut crate::parsing::BackwardBitParser,
    ) -> Result<(), Error> {
        unimplemented!()
        // self.literals_lengths_decoder.initialize(bitstream)?;
        // self.offsets_decoder.initialize(bitstream)?;
        // self.match_lengths_decoder.initialize(bitstream)?;
        // Ok(())
    }

    fn expected_bits(&self) -> usize {
        unimplemented!()
    }

    fn symbol(&mut self) -> (u16, u16, u16) {
        let literals_code = self.literals_lengths_decoder.symbol();
        let offset_code = self.offsets_decoder.symbol();
        let match_code = self.match_lengths_decoder.symbol();
        (literals_code, offset_code, match_code)
    }

    fn update_bits(
        &mut self,
        bitstream: &mut crate::parsing::BackwardBitParser,
    ) -> Result<bool, Error> {
        // update order: literals > offsets > match
        let mut zeroes = self.literals_lengths_decoder.update_bits(bitstream)?;
        zeroes |= self.offsets_decoder.update_bits(bitstream)?;
        zeroes |= self.match_lengths_decoder.update_bits(bitstream)?;

        Ok(zeroes)
    }

    fn reset(&mut self) {
        unimplemented!()
        // self.literals_lengths_decoder.reset();
        // self.offsets_decoder.reset();
        // self.match_lengths_decoder.reset();
    }
}

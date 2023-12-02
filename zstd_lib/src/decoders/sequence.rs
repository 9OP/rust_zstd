use super::{BitDecoder, Error};
use crate::parsing::BackwardBitParser;

pub type SymbolDecoder = dyn BitDecoder<u16, Error>;
pub struct SequenceDecoder<'d> {
    literals_lengths_decoder: &'d mut SymbolDecoder,
    offsets_decoder: &'d mut SymbolDecoder,
    match_lengths_decoder: &'d mut SymbolDecoder,
}

impl<'a> SequenceDecoder<'a> {
    pub fn new(
        ll_d: &'a mut Box<SymbolDecoder>,
        o_d: &'a mut Box<SymbolDecoder>,
        ml_d: &'a mut Box<SymbolDecoder>,
    ) -> Self {
        Self {
            literals_lengths_decoder: &mut **ll_d,
            offsets_decoder: &mut **o_d,
            match_lengths_decoder: &mut **ml_d,
        }
    }
}

impl BitDecoder<(u16, u16, u16), Error> for SequenceDecoder<'_> {
    fn initialize(&mut self, _: &mut BackwardBitParser) -> Result<(), Error> {
        unimplemented!("initialize not supported for SequenceDecoder")
    }

    fn expected_bits(&self) -> usize {
        unimplemented!("expected_bits not supported for SequenceDecoder")
    }

    fn symbol(&mut self) -> (u16, u16, u16) {
        let literals_code = self.literals_lengths_decoder.symbol();
        let offset_code = self.offsets_decoder.symbol();
        let match_code = self.match_lengths_decoder.symbol();
        (literals_code, offset_code, match_code)
    }

    fn update_bits(&mut self, bitstream: &mut BackwardBitParser) -> Result<bool, Error> {
        // update order: literals > offsets > match
        let mut zeroes = self.literals_lengths_decoder.update_bits(bitstream)?;
        zeroes |= self.match_lengths_decoder.update_bits(bitstream)?;
        zeroes |= self.offsets_decoder.update_bits(bitstream)?;
        Ok(zeroes)
    }

    fn reset(&mut self) {
        unimplemented!("reset not supported for SequenceDecoder")
    }
}

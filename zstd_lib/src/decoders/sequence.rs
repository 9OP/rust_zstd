use super::{BitDecoder, Error};

pub struct SequenceDecoder<'d> {
    literals_lengths_decoder: &'d mut dyn BitDecoder<u16>,
    offsets_decoder: &'d mut dyn BitDecoder<u16>,
    match_lengths_decoder: &'d mut dyn BitDecoder<u16>,
}

impl BitDecoder<(u16, u16, u16), Error> for SequenceDecoder<'_> {
    fn initialize(
        &mut self,
        bitstream: &mut crate::parsing::BackwardBitParser,
    ) -> Result<(), (u16, u16, u16)> {
        todo!()
    }

    fn expected_bits(&self) -> usize {
        todo!()
    }

    fn symbol(&mut self) -> Error {
        todo!()
    }

    fn update_bits(
        &mut self,
        bitstream: &mut crate::parsing::BackwardBitParser,
    ) -> Result<bool, (u16, u16, u16)> {
        todo!()
    }

    fn reset(&mut self) {
        todo!()
    }
}

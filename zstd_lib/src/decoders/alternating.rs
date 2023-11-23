use super::{
    BitDecoder,
    Error::{self},
    FseDecoder, FseTable, Symbol,
};
use crate::parsing::*;

pub struct AlternatingDecoder {
    decoder_1: FseDecoder,
    decoder_2: FseDecoder,
    last_used: bool,
}

impl AlternatingDecoder {
    pub fn new(fse_table: &FseTable) -> Self {
        Self {
            decoder_1: FseDecoder::new(fse_table.clone()),
            decoder_2: FseDecoder::new(fse_table.clone()),
            last_used: false,
        }
    }

    fn alternate(&mut self) {
        self.last_used = !self.last_used;
    }

    fn mut_decoder(&mut self) -> &mut FseDecoder {
        match self.last_used {
            true => &mut self.decoder_2,
            false => &mut self.decoder_1,
        }
    }

    fn decoder(&self) -> &FseDecoder {
        match self.last_used {
            true => &self.decoder_2,
            false => &self.decoder_1,
        }
    }
}

impl BitDecoder<Error, Symbol> for AlternatingDecoder {
    fn initialize(&mut self, bitstream: &mut BackwardBitParser) -> Result<(), Error> {
        self.decoder_1.initialize(bitstream)?;
        self.decoder_2.initialize(bitstream)?;
        Ok(())
    }

    fn expected_bits(&self) -> usize {
        self.decoder().expected_bits()
    }

    fn symbol(&mut self) -> Symbol {
        let symbol = self.mut_decoder().symbol();
        self.alternate();
        symbol
    }

    fn update_bits(&mut self, bitstream: &mut BackwardBitParser) -> Result<bool, Error> {
        self.mut_decoder().update_bits(bitstream)
    }

    fn reset(&mut self) {
        self.mut_decoder().reset()
    }
}
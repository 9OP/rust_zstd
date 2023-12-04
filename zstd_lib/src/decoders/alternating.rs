use super::{BackwardBitParser, BitDecoder, Error, FseDecoder, FseTable};

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
            false => &mut self.decoder_1,
            true => &mut self.decoder_2,
        }
    }

    fn decoder(&self) -> &FseDecoder {
        match self.last_used {
            false => &self.decoder_1,
            true => &self.decoder_2,
        }
    }
}

impl BitDecoder<u16, Error> for AlternatingDecoder {
    // // #[cfg(dev)]
    // fn debug(&self) {
    //     println!("{:?} {:?}", self.decoder_1, self.decoder_2)
    // }

    fn initialize(&mut self, bitstream: &mut BackwardBitParser) -> Result<(), Error> {
        self.decoder_1.initialize(bitstream)?;
        self.decoder_2.initialize(bitstream)?;
        Ok(())
    }

    fn expected_bits(&self) -> usize {
        self.decoder().expected_bits()
    }

    fn symbol(&mut self) -> u16 {
        let symbol = self.mut_decoder().symbol();
        symbol
    }

    fn update_bits(&mut self, bitstream: &mut BackwardBitParser) -> Result<bool, Error> {
        let zeroes = self.mut_decoder().update_bits(bitstream)?;
        self.alternate();
        Ok(zeroes)
    }

    fn reset(&mut self) {
        self.mut_decoder().reset()
    }
}

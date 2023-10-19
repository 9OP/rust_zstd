use super::error::{Error::*, Result};

#[derive(Debug)]
pub struct ForwardBitParser<'a> {
    bitstream: &'a [u8],
    pub position: usize,
}

impl<'a> ForwardBitParser<'a> {
    pub fn new(bitstream: &'a [u8]) -> Result<Self> {
        let (first_byte, rest) = bitstream.split_first().ok_or(NotEnoughBytes {
            requested: 1,
            available: 0,
        })?;

        // skip all initial 0 and the first 1
        // position 7 is MSB and position 0 is LSB: 0b7654_3210
        for i in 0..8 {
            if first_byte & (0b0000_0001 << i) != 0 {
                // first_byte = 0b1000_0000
                // in this case skip entire first_byte from the stream
                if i == 7 {
                    return Ok(Self {
                        bitstream: rest,
                        position: 0,
                    });
                }

                return Ok(Self {
                    bitstream,
                    position: i + 1, // skip first 1
                });
            }
        }

        Err(MalformedBitstream)
    }

    /// Return the number of bytes still unparsed
    pub fn len(&self) -> usize {
        self.bitstream.len()
    }

    /// Check if the input is exhausted
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    // fn available_bits(&mut self) -> usize {
    //     if self.is_empty() {
    //         return 0;
    //     }
    //     8 * (self.len() - 1) + self.position + 1
    // }

    // /// Get the given number of bits, or return an error.
    // fn take(&mut self, len: usize) -> Result<u64> {
    //     todo!()
    // }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        // update position, keep all bytes
        let bitstream: &[u8; 2] = &[0b0000_0110, 0b0111_0100];
        let parser = ForwardBitParser::new(bitstream).unwrap();
        assert_eq!(parser.bitstream, bitstream);
        assert_eq!(parser.position, 2);

        // skip first byte, move position to 0
        let bitstream: &[u8; 2] = &[0b1000_0000, 0b0111_0100];
        let parser = ForwardBitParser::new(bitstream).unwrap();
        assert_eq!(parser.bitstream, &[bitstream[1]]);
        assert_eq!(parser.position, 0);

        // ok on skipped bitstream
        let bitstream: &[u8; 1] = &[0b1000_0000];
        let parser = ForwardBitParser::new(bitstream).unwrap();
        assert_eq!(parser.bitstream, &[]);
        assert_eq!(parser.position, 0);

        // error on empty bitstream
        assert!(matches!(
            ForwardBitParser::new(&[]),
            Err(NotEnoughBytes {
                requested: 1,
                available: 0,
            })
        ));

        assert!(matches!(
            ForwardBitParser::new(&[0b0000_0000, 0b0111_0100]),
            Err(MalformedBitstream)
        ));
    }
}

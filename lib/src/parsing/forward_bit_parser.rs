use super::error::{Error::*, Result};

#[derive(Debug)]
pub struct ForwardBitParser<'a> {
    bitstream: &'a [u8],
    position: usize,
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

    fn available_bits(&mut self) -> usize {
        if self.is_empty() {
            return 0;
        }
        8 * (self.len() - 1) + (8 - self.position)
    }

    /// Get the given number of bits, or return an error.
    pub fn take(&mut self, len: usize) -> Result<u64> {
        if len == 0 {
            return Ok(0);
        }

        // The result contains at most 64 bits (u64)
        if len > 64 {
            return Err(LengthOverflow {
                length: len,
                range: 64,
            });
        }

        if len > self.available_bits() {
            return Err(NotEnoughBits {
                requested: len,
                available: self.available_bits(),
            });
        }

        // extract a subslice of requested bytes for number of bits to take
        let div_ceil_by_eight = |n| if n % 8 == 0 { n / 8 } else { (n / 8) + 1 };
        let requested_bytes = div_ceil_by_eight(len);
        let split = requested_bytes;
        let (slice, _) = self.bitstream.split_at(split);

        let mut result: u64 = 0;
        let mut bits_remaining = len;

        for byte in slice {
            // read up to 8-position per byte, position is in [0,7]
            let bits_to_read = std::cmp::min(bits_remaining, 8 - self.position);

            // apply position offset in order to discard RHS bits
            let offset = self.position;
            let bits = byte >> offset;

            // reverse bits order LSB<->MSB
            let bits = bits.reverse_bits();

            // read bits, shift in order to discard LHS bits
            let bits = bits >> (8 - bits_to_read);

            // shift result to make space for new bits
            result <<= bits_to_read;

            // merge read bits into result;
            result |= bits as u64;

            // update remaining bits count to read
            bits_remaining -= bits_to_read;

            // update position by adding bits read modulo u8
            self.position = (self.position + bits_to_read) % 8;

            // no more bits to read, exit
            if bits_remaining == 0 {
                break;
            }
        }

        // last byte has unread bits
        let include_last_byte = self.position != 0;
        let split = if include_last_byte { split - 1 } else { split };
        let (_, new_bitstream) = self.bitstream.split_at(split);
        self.bitstream = new_bitstream;
        // dbg!(self.bitstream, self.position);

        Ok(result)
    }
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

    #[test]
    fn test_take() {
        let bitstream: &[u8; 2] = &[0b1010_0110, 0b0111_0100];

        // large bits take error, by 1 bit
        let mut parser = ForwardBitParser::new(bitstream).unwrap();
        assert!(matches!(
            parser.take(65),
            Err(LengthOverflow {
                length: 65,
                range: 64
            })
        ));

        // not enough bits error, by 1 bit
        let mut parser = ForwardBitParser::new(bitstream).unwrap();
        assert!(matches!(
            parser.take(14 + 1),
            Err(NotEnoughBits {
                requested: 15,
                available: 14
            })
        ));

        // take bits and keep first byte
        let mut parser = ForwardBitParser::new(bitstream).unwrap();
        assert_eq!(parser.take(5).unwrap(), 0b10010);
        assert_eq!(parser.bitstream, bitstream);
        assert_eq!(parser.position, 7);

        // take bits an consume first byte
        let mut parser = ForwardBitParser::new(bitstream).unwrap();
        assert_eq!(parser.take(6).unwrap(), 0b100101);
        assert_eq!(parser.bitstream, &[bitstream[1]]);
        assert_eq!(parser.position, 0);

        // take all bits
        let mut parser = ForwardBitParser::new(bitstream).unwrap();
        assert_eq!(parser.take(14).unwrap(), 0b1001_0100_1011_10);
        assert_eq!(parser.bitstream, &[]);
        assert_eq!(parser.position, 0);
        assert_eq!(parser.take(0).unwrap(), 0);
        assert!(matches!(
            parser.take(1),
            Err(NotEnoughBits {
                requested: 1,
                available: 0
            })
        ));

        // apply multiple take
        let bitstream: &[u8; 2] = &[0b1010_0110, 0b0111_0100];
        let mut parser = ForwardBitParser::new(bitstream).unwrap();
        assert_eq!(parser.take(1).unwrap(), 0b1);
        assert_eq!(parser.take(1).unwrap(), 0b0);
        assert_eq!(parser.take(1).unwrap(), 0b0);
        assert_eq!(parser.take(1).unwrap(), 0b1);
        assert_eq!(parser.take(1).unwrap(), 0b0);
        assert_eq!(parser.take(1).unwrap(), 0b1);
        assert_eq!(parser.take(1).unwrap(), 0b0);
        assert_eq!(parser.take(1).unwrap(), 0b0);
        assert_eq!(parser.take(1).unwrap(), 0b1);
        assert_eq!(parser.take(1).unwrap(), 0b0);
        assert_eq!(parser.take(1).unwrap(), 0b1);
        assert_eq!(parser.take(1).unwrap(), 0b1);
        assert_eq!(parser.take(1).unwrap(), 0b1);
        assert_eq!(parser.take(1).unwrap(), 0b0);
        assert!(matches!(
            parser.take(1),
            Err(NotEnoughBits {
                requested: 1,
                available: 0
            })
        ));
        assert_eq!(parser.bitstream, &[]);
        assert_eq!(parser.position, 0);

        // parse only header
        let bitstream: &[u8; 1] = &[0b1000_0000];
        let mut parser = ForwardBitParser::new(bitstream).unwrap();
        assert!(matches!(
            parser.take(1),
            Err(NotEnoughBits {
                requested: 1,
                available: 0
            })
        ));

        // take 0 on valid non empty bitestream
        let bitstream: &[u8; 1] = &[0b1001_0000];
        let mut parser = ForwardBitParser::new(bitstream).unwrap();
        assert_eq!(parser.take(0).unwrap(), 0b0);
    }
}

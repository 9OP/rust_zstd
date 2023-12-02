use super::{Error::*, Result};

pub struct ForwardBitParser<'a> {
    bitstream: &'a [u8],
    position: usize,
}

impl<'a> ForwardBitParser<'a> {
    pub fn new(bitstream: &'a [u8]) -> Self {
        Self {
            bitstream,
            position: 0,
        }
    }

    /// Return the number of bytes still unparsed
    pub fn len(&self) -> usize {
        let include_first = self.position != 0;
        self.bitstream.len() - include_first as usize
    }

    /// Check if the input is exhausted
    pub fn is_empty(&self) -> bool {
        self.bitstream.len() == 0
    }

    /// Return the number of bits available wrt position
    pub fn available_bits(&self) -> usize {
        if self.is_empty() {
            return 0;
        }
        8 * (self.bitstream.len() - 1) + (8 - self.position)
    }

    /// Return the next bit without consumming it
    pub fn peek(&self) -> Result<u8> {
        let available_bits = self.available_bits();
        if 1 > available_bits {
            return Err(NotEnoughBits {
                requested: 1,
                available: available_bits,
            });
        }
        let is_bit_set = (self.bitstream[0] & (0x0000_0001 << self.position)) != 0;
        // Ok(is_bit_set as u8)
        Ok(if is_bit_set { 1 } else { 0 })
    }

    /// Get the given number of bits, or return an error.
    pub fn take(&mut self, len: usize) -> Result<u64> {
        if len == 0 {
            return Ok(0);
        }
        assert!(len <= 64, "unexpected len: {len} > 64");

        let available_bits = self.available_bits();
        if len > available_bits {
            return Err(NotEnoughBits {
                requested: len,
                available: available_bits,
            });
        }

        let stream = self.bitstream.iter();
        let mut result: u64 = 0;
        let mut bits_remaining = len;
        let mut byte_read = 0;

        for byte in stream {
            byte_read += 1;
            // read up to 8-position per byte, position is in [0,7]
            let bits_to_read = std::cmp::min(bits_remaining, 8 - self.position);
            let offset = self.position;

            // read bits, shift in order to discard LHS bits
            let bits = byte << (8 - bits_to_read - offset);

            // apply position offset in order to discard RHS bits
            let bits = bits >> (8 - bits_to_read);

            // merge read bits into result;
            result |= (bits as u64) << (len - bits_remaining);

            // update remaining bits count to read
            bits_remaining -= bits_to_read;

            // update position by adding bits read modulo u8
            self.position = (self.position + bits_to_read) % 8;

            // no more bits to read, exit
            if bits_remaining == 0 {
                break;
            }
        }

        // first byte has unread bits
        let include_first_byte = self.position != 0;
        let (_, new_bitstream) = self
            .bitstream
            .split_at(byte_read - include_first_byte as usize);
        self.bitstream = new_bitstream;

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let bitstream: &[u8; 2] = &[0b0000_0110, 0b0111_0100];
        let parser = ForwardBitParser::new(bitstream);
        assert_eq!(parser.bitstream, bitstream);
        assert_eq!(parser.position, 0);
    }

    #[test]
    fn test_len() {
        let bitstream: &[u8; 2] = &[0b1000_0001, 0b0111_0100];
        let mut parser = ForwardBitParser::new(bitstream);
        assert_eq!(parser.len(), 2);

        assert_eq!(parser.take(1).unwrap(), 0b1);
        assert_eq!(parser.len(), 1);
    }

    #[test]
    fn test_available_bits() {
        let bitstream: &[u8; 2] = &[0b1010_0110, 0b0111_0100];
        let mut parser = ForwardBitParser::new(bitstream);
        assert_eq!(parser.available_bits(), 16);
        let _ = parser.take(5);
        assert_eq!(parser.available_bits(), 16 - 5);
    }

    mod take {
        use super::*;

        #[test]
        #[should_panic(expected = "unexpected len: 65 > 64")]
        fn test_take_overflow() {
            let bitstream: &[u8; 2] = &[0b1010_0110, 0b0111_0100];
            let mut parser = ForwardBitParser::new(bitstream);
            let _ = parser.take(65);
        }

        #[test]
        fn test_take_not_enough_bits() {
            let bitstream: &[u8; 2] = &[0b1010_0110, 0b0111_0100];
            let mut parser = ForwardBitParser::new(bitstream);
            assert!(matches!(
                parser.take(16 + 1),
                Err(NotEnoughBits {
                    requested: 17,
                    available: 16
                })
            ));
        }

        #[test]
        fn test_take_keep_first_byte() {
            let bitstream: &[u8; 2] = &[0b1010_0110, 0b0111_0100];
            let mut parser = ForwardBitParser::new(bitstream);
            assert_eq!(parser.take(5).unwrap(), 0b00110);
            assert_eq!(parser.bitstream, bitstream);
            assert_eq!(parser.position, 5);
        }

        #[test]
        fn test_take_consumme_first_byte() {
            let bitstream: &[u8; 2] = &[0b1010_0110, 0b0111_0111];
            let mut parser = ForwardBitParser::new(bitstream);
            assert_eq!(parser.take(10).unwrap(), 0b11_1010_0110);
            assert_eq!(parser.bitstream, &[bitstream[1]]);
            assert_eq!(parser.position, 2);

            let bitstream: &[u8; 2] = &[0x30, 0x6F];
            let mut parser = ForwardBitParser::new(bitstream);
            assert_eq!(parser.take(4).unwrap(), 0);
            assert_eq!(parser.take(5).unwrap(), 19);

            let bitstream: &[u8; 3] = &[0b1010_0110, 0b0111_0111, 0b0011_1100];
            let mut parser = ForwardBitParser::new(bitstream);
            assert_eq!(parser.take(2).unwrap(), 0b10);
            assert_eq!(parser.bitstream, bitstream);
            assert_eq!(parser.position, 2);

            assert_eq!(parser.take(14).unwrap(), 0b0111_0111_1010_01);
            assert_eq!(parser.bitstream, &[bitstream[2]]);
            assert_eq!(parser.position, 0);
        }

        #[test]
        fn test_take_all_bits() {
            let bitstream: &[u8; 2] = &[0b1010_0110, 0b0111_0100];
            let mut parser = ForwardBitParser::new(bitstream);
            assert_eq!(parser.take(16).unwrap(), 0b0111_0100_1010_0110);
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
        }

        #[test]
        fn test_take_many() {
            let bitstream: &[u8; 2] = &[0b1010_0110, 0b0111_0100];
            let mut parser = ForwardBitParser::new(bitstream);
            assert_eq!(parser.peek().unwrap(), 0);
            assert_eq!(parser.take(1).unwrap(), 0);

            assert_eq!(parser.peek().unwrap(), 1);
            assert_eq!(parser.take(1).unwrap(), 1);

            assert_eq!(parser.peek().unwrap(), 1);
            assert_eq!(parser.take(1).unwrap(), 1);

            assert_eq!(parser.peek().unwrap(), 0);
            assert_eq!(parser.take(1).unwrap(), 0);

            assert_eq!(parser.peek().unwrap(), 0);
            assert_eq!(parser.take(1).unwrap(), 0);

            assert_eq!(parser.peek().unwrap(), 1);
            assert_eq!(parser.take(1).unwrap(), 1);

            assert_eq!(parser.peek().unwrap(), 0);
            assert_eq!(parser.take(1).unwrap(), 0);

            //
            assert_eq!(parser.peek().unwrap(), 1);
            assert_eq!(parser.take(1).unwrap(), 1);

            assert_eq!(parser.peek().unwrap(), 0);
            assert_eq!(parser.take(1).unwrap(), 0);

            assert_eq!(parser.peek().unwrap(), 0);
            assert_eq!(parser.take(1).unwrap(), 0);

            assert_eq!(parser.peek().unwrap(), 1);
            assert_eq!(parser.take(1).unwrap(), 1);

            assert_eq!(parser.peek().unwrap(), 0);
            assert_eq!(parser.take(1).unwrap(), 0);

            assert_eq!(parser.peek().unwrap(), 1);
            assert_eq!(parser.take(1).unwrap(), 1);

            assert_eq!(parser.peek().unwrap(), 1);
            assert_eq!(parser.take(1).unwrap(), 1);

            assert_eq!(parser.peek().unwrap(), 1);
            assert_eq!(parser.take(1).unwrap(), 1);

            assert_eq!(parser.peek().unwrap(), 0);
            assert_eq!(parser.take(1).unwrap(), 0);

            assert!(matches!(
                parser.peek(),
                Err(NotEnoughBits {
                    requested: 1,
                    available: 0
                })
            ));
            assert!(matches!(
                parser.take(1),
                Err(NotEnoughBits {
                    requested: 1,
                    available: 0
                })
            ));

            assert_eq!(parser.bitstream, &[]);
            assert_eq!(parser.position, 0);
        }

        #[test]
        fn test_take_zero() {
            let bitstream: &[u8; 1] = &[0b1111_1111];
            let mut parser = ForwardBitParser::new(bitstream);
            assert_eq!(parser.take(0).unwrap(), 0b0);
        }
    }
}

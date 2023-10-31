use super::error::{Error::*, Result};

#[derive(Debug)]
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
        self.bitstream.len()
    }

    /// Check if the input is exhausted
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn available_bits(&self) -> usize {
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
            let offset = self.position;

            // read bits, shift in order to discard LHS bits
            let bits = byte << (8 - bits_to_read - offset);

            // apply position offset in order to discard RHS bits
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
        let bitstream: &[u8; 2] = &[0b0000_0110, 0b0111_0100];
        let parser = ForwardBitParser::new(bitstream);
        assert_eq!(parser.bitstream, bitstream);
        assert_eq!(parser.position, 0);
    }

    #[test]
    fn test_len() {
        let bitstream: &[u8; 2] = &[0b1000_0000, 0b0111_0100];
        let parser = ForwardBitParser::new(bitstream);
        assert_eq!(parser.len(), 2);
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
        fn test_take_overflow() {
            let bitstream: &[u8; 2] = &[0b1010_0110, 0b0111_0100];
            let mut parser = ForwardBitParser::new(bitstream);
            assert!(matches!(
                parser.take(65),
                Err(LengthOverflow {
                    length: 65,
                    range: 64
                })
            ));
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
            let bitstream: &[u8; 2] = &[0b1010_0110, 0b0111_0100];
            let mut parser = ForwardBitParser::new(bitstream);
            assert_eq!(parser.take(8).unwrap(), 0b1010_0110);
            assert_eq!(parser.bitstream, &[bitstream[1]]);
            assert_eq!(parser.position, 0);
        }

        #[test]
        fn test_take_all_bits() {
            let bitstream: &[u8; 2] = &[0b1010_0110, 0b0111_0100];
            let mut parser = ForwardBitParser::new(bitstream);
            assert_eq!(parser.take(16).unwrap(), 0b1010_0110_0111_0100);
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
            assert_eq!(parser.take(1).unwrap(), 0b0);
            assert_eq!(parser.take(1).unwrap(), 0b1);
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
        }

        #[test]
        fn test_take_zero() {
            let bitstream: &[u8; 1] = &[0b1111_1111];
            let mut parser = ForwardBitParser::new(bitstream);
            assert_eq!(parser.take(0).unwrap(), 0b0);
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Not enough bytes: {requested:#06x} requested out of {available:#06x} available")]
    NotEnoughBytes { requested: usize, available: usize },

    #[error("Not enough bits: {requested} requested out of {available} available")]
    NotEnoughBits { requested: usize, available: usize },

    #[error("Bitstream header does not contain any '1'")]
    MalformedBitstream,

    #[error("Cannot take {requested} bits at once")]
    LargeBitsTake { requested: usize },
}

use Error::*;
type Result<T, E = Error> = std::result::Result<T, E>;

pub struct ForwardByteParser<'a>(&'a [u8]);

impl<'a> ForwardByteParser<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self(data)
    }

    /// Consume and return u8
    pub fn u8(&mut self) -> Result<u8> {
        let (first, rest) = self.0.split_first().ok_or(NotEnoughBytes {
            requested: 1,
            available: 0,
        })?;
        self.0 = rest;
        Ok(*first)
    }

    /// Return the number of bytes still unparsed
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Check if the input is exhausted
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Extract `len` bytes as a slice
    pub fn slice(&mut self, len: usize) -> Result<&'a [u8]> {
        match len <= self.len() {
            false => Err(NotEnoughBytes {
                requested: len,
                available: self.len(),
            }),
            true => {
                let (slice, rest) = self.0.split_at(len);
                self.0 = rest;
                Ok(slice)
            }
        }
    }

    /// Consume and return a u32 in little-endian format
    pub fn le_u32(&mut self) -> Result<u32> {
        // Consume 4bytes or Err. Do not consume partially !
        let byte_array = self.slice(4)?;
        let result = (byte_array[3] as u32) << 24
            | (byte_array[2] as u32) << 16
            | (byte_array[1] as u32) << 8
            | (byte_array[0] as u32);
        Ok(result.to_le())
    }
}

#[derive(Debug)]
pub struct BackwardBitParser<'a> {
    bitstream: &'a [u8],
    position: usize,
}

impl<'a> BackwardBitParser<'a> {
    pub fn new(bitstream: &'a [u8]) -> Result<Self> {
        let (last_byte, rest) = bitstream.split_last().ok_or(NotEnoughBytes {
            requested: 1,
            available: 0,
        })?;

        // skip all initial 0 and the first 1
        // position 7 is MSB and position 0 is LSB: 0b7654_3210
        for i in (0..8).rev() {
            if last_byte & (0b0000_0001 << i) != 0 {
                // last_byte = 0b0000_0001
                // in this case skip entire last_byte from the stream
                if i == 0 {
                    return Ok(Self {
                        bitstream: rest,
                        position: 7,
                    });
                }

                return Ok(Self {
                    bitstream,
                    position: i - 1, // skip first 1
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
        8 * (self.len() - 1) + self.position + 1
    }

    /// Get the given number of bits, or return an error.
    pub fn take(&mut self, len: usize) -> Result<u64> {
        // The result contains at most 64 bits (u64)
        if len > 64 {
            return Err(LargeBitsTake { requested: len });
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
        let split = self.len() - requested_bytes;
        let (_, slice) = self.bitstream.split_at(split);
        let slice: Vec<u8> = slice.iter().rev().cloned().collect();

        let mut result: u64 = 0;
        let mut bits_remaining = len;

        for byte in slice {
            // read up to position+1 per byte, position is in [0,7]
            let bits_to_read = std::cmp::min(bits_remaining, self.position + 1);

            // apply position offset in order to discard LHS bits
            let offset = 7 - self.position;
            let bits = byte << offset;

            // read bits, shift in order to discard RHS bits
            let bits = bits >> (8 - bits_to_read);

            // shift result to make space for new bits
            result <<= bits_to_read;

            // merge read bits into result;
            result |= bits as u64;

            // update remaining bits count to read
            bits_remaining -= bits_to_read;

            // update position
            if bits_to_read > self.position {
                // all byte's bits are read, reset position for next byte read
                self.position = 7;
            } else {
                // there are still unread bits in current byte, move position
                self.position -= bits_to_read;
            }

            // no more bits to read, exit
            if bits_remaining == 0 {
                break;
            }
        }

        // Last byte has unread bits
        let include_last_byte = self.position != 7;
        let (new_bitstream, _) = self
            .bitstream
            .split_at(split + if include_last_byte { 1 } else { 0 });
        self.bitstream = new_bitstream;

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod forward_byte_parser {
        use super::*;

        #[test]
        fn test_u8() {
            let mut parser = ForwardByteParser::new(&[0x12, 0x23, 0x34]);
            assert_eq!(parser.0.len(), 3);
            assert_eq!(parser.u8().unwrap(), 0x12);
            assert_eq!(parser.0.len(), 2);
            assert_eq!(parser.u8().unwrap(), 0x23);
            assert_eq!(parser.0.len(), 1);
            assert_eq!(parser.u8().unwrap(), 0x34);
            assert_eq!(parser.0.len(), 0);
            assert!(matches!(
                parser.u8(),
                Err(NotEnoughBytes {
                    requested: 1,
                    available: 0,
                })
            ));
        }

        #[test]
        fn test_len() {
            let parser = ForwardByteParser::new(&[0x12, 0x23, 0x34]);
            assert_eq!(parser.len(), 3);
            let parser = ForwardByteParser::new(&[0x12]);
            assert_eq!(parser.len(), 1);
            let parser = ForwardByteParser::new(&[]);
            assert_eq!(parser.len(), 0);
        }

        #[test]
        fn test_is_empty() {
            let parser = ForwardByteParser::new(&[0x12, 0x23, 0x34]);
            assert_eq!(false, parser.is_empty());
            let parser = ForwardByteParser::new(&[]);
            assert_eq!(true, parser.is_empty());
        }

        #[test]
        fn test_slice() {
            let mut parser = ForwardByteParser::new(&[0x12, 0x23, 0x34]);
            assert_eq!(&[] as &[u8], parser.slice(0).unwrap());
            assert_eq!(&[0x12, 0x23], parser.slice(2).unwrap());
            assert_eq!(1, parser.0.len());
            assert_eq!(&[0x34], parser.slice(1).unwrap());
            assert!(matches!(
                parser.slice(1),
                Err(NotEnoughBytes {
                    requested: 1,
                    available: 0,
                })
            ));
            let mut parser = ForwardByteParser::new(&[0x12, 0x23, 0x34]);
            assert!(matches!(
                parser.slice(4),
                Err(NotEnoughBytes {
                    requested: 4,
                    available: 3,
                })
            ));
            assert_eq!(3, parser.0.len());
            assert_eq!(&[0x12, 0x23, 0x34], parser.slice(3).unwrap());
            assert_eq!(0, parser.0.len());
        }

        #[test]
        fn test_le_u32() {
            let mut parser = ForwardByteParser::new(&[0x12, 0x34, 0x56, 0x78, 0xFF]);
            assert_eq!(5, parser.0.len());
            assert_eq!(0x78563412, parser.le_u32().unwrap());
            assert_eq!(1, parser.0.len());

            // Do not consume u8 when Error
            assert!(matches!(
                parser.le_u32(),
                Err(NotEnoughBytes {
                    requested: 4,
                    available: 1,
                })
            ));
            assert_eq!(1, parser.0.len());
        }
    }

    mod backward_bit_parser {
        use super::*;

        #[test]
        fn test_new() {
            // update position, keep all bytes
            let bitstream: &[u8; 2] = &[0b0011_1100, 0b0001_0111];
            let parser = BackwardBitParser::new(bitstream).unwrap();
            assert_eq!(parser.bitstream, bitstream);
            assert_eq!(parser.position, 3);

            // skip last byte, move position to 7
            let bitstream: &[u8; 2] = &[0b0011_1100, 0b0000_0001];
            let parser = BackwardBitParser::new(bitstream).unwrap();
            assert_eq!(parser.bitstream, &[bitstream[0]]);
            assert_eq!(parser.position, 7);

            // ok on skipped bitstream
            let bitstream: &[u8; 1] = &[0b0000_0001];
            let parser = BackwardBitParser::new(bitstream).unwrap();
            assert_eq!(parser.bitstream, &[]);
            assert_eq!(parser.position, 7);

            // error on empty bitstream
            assert!(matches!(
                BackwardBitParser::new(&[]),
                Err(NotEnoughBytes {
                    requested: 1,
                    available: 0,
                })
            ));

            assert!(matches!(
                BackwardBitParser::new(&[0b0011_1100, 0b0000_0000]),
                Err(MalformedBitstream)
            ));
        }

        #[test]
        fn test_take() {
            let bitstream: &[u8; 2] = &[0b0011_1100, 0b0001_0111];

            // large bits take error, by 1 bit
            let mut parser = BackwardBitParser::new(bitstream).unwrap();
            assert!(matches!(
                parser.take(65),
                Err(LargeBitsTake { requested: 65 })
            ));

            // not enough bits error, by 1 bit
            let mut parser = BackwardBitParser::new(bitstream).unwrap();
            assert!(matches!(
                parser.take(12 + 1),
                Err(NotEnoughBits {
                    requested: 13,
                    available: 12
                })
            ));

            // take bits and keep last byte
            let mut parser = BackwardBitParser::new(bitstream).unwrap();
            assert_eq!(parser.take(3).unwrap(), 0b011);
            assert_eq!(parser.bitstream, bitstream);
            assert_eq!(parser.position, 0);

            // take bits an consume last byte
            let mut parser = BackwardBitParser::new(bitstream).unwrap();
            assert_eq!(parser.take(10).unwrap(), 0b0111_0011_11);
            assert_eq!(parser.bitstream, &[bitstream[0]]);
            assert_eq!(parser.position, 1);

            // take all bits
            let mut parser = BackwardBitParser::new(bitstream).unwrap();
            assert_eq!(parser.take(12).unwrap(), 0b0111_0011_1100);
            assert_eq!(parser.bitstream, &[]);
            assert_eq!(parser.position, 7);
            assert_eq!(parser.take(0).unwrap(), 0);
            assert!(matches!(
                parser.take(1),
                Err(NotEnoughBits {
                    requested: 1,
                    available: 0
                })
            ));

            // apply multiple take
            let mut parser = BackwardBitParser::new(bitstream).unwrap();
            assert_eq!(parser.take(1).unwrap(), 0b0);
            assert_eq!(parser.take(1).unwrap(), 0b1);
            assert_eq!(parser.take(1).unwrap(), 0b1);
            assert_eq!(parser.take(1).unwrap(), 0b1);
            assert_eq!(parser.take(1).unwrap(), 0b0);
            assert_eq!(parser.take(1).unwrap(), 0b0);
            assert_eq!(parser.take(1).unwrap(), 0b1);
            assert_eq!(parser.take(1).unwrap(), 0b1);
            assert_eq!(parser.take(1).unwrap(), 0b1);
            assert_eq!(parser.take(1).unwrap(), 0b1);
            assert_eq!(parser.take(1).unwrap(), 0b0);
            assert_eq!(parser.take(1).unwrap(), 0b0);
            assert!(matches!(
                parser.take(1),
                Err(NotEnoughBits {
                    requested: 1,
                    available: 0
                })
            ));
            assert_eq!(parser.bitstream, &[]);
            assert_eq!(parser.position, 7);
        }
    }
}

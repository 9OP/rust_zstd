use super::{Error, Result};

pub struct BackwardBitParser<'a> {
    bitstream: &'a [u8],
    position: usize,
}

impl<'a> BackwardBitParser<'a> {
    /// Create a new `BackwardBitParser` instance from a byte slice
    /// or return `NotEnoughByte` error when the byte slice is empty.
    /// The parser is initialized skipping all 0 and the first 1 from MSB.
    /// # Example
    /// ```
    /// # use zstd_lib::parsing::{BackwardBitParser, ParsingError};
    /// let mut parser = BackwardBitParser::new(&[0b0001_1010, 0b0110_0000])?;
    /// // stream: 0b0001_1010, 0b0011_0000
    /// //                        --^ skipped initial zeroes and first one from MSB to LSB
    /// # Ok::<(), ParsingError>(())
    /// ```
    pub fn new(bitstream: &'a [u8]) -> Result<Self> {
        let (last_byte, rest) = bitstream.split_last().ok_or(Error::NotEnoughBytes {
            requested: 1,
            available: 0,
        })?;

        // skip all initial 0 and the first 1 from
        // from position 7 (MSB) to position 0 (LSB): 0b7654_3210
        for i in (0..8).rev() {
            if (last_byte & (1 << i)) != 0 {
                if i == 0 {
                    // last_byte = 0b0000_0001
                    // in this case skip entire last_byte from the stream
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

        Err(Error::MalformedBitstream)
    }

    /// Return the number of bytes still unparsed.
    /// **Note**: partially parsed byte are **not** included.
    /// # Example
    /// ```
    /// # use zstd_lib::parsing::{BackwardBitParser, ParsingError};
    /// let mut parser = BackwardBitParser::new(&[0b0001_1010, 0b0110_0000])?;
    /// assert_eq!(parser.len(), 1);    // 2nd byte is partially parsed
    /// parser.take(6)?;                // consume all bits of 2nd byte
    /// assert_eq!(parser.len(), 1);    // 2nd byte fully parsed
    /// parser.take(1)?;                // consume 1st bit of 1st byte
    /// assert_eq!(parser.len(), 0);    // 1st byte partially parsed
    /// # Ok::<(), ParsingError>(())
    /// ```
    #[allow(dead_code)]
    #[must_use]
    pub fn len(&self) -> usize {
        let include_last = self.position == 7;
        self.bitstream.len() - 1 + usize::from(include_last)
    }

    /// Check if the bitstream is exhausted
    /// # Example
    /// ```
    /// # use zstd_lib::parsing::{BackwardBitParser, ParsingError};
    /// let mut parser = BackwardBitParser::new(&[0b0000_0001])?; // creates an empty parser
    /// assert_eq!(parser.is_empty(), true);
    /// # Ok::<(), ParsingError>(())
    /// ```
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bitstream.len() == 0
    }

    /// Return the number of available bits in the parser
    /// # Example
    /// ```
    /// # use zstd_lib::parsing::{BackwardBitParser, ParsingError};
    /// let mut parser = BackwardBitParser::new(&[0b0100_1010])?;
    /// // stream: 0b0100_1010
    /// //           --^ skip the first 0 and first 1
    /// assert_eq!(parser.available_bits(), 6);
    /// # Ok::<(), ParsingError>(())
    /// ```
    #[must_use]
    pub fn available_bits(&self) -> usize {
        if self.is_empty() {
            return 0;
        }
        8 * (self.bitstream.len() - 1) + self.position + 1
    }

    /// Return a u64 made of `len` bits read backward: MSB to LSB and last byte to first byte.
    /// Returns an error when `len > available_bits`
    /// # Panic
    /// Panics when `len > 64` for obvious reason.
    /// # Example
    /// ```
    /// # use zstd_lib::parsing::{BackwardBitParser, ParsingError};
    /// let mut parser = BackwardBitParser::new(&[0b0111_1011])?;
    /// // stream: 0b0111_1011
    /// //           --^ skip the first 0 and first 1
    /// assert_eq!(parser.take(2)?, 0b11);
    /// assert_eq!(parser.take(1)?, 0b1);
    /// assert_eq!(parser.take(3)?, 0b011);
    /// # Ok::<(), ParsingError>(())
    /// ```
    pub fn take(&mut self, len: usize) -> Result<u64> {
        if len == 0 {
            return Ok(0);
        }
        let available_bits = std::cmp::min(self.available_bits(), 64);
        if len > available_bits {
            return Err(Error::NotEnoughBits {
                requested: len,
                available: available_bits,
            });
        }

        let reversed_stream = self.bitstream.iter().rev();
        let mut result: u64 = 0;
        let mut bits_remaining = len;
        let mut byte_read = 0;

        for byte in reversed_stream {
            byte_read += 1;
            // read up to position+1 per byte, position is in [0,7]
            let bits_to_read = std::cmp::min(bits_remaining, self.position + 1);

            // apply position offset in order to discard left-hand-side bits
            let offset = 7 - self.position;
            let bits = byte << offset;

            // read bits, shift in order to discard right-hand-side bits
            let bits = bits >> (8 - bits_to_read);

            // shift result to make space for new bits
            result <<= bits_to_read;

            // merge read bits into result;
            result |= u64::from(bits);

            // update remaining bits count to read
            bits_remaining -= bits_to_read;

            // update position by removing bits read modulo u8
            // (+8 is a trick to prevent int substrack overflow)
            self.position = ((self.position + 8) - bits_to_read) % 8;

            // no more bits to read, exit
            if bits_remaining == 0 {
                break;
            }
        }

        // Last byte has unread bits
        let include_last_byte = self.position != 7;
        let remaining_bytes = self.bitstream.len() - byte_read;
        let (new_bitstream, _) = self
            .bitstream
            .split_at(remaining_bytes + usize::from(include_last_byte));
        self.bitstream = new_bitstream;

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod new {
        use super::*;

        #[test]
        fn test_new_keep_bytes() {
            // update position, keep all bytes
            let bitstream: &[u8; 2] = &[0b0011_1100, 0b0001_0111];
            let parser = BackwardBitParser::new(bitstream).unwrap();
            assert_eq!(parser.bitstream, bitstream);
            assert_eq!(parser.position, 3);
        }

        #[test]
        fn test_new_skip_byte() {
            // skip last byte, move position to 7
            let bitstream: &[u8; 2] = &[0b0011_1100, 0b0000_0001];
            let parser = BackwardBitParser::new(bitstream).unwrap();
            assert_eq!(parser.bitstream, &[bitstream[0]]);
            assert_eq!(parser.position, 7);
        }

        #[test]
        fn test_new_skip_stream() {
            let bitstream: &[u8; 1] = &[0b0000_0001];
            let parser = BackwardBitParser::new(bitstream).unwrap();
            assert_eq!(parser.bitstream, &[]);
            assert_eq!(parser.position, 7);
        }

        #[test]
        fn test_new_empty_header() {
            assert!(matches!(
                BackwardBitParser::new(&[]),
                Err(Error::NotEnoughBytes {
                    requested: 1,
                    available: 0,
                })
            ));
        }

        #[test]
        fn test_new_malformed_header() {
            assert!(matches!(
                BackwardBitParser::new(&[0b0011_1100, 0b0000_0000]),
                Err(Error::MalformedBitstream)
            ));
        }
    }

    #[test]
    fn test_len() {
        let bitstream: &[u8; 2] = &[0b0011_1100, 0b0000_0110];
        let mut parser = BackwardBitParser::new(bitstream).unwrap();
        assert_eq!(parser.len(), 1);
        parser.take(2).unwrap();
        assert_eq!(parser.len(), 1);
        parser.take(1).unwrap();
        assert_eq!(parser.len(), 0);
    }

    #[test]
    fn test_available_bits() {
        let bitstream: &[u8; 2] = &[0b0011_1100, 0b0000_0001];
        let parser = BackwardBitParser::new(bitstream).unwrap();
        assert_eq!(parser.available_bits(), 8);

        let parser = BackwardBitParser::new(&[0b0000_0001]).unwrap();
        assert_eq!(parser.is_empty(), true);
        assert_eq!(parser.available_bits(), 0);
    }

    mod take {
        use super::*;

        #[test]
        fn test_take_overflow() {
            let bitstream: &[u8; 2] = &[0b0011_1100, 0b0001_0111];
            let mut parser = BackwardBitParser::new(bitstream).unwrap();
            assert!(matches!(
                parser.take(65),
                Err(Error::NotEnoughBits {
                    requested: 65,
                    available: 12
                })
            ));

            let bitstream = &[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
            let mut parser = BackwardBitParser::new(bitstream).unwrap();
            assert!(matches!(
                parser.take(65),
                Err(Error::NotEnoughBits {
                    requested: 65,
                    available: 64
                })
            ));
        }

        #[test]
        fn test_take_not_enough_bits() {
            let bitstream: &[u8; 2] = &[0b0011_1100, 0b0001_0111];
            let mut parser = BackwardBitParser::new(bitstream).unwrap();
            assert!(matches!(
                parser.take(12 + 1),
                Err(Error::NotEnoughBits {
                    requested: 13,
                    available: 12
                })
            ));
        }

        #[test]
        fn test_take_keep_last_byte() {
            let bitstream: &[u8; 2] = &[0b0011_1100, 0b0001_0111];
            let mut parser = BackwardBitParser::new(bitstream).unwrap();
            assert_eq!(parser.take(3).unwrap(), 0b011);
            assert_eq!(parser.bitstream, bitstream);
            assert_eq!(parser.position, 0);
        }

        #[test]
        fn test_take_consumme_last_byte() {
            let bitstream: &[u8; 2] = &[0b0011_1100, 0b0001_0111];
            let mut parser = BackwardBitParser::new(bitstream).unwrap();
            assert_eq!(parser.take(10).unwrap(), 0b0111_0011_11);
            assert_eq!(parser.bitstream, &[bitstream[0]]);
            assert_eq!(parser.position, 1);

            let bitstream: &[u8; 2] = &[0b1101_1001, 0b0000_0100];
            let mut parser = BackwardBitParser::new(bitstream).unwrap();
            assert_eq!(parser.take(6).unwrap(), 0b001101);
            assert_eq!(parser.bitstream, &[bitstream[0]]);
            assert_eq!(parser.position, 3);
        }

        #[test]
        fn test_take_all_bits() {
            let bitstream: &[u8; 2] = &[0b0011_1100, 0b0001_0111];
            let mut parser = BackwardBitParser::new(bitstream).unwrap();
            assert_eq!(parser.take(12).unwrap(), 0b0111_0011_1100);
            assert_eq!(parser.bitstream, &[]);
            assert_eq!(parser.position, 7);
            assert_eq!(parser.take(0).unwrap(), 0);
            assert!(matches!(
                parser.take(1),
                Err(Error::NotEnoughBits {
                    requested: 1,
                    available: 0
                })
            ));
        }

        #[test]
        fn test_take_many() {
            let bitstream: &[u8; 2] = &[0b0011_1100, 0b0001_0111];
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
                Err(Error::NotEnoughBits {
                    requested: 1,
                    available: 0
                })
            ));
            assert_eq!(parser.bitstream, &[]);
            assert_eq!(parser.position, 7);
        }

        #[test]
        fn test_take_header_only() {
            let bitstream: &[u8; 1] = &[0b000_0001];
            let mut parser = BackwardBitParser::new(bitstream).unwrap();
            assert!(matches!(
                parser.take(1),
                Err(Error::NotEnoughBits {
                    requested: 1,
                    available: 0
                })
            ));
        }

        #[test]
        fn test_take_zero() {
            let bitstream: &[u8; 1] = &[0b1001_0000];
            let mut parser = BackwardBitParser::new(bitstream).unwrap();
            assert_eq!(parser.take(0).unwrap(), 0b0);
        }
    }
}

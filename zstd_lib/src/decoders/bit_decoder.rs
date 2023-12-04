use super::BackwardBitParser;

/// A (possibly) stateful bit-level decoder
pub trait BitDecoder<T, E> {
    // #[cfg(dev)]
    fn debug(&self);

    /// Initialize the state.
    ///
    /// # Panics
    ///
    /// This method may panic if the decoder is already initialized.
    fn initialize(&mut self, bitstream: &mut BackwardBitParser) -> Result<(), E>;

    /// Return the next expected input size in bits
    ///
    /// # Panics
    ///
    /// This method may panic if no bits are expected right now
    fn expected_bits(&self) -> usize;

    /// Retrieve a decoded symbol
    ///
    /// # Panics
    ///
    /// This method may panic if the state has not been updated
    /// since the last state retrieval.
    fn symbol(&mut self) -> T;

    /// Update the state from a bitstream by reading the right
    /// number of bits, silently completing with zeroes if needed.
    /// Return `true` if zeroes have been added.
    ///
    /// # Panics
    ///
    /// This method may panic if the symbol has not been retrieved since
    /// the last update.
    fn update_bits(&mut self, bitstream: &mut BackwardBitParser) -> Result<bool, E>;

    /// Reset the table at its state before `initialize` is called. It allows
    /// reusing the same decoder.
    fn reset(&mut self);
}

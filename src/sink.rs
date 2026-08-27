//! Where serialized bytes go.
//!
//! [`Sink`] abstracts the write destination so the same generated `write` method can serve a
//! fixed stack buffer on an MCU, a `Vec<u8>`, or a file - no allocator or `std` required unless
//! the destination itself needs one.
//!
//! The associated `Error` type is the point of the design. `Vec<u8>` cannot fail, so its impl
//! uses [`Infallible`](core::convert::Infallible) - which makes the whole `alloc` write path
//! statically infallible, and lets callers of an infallible write discard the result without
//! pretending to handle an error that cannot occur.

use crate::error::WriteError;

/// A byte destination.
pub trait Sink {
    /// How a write can fail. [`Infallible`](core::convert::Infallible) when it cannot.
    type Error;

    /// Append `bytes`.
    ///
    /// # Errors
    ///
    /// Implementation-defined; see [`Sink::Error`].
    fn write(&mut self, bytes: &[u8]) -> Result<(), Self::Error>;
}

/// Writes into a fixed caller-supplied buffer. The `no_std` sink.
///
/// Pair it with the `lcf_size` method the schema macro generates to size the buffer exactly.
#[derive(Debug)]
pub struct SliceSink<'a> {
    buf: &'a mut [u8],
    len: usize,
}

impl<'a> SliceSink<'a> {
    /// Wrap a buffer.
    #[must_use]
    pub fn new(buf: &'a mut [u8]) -> Self {
        Self { buf, len: 0 }
    }

    /// Bytes written so far.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Whether nothing has been written.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The written prefix.
    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        &self.buf[..self.len]
    }

    /// Consume the sink and return the written prefix.
    #[must_use]
    pub fn into_slice(self) -> &'a [u8] {
        &self.buf[..self.len]
    }
}

impl Sink for SliceSink<'_> {
    type Error = WriteError;

    #[inline]
    fn write(&mut self, bytes: &[u8]) -> Result<(), WriteError> {
        let end = self.len.checked_add(bytes.len()).ok_or(WriteError::BufferFull)?;
        let dst = self.buf.get_mut(self.len..end).ok_or(WriteError::BufferFull)?;
        dst.copy_from_slice(bytes);
        self.len = end;
        Ok(())
    }
}

/// Counts bytes without storing them.
///
/// A cross-check for the generated `lcf_size` methods: writing into a `CountSink` must produce
/// exactly what `lcf_size` predicted, or the two-pass writer would emit wrong chunk lengths.
/// Cheap enough to assert on in tests for every fixture.
#[derive(Clone, Copy, Debug, Default)]
pub struct CountSink {
    len: u64,
}

impl CountSink {
    /// A fresh counter.
    #[must_use]
    pub const fn new() -> Self {
        Self { len: 0 }
    }

    /// Bytes that would have been written.
    #[must_use]
    pub const fn len(&self) -> u64 {
        self.len
    }

    /// Whether nothing would have been written.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl Sink for CountSink {
    type Error = core::convert::Infallible;

    #[inline]
    fn write(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        self.len += bytes.len() as u64;
        Ok(())
    }
}

#[cfg(feature = "alloc")]
impl Sink for alloc::vec::Vec<u8> {
    type Error = core::convert::Infallible;

    #[inline]
    fn write(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        self.extend_from_slice(bytes);
        Ok(())
    }
}

/// Adapts any [`std::io::Write`] into a [`Sink`].
#[cfg(feature = "std")]
#[derive(Debug)]
pub struct IoSink<W>(pub W);

#[cfg(feature = "std")]
impl<W: std::io::Write> Sink for IoSink<W> {
    type Error = std::io::Error;

    #[inline]
    fn write(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        self.0.write_all(bytes)
    }
}

//! Fatal errors.
//!
//! A [`ReadError`] means parsing *stopped* (input too short for even a header); a
//! [`Diagnostic`](crate::diag::Diagnostic) means parsing recovered and went on. Both are `Copy`
//! and hold only offsets and `&'static str`, so they work in the core tier with no allocator.

use thiserror::Error;

/// Something went wrong badly enough that parsing could not continue.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum ReadError {
    /// The buffer ended in the middle of a value that must be present.
    #[error("unexpected end of input at offset {offset}")]
    UnexpectedEof {
        /// File-absolute offset where the input ran out.
        offset: u32,
    },
    /// The file's magic string is not the one this loader expects.
    ///
    /// Carries the expected magic; the observed bytes stay in the caller's buffer.
    #[error("bad magic at offset {offset}: expected {expected:?}")]
    BadMagic {
        /// The magic string this loader requires.
        expected: &'static str,
        /// File-absolute offset the magic was read from.
        offset: u32,
    },
    /// A compressed integer ran past the format's 5-byte cap.
    #[error("compressed integer at offset {offset} exceeds 5 bytes")]
    VarintOverflow {
        /// File-absolute offset of the integer's first byte.
        offset: u32,
    },
    /// A chunk declared a length extending past the end of its parent.
    #[error("chunk 0x{chunk_id:02x} at offset {offset} declares {len} bytes, past end of parent")]
    ChunkOverrun {
        /// File-absolute offset of the chunk's payload.
        offset: u32,
        /// The chunk's field id.
        chunk_id: u32,
        /// The length the chunk declared.
        len: u32,
    },
    /// Strict mode was requested and a recoverable problem was found.
    ///
    /// The payload is deliberately flattened to the diagnostic's own fields so this stays `Copy`.
    #[error("strict mode: {0}")]
    Strict(crate::diag::Diagnostic),
}

/// Something went wrong while serialising.
///
/// One variant, unreachable when writing into a `Vec<u8>` (its [`Sink`](crate::sink::Sink) impl
/// uses `Infallible`).
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum WriteError {
    /// A fixed-size [`SliceSink`](crate::sink::SliceSink) ran out of room.
    #[error("output buffer full")]
    BufferFull,
}

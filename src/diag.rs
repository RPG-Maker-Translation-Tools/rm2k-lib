//! Recoverable problems found while parsing.
//!
//! Real RPG Maker 2000/2003 files in the wild are frequently malformed - a corrupt chunk, a
//! malformed compressed integer, an unknown field id. Parsing recovers from all of these and
//! keeps going rather than rejecting the file, but reports what happened instead of silently
//! swallowing it.
//!
//! Every recovery point pushes a [`Diagnostic`] into a caller-supplied [`DiagSink`]. Which sink
//! you pass decides the policy, at zero runtime cost when you don't care:
//!
//! | Sink | Behaviour |
//! |---|---|
//! | [`NullSink`] | Ignore everything. Monomorphises away entirely. |
//! | [`SliceSink`] | Record into a fixed buffer. The `no_std` option. |
//! | [`StrictSink`] | Latch the first problem; the loader turns it into `Err`. |
//! | `Vec<Diagnostic>` | Record everything. Requires `alloc`. |

use core::fmt;

/// What kind of recoverable problem was found.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum DiagKind {
    /// A chunk id this struct has no field for. Extremely common and usually benign: it is how
    /// RPG Maker's own forward-compatibility works.
    ///
    /// The chunk is **kept**, not dropped - it lands in the owning struct's
    /// [`unknown_chunks`](crate::field::LcfStruct::unknown_chunks) and is written back in the
    /// same position, so an unrecognised id does not cost byte-exactness. This is still reported
    /// because preserved is not understood: a caller that wants to reject such a file passes
    /// [`StrictSink`].
    UnknownChunk,
    /// A chunk's payload was shorter than the field needed. The field keeps its default.
    ShortChunk,
    /// A field did not consume its whole chunk payload. The remainder is dropped.
    TrailingBytes,
    /// A compressed integer exceeded the 5-byte cap. Substituted with `0`, as liblcf does.
    BadVarint,
    /// A chunk declared a length running past the end of its parent. Truncated to what is left.
    ChunkOverrun,
    /// The file's magic string did not match. liblcf warns and parses anyway
    /// so this is recoverable rather than fatal.
    BadMagic,
    /// An event command list was not terminated by the expected four zero bytes.
    EventListCorrupt,
    /// A raw primitive array's payload length was not a whole multiple of the element size.
    RaggedArray,
}

impl DiagKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UnknownChunk => "unknown chunk",
            Self::ShortChunk => "chunk too short for field",
            Self::TrailingBytes => "trailing bytes in chunk",
            Self::BadVarint => "malformed compressed integer",
            Self::ChunkOverrun => "chunk length past end of parent",
            Self::BadMagic => "unexpected file magic",
            Self::EventListCorrupt => "event command list not terminated",
            Self::RaggedArray => "array payload not a multiple of element size",
        }
    }
}

/// One recoverable problem, located.
///
/// Kept `Copy` and allocation-free on purpose: `struct_name` is always a `&'static str` baked in
/// by the schema macro, never a formatted string.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    /// Byte offset into the whole input buffer.
    pub offset: u32,
    /// Chunk id involved, or `0` when the problem is not tied to one.
    pub chunk_id: u32,
    pub struct_name: &'static str,
    pub kind: DiagKind,
}

impl Diagnostic {
    /// A filler value for declaring a [`SliceSink`] buffer without an allocator.
    ///
    /// `SliceSink` never reads a slot it has not written, so the contents are irrelevant; this
    /// exists only so `[Diagnostic::EMPTY; N]` is spellable in a `no_std` caller.
    pub const EMPTY: Self = Self {
        offset: 0,
        chunk_id: 0,
        struct_name: "",
        kind: DiagKind::UnknownChunk,
    };
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {} (chunk 0x{:02x}, offset {})",
            self.struct_name,
            self.kind.as_str(),
            self.chunk_id,
            self.offset
        )
    }
}

/// Where recovered problems go.
///
/// Implementors must not panic and must not allocate unless they are `alloc`-gated.
pub trait DiagSink {
    /// Record one problem.
    fn push(&mut self, diag: Diagnostic);

    /// Whether the loader should abort at the next opportunity.
    ///
    /// Only [`StrictSink`] ever returns `true`; the default lets the optimiser delete every
    /// check for the other sinks.
    #[inline]
    fn should_abort(&self) -> bool {
        false
    }

    /// The latched problem, for a sink that aborts.
    #[inline]
    fn first(&self) -> Option<Diagnostic> {
        None
    }
}

/// Discards every diagnostic. The default, and free - it compiles to nothing.
#[derive(Clone, Copy, Debug, Default)]
pub struct NullSink;

impl DiagSink for NullSink {
    #[inline]
    fn push(&mut self, _diag: Diagnostic) {}
}

/// Records into a caller-supplied fixed buffer. Overflow drops the excess and bumps
/// [`dropped`](SliceSink::dropped) rather than failing - a diagnostic sink that can itself fail
/// would be a poor kind of diagnostic sink.
#[derive(Debug)]
pub struct SliceSink<'b> {
    slots: &'b mut [Diagnostic],
    len: usize,
    dropped: u32,
}

impl<'b> SliceSink<'b> {
    /// Wrap a buffer.
    #[must_use]
    pub fn new(slots: &'b mut [Diagnostic]) -> Self {
        Self {
            slots,
            len: 0,
            dropped: 0,
        }
    }

    /// The diagnostics recorded so far.
    #[must_use]
    pub fn as_slice(&self) -> &[Diagnostic] {
        &self.slots[..self.len]
    }

    /// How many diagnostics did not fit.
    #[must_use]
    pub const fn dropped(&self) -> u32 {
        self.dropped
    }
}

impl DiagSink for SliceSink<'_> {
    #[inline]
    fn push(&mut self, diag: Diagnostic) {
        if self.len < self.slots.len() {
            self.slots[self.len] = diag;
            self.len += 1;
        } else {
            self.dropped = self.dropped.saturating_add(1);
        }
    }
}

/// Latches the first diagnostic and asks the loader to stop. This is what "strict mode" is -
/// no separate parser, just a different sink.
#[derive(Clone, Copy, Debug, Default)]
pub struct StrictSink {
    first: Option<Diagnostic>,
}

impl StrictSink {
    /// A fresh sink with nothing latched.
    #[must_use]
    pub const fn new() -> Self {
        Self { first: None }
    }
}

impl DiagSink for StrictSink {
    #[inline]
    fn push(&mut self, diag: Diagnostic) {
        if self.first.is_none() {
            self.first = Some(diag);
        }
    }

    #[inline]
    fn should_abort(&self) -> bool {
        self.first.is_some()
    }

    #[inline]
    fn first(&self) -> Option<Diagnostic> {
        self.first
    }
}

#[cfg(feature = "alloc")]
impl DiagSink for alloc::vec::Vec<Diagnostic> {
    #[inline]
    fn push(&mut self, diag: Diagnostic) {
        Self::push(self, diag);
    }
}

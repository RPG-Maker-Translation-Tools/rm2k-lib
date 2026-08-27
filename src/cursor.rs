//! Position tracking and chunk framing over a borrowed byte slice.
//!
//! Every LCF chunk is length-prefixed, so [`ChunkIter`] can slice out each field's payload as a
//! `&'a [u8]` bounded to exactly that chunk before the field ever sees it. A field that misreads
//! its own payload cannot walk into the next chunk's bytes, because they are not reachable from
//! where it is looking - malformed input is contained, not just detected after the fact.
//!
//! Every read goes through `slice::get`, returning `None` at the end of input. There is no
//! `get_unchecked` and no `transmute` anywhere in the decode path: the input is untrusted, and a
//! mispredicted branch costs less than a CVE.

use crate::{
    diag::{DiagKind, DiagSink, Diagnostic},
    engine::Engine,
    wire::{self, Varint, Varint64},
};

/// Parse-time context threaded through every read: the target engine, and where to report
/// recoverable problems.
///
/// The sink is `dyn` rather than a generic parameter on purpose. Diagnostics are the cold path,
/// so the indirect call costs nothing measurable, while monomorphising ~70 generated `read`
/// methods across every sink type would multiply code size for no gain. marshal-rs makes the
/// same trade for its symbol table.
pub struct Ctx<'d> {
    pub engine: Engine,
    diag: &'d mut dyn DiagSink,
}

impl<'d> Ctx<'d> {
    pub fn new(engine: Engine, diag: &'d mut dyn DiagSink) -> Self {
        Self { engine, diag }
    }

    /// Reborrow for a nested struct.
    ///
    /// Schema recursion depth is fixed by the format (four levels at most, e.g. `Database` ->
    /// `Troop` -> `TroopPage` -> `EventCommand`), so recursive descent is safe here in a way it
    /// would not be for an attacker-controlled shape.
    #[inline]
    pub fn reborrow(&mut self) -> Ctx<'_> {
        Ctx {
            engine: self.engine,
            diag: &mut *self.diag,
        }
    }

    /// Record a recoverable problem.
    #[inline]
    pub fn report(&mut self, struct_name: &'static str, chunk_id: u32, offset: u32, kind: DiagKind) {
        self.diag.push(Diagnostic {
            offset,
            chunk_id,
            struct_name,
            kind,
        });
    }

    /// Whether the sink has asked parsing to stop (strict mode).
    #[must_use]
    #[inline]
    pub fn should_abort(&self) -> bool {
        self.diag.should_abort()
    }

    /// The latched diagnostic, if the sink latches one.
    #[must_use]
    #[inline]
    pub fn latched(&self) -> Option<Diagnostic> {
        self.diag.first()
    }
}

impl core::fmt::Debug for Ctx<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Ctx")
            .field("engine", &self.engine)
            .finish_non_exhaustive()
    }
}

/// A position within a borrowed buffer.
///
/// `base` is the offset of `buf[0]` within the *whole* file, so a cursor over a sliced-out chunk
/// payload still reports file-absolute offsets in diagnostics.
#[derive(Clone, Debug)]
pub struct Cursor<'a> {
    buf: &'a [u8],
    pos: usize,
    base: u32,
}

impl<'a> Cursor<'a> {
    /// A cursor over a whole file.
    #[must_use]
    #[inline]
    pub const fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0, base: 0 }
    }

    /// A cursor over a slice that begins `base` bytes into the file.
    #[must_use]
    #[inline]
    pub const fn with_base(buf: &'a [u8], base: u32) -> Self {
        Self { buf, pos: 0, base }
    }

    /// Bytes consumed so far.
    #[must_use]
    #[inline]
    pub const fn pos(&self) -> usize {
        self.pos
    }

    /// File-absolute offset of the current position.
    #[must_use]
    #[inline]
    pub const fn offset(&self) -> u32 {
        self.base.wrapping_add(self.pos as u32)
    }

    /// The whole buffer, consumed part included.
    ///
    /// Needed to slice out a span the cursor has already walked past - the event-command codec
    /// scans its parameters to find their extent, then hands back that exact subslice.
    #[must_use]
    #[inline]
    pub const fn buf(&self) -> &'a [u8] {
        self.buf
    }

    /// The not-yet-consumed bytes.
    #[must_use]
    #[inline]
    pub fn remaining(&self) -> &'a [u8] {
        // `pos` never exceeds `buf.len()`: every advance goes through a checked `take`.
        self.buf.get(self.pos..).unwrap_or(&[])
    }

    /// How many bytes are left.
    #[must_use]
    #[inline]
    pub const fn remaining_len(&self) -> usize {
        self.buf.len() - self.pos
    }

    /// Whether the cursor is exhausted.
    #[must_use]
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.pos >= self.buf.len()
    }

    /// Consume and return the next `n` bytes, or `None` if fewer remain.
    #[must_use]
    #[inline]
    pub fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let end = self.pos.checked_add(n)?;
        let out = self.buf.get(self.pos..end)?;
        self.pos = end;
        Some(out)
    }

    /// Consume and return everything left.
    #[must_use]
    #[inline]
    pub fn take_rest(&mut self) -> &'a [u8] {
        let out = self.remaining();
        self.pos = self.buf.len();
        out
    }

    /// Advance by at most `n` bytes, saturating at the end.
    #[inline]
    pub fn skip(&mut self, n: usize) {
        self.pos = self.pos.saturating_add(n).min(self.buf.len());
    }

    /// Consume a compressed integer.
    ///
    /// Advances past the whole encoding even when it overflows the 5-byte cap, matching
    /// `LcfReader::ReadInt`. Returns `None` at end of input.
    #[must_use]
    #[inline]
    pub fn varint(&mut self) -> Option<Varint> {
        let v = wire::decode_varint(self.remaining())?;
        self.pos += v.consumed;
        Some(v)
    }

    /// Consume a compressed integer, substituting `0` on malformed or truncated input.
    ///
    /// This is the lenient form the generated readers use: liblcf yields `0` in both cases, and
    /// real files rely on it, so the value is reported alongside a diagnostic rather than an error.
    #[inline]
    pub fn varint_lenient(&mut self, ctx: &mut Ctx<'_>, struct_name: &'static str, chunk_id: u32) -> i32 {
        let offset = self.offset();
        match self.varint() {
            Some(v) => {
                if v.overflow {
                    ctx.report(struct_name, chunk_id, offset, DiagKind::BadVarint);
                }
                v.value as i32
            }
            None => {
                self.pos = self.buf.len();
                ctx.report(struct_name, chunk_id, offset, DiagKind::ShortChunk);
                0
            }
        }
    }

    /// Consume a 64-bit compressed integer.
    #[must_use]
    #[inline]
    pub fn varint64(&mut self) -> Option<Varint64> {
        let v = wire::decode_varint64(self.remaining())?;
        self.pos += v.consumed;
        Some(v)
    }

    /// Consume one raw byte.
    #[must_use]
    #[inline]
    pub fn u8(&mut self) -> Option<u8> {
        self.take(1).map(|b| b[0])
    }

    /// Consume a little-endian `i16`.
    #[must_use]
    #[inline]
    pub fn i16_le(&mut self) -> Option<i16> {
        self.take(2).map(|b| i16::from_le_bytes([b[0], b[1]]))
    }

    /// Consume a little-endian `u32`.
    #[must_use]
    #[inline]
    pub fn u32_le(&mut self) -> Option<u32> {
        self.take(4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    /// Consume a little-endian `i32`.
    #[must_use]
    #[inline]
    pub fn i32_le(&mut self) -> Option<i32> {
        self.take(4).map(|b| i32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    /// Consume a little-endian `f64`.
    #[must_use]
    #[inline]
    pub fn f64_le(&mut self) -> Option<f64> {
        self.take(8)
            .map(|b| f64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]))
    }

    /// Peek at the next byte without consuming it.
    #[must_use]
    #[inline]
    pub fn peek(&self) -> Option<u8> {
        self.buf.get(self.pos).copied()
    }

    /// Read a length-prefixed byte string: a compressed integer length followed by that many
    /// raw bytes.
    ///
    /// The bytes are returned **verbatim**. This crate never transcodes: LCF carries no encoding
    /// marker of any kind, so guessing would be exactly that. See the crate docs.
    #[must_use]
    #[inline]
    pub fn lstring(&mut self) -> Option<&'a [u8]> {
        let len = self.varint()?;
        self.take(len.value as usize)
    }
}

/// One framed chunk: `varint(id) || varint(len) || len bytes`.
#[derive(Clone, Copy, Debug)]
pub struct Chunk<'a> {
    pub id: u32,
    pub data: &'a [u8],
    pub offset: u32,
}

impl<'a> Chunk<'a> {
    /// A cursor over this chunk's payload, reporting file-absolute offsets.
    #[must_use]
    #[inline]
    pub const fn cursor(&self) -> Cursor<'a> {
        Cursor::with_base(self.data, self.offset)
    }
}

/// Walks a chunk stream until the terminating zero id or the end of the buffer.
///
/// This is not an [`Iterator`]: advancing needs a [`Ctx`] to report a malformed frame.
#[derive(Clone, Debug)]
pub struct ChunkIter<'a> {
    cur: Cursor<'a>,
    done: bool,
}

impl<'a> ChunkIter<'a> {
    /// Frame the chunks in `buf`, whose first byte sits at file offset `base`.
    #[must_use]
    #[inline]
    pub const fn new(buf: &'a [u8], base: u32) -> Self {
        Self {
            cur: Cursor::with_base(buf, base),
            done: false,
        }
    }

    /// Frame the chunks in a cursor's remaining bytes.
    #[must_use]
    #[inline]
    pub const fn from_cursor(cur: Cursor<'a>) -> Self {
        Self { cur, done: false }
    }

    /// The underlying cursor, positioned after the last chunk yielded.
    #[must_use]
    #[inline]
    pub const fn cursor(&self) -> &Cursor<'a> {
        &self.cur
    }

    /// Recover the cursor, positioned just past the terminating zero id.
    ///
    /// Needed by the indexed-list codec: a list's elements are chunk streams laid end to end
    /// inside one payload with no length prefix of their own, so after reading an element the
    /// caller must resume from wherever that element's terminator left off.
    #[must_use]
    #[inline]
    pub const fn into_cursor(self) -> Cursor<'a> {
        self.cur
    }

    /// Bytes left unread after the stream terminated.
    #[must_use]
    #[inline]
    pub fn tail(&self) -> &'a [u8] {
        self.cur.remaining()
    }

    /// Consume a whole chunk stream, returning the span it occupied.
    ///
    /// The alternative would be handing out a borrowed iterator and asking the caller to drive it
    /// to the end before moving on - which works exactly once and silently desynchronises if they
    /// stop early. Returning the span instead lets a caller re-walk the same element as often as
    /// they like, and costs only a skim over the chunk headers.
    #[must_use]
    pub fn consume(cur: &mut Cursor<'a>, ctx: &mut Ctx<'_>, struct_name: &'static str) -> &'a [u8] {
        let start = cur.pos();
        let mut chunks = Self::from_cursor(cur.clone());
        while chunks.next(ctx, struct_name).is_some() {}
        *cur = chunks.into_cursor();
        cur.buf().get(start..cur.pos()).unwrap_or(&[])
    }

    /// Find one chunk in a stream by id, without allocating.
    ///
    /// `O(chunks)`, which is the only option: a chunk stream has no index and its entries are
    /// variable-width. Fine for reaching one known field; walk the stream yourself if you want
    /// several.
    #[must_use]
    pub fn find(body: &'a [u8], base: u32, id: u32, ctx: &mut Ctx<'_>, struct_name: &'static str) -> Option<Chunk<'a>> {
        let mut chunks = Self::new(body, base);
        while let Some(chunk) = chunks.next(ctx, struct_name) {
            if chunk.id == id {
                return Some(chunk);
            }
        }
        None
    }

    /// Yield the next chunk.
    ///
    /// Returns `None` on the terminating zero id, at end of input, or once a frame has been found
    /// unrecoverable. A chunk whose declared length runs past the end of the buffer is truncated
    /// to what is actually there and reported as [`DiagKind::ChunkOverrun`].
    #[must_use]
    pub fn next(&mut self, ctx: &mut Ctx<'_>, struct_name: &'static str) -> Option<Chunk<'a>> {
        if self.done || self.cur.is_empty() {
            return None;
        }

        let id_offset = self.cur.offset();
        let id = match self.cur.varint() {
            Some(v) if v.overflow => {
                ctx.report(struct_name, 0, id_offset, DiagKind::BadVarint);
                self.done = true;
                return None;
            }
            Some(v) => v.value,
            None => {
                self.done = true;
                return None;
            }
        };

        // Id 0 terminates the struct (`src/reader_struct_impl.h:68`).
        if id == 0 {
            self.done = true;
            return None;
        }

        let len_offset = self.cur.offset();
        let len = match self.cur.varint() {
            Some(v) if v.overflow => {
                ctx.report(struct_name, id, len_offset, DiagKind::BadVarint);
                self.done = true;
                return None;
            }
            Some(v) => v.value as usize,
            None => {
                ctx.report(struct_name, id, len_offset, DiagKind::ShortChunk);
                self.done = true;
                return None;
            }
        };

        let offset = self.cur.offset();
        let data = match self.cur.take(len) {
            Some(data) => data,
            None => {
                ctx.report(struct_name, id, offset, DiagKind::ChunkOverrun);
                self.done = true;
                self.cur.take_rest()
            }
        };

        Some(Chunk { id, data, offset })
    }
}

/// One element of an indexed list.
#[derive(Clone, Copy, Debug)]
pub struct ListItem<'a> {
    /// The element's id, from the compressed integer preceding its chunk stream.
    pub id: i32,
    /// The element's chunk stream, terminator excluded.
    pub body: &'a [u8],
    /// File-absolute offset of `body`.
    pub offset: u32,
}

impl<'a> ListItem<'a> {
    /// The element's chunks.
    #[must_use]
    #[inline]
    pub const fn chunks(&self) -> ChunkIter<'a> {
        ChunkIter::new(self.body, self.offset)
    }

    /// One of the element's chunks, by id.
    #[must_use]
    #[inline]
    pub fn find(&self, id: u32, ctx: &mut Ctx<'_>, struct_name: &'static str) -> Option<Chunk<'a>> {
        ChunkIter::find(self.body, self.offset, id, ctx, struct_name)
    }
}

/// Walks an indexed list without allocating.
///
/// The core-tier counterpart to [`List<T>`](crate::field::List), which needs `alloc` because it
/// materialises every element. This yields borrowed spans instead, so an embedded consumer can
/// reach one map's events, or one actor's name, without a heap.
#[derive(Clone, Debug)]
pub struct ListWalker<'a> {
    cur: Cursor<'a>,
    left: u32,
    name: &'static str,
}

impl<'a> ListWalker<'a> {
    /// Start walking the list in a chunk payload.
    ///
    /// `struct_name` names the *element* type, and is only used to locate diagnostics.
    #[must_use]
    pub fn new(body: &'a [u8], base: u32, ctx: &mut Ctx<'_>, struct_name: &'static str) -> Self {
        let mut cur = Cursor::with_base(body, base);
        let count = cur.varint_lenient(ctx, struct_name, 0);
        // Each element costs at least two bytes (an id and a terminator), which bounds a count
        // that came from a hostile file.
        let left = (count.max(0) as usize).min(cur.remaining_len() / 2 + 1) as u32;
        Self {
            cur,
            left,
            name: struct_name,
        }
    }

    /// How many elements the list declared and has not yielded yet.
    #[must_use]
    #[inline]
    pub const fn remaining(&self) -> u32 {
        self.left
    }

    /// Yield the next element.
    #[must_use]
    pub fn next(&mut self, ctx: &mut Ctx<'_>) -> Option<ListItem<'a>> {
        if self.left == 0 || self.cur.is_empty() {
            return None;
        }
        self.left -= 1;

        let id = self.cur.varint_lenient(ctx, self.name, 0);
        let offset = self.cur.offset();
        let body = ChunkIter::consume(&mut self.cur, ctx, self.name);
        Some(ListItem { id, body, offset })
    }
}

//! Per-field wire codecs.
//!
//! Every LCF field kind - a compressed integer, a raw byte string, a packed primitive array, a
//! nested struct, an indexed struct list - implements [`LcfField`]. The schema macro then needs
//! to know nothing about types: it emits `match chunk.id { 0x01 => obj.name = LcfField::read(...) }`
//! and the right codec is selected at compile time, with no runtime dispatch table.
//!
//! **Reading is infallible** - [`LcfField::read`] returns a value, never a `Result`. A field whose payload is malformed keeps
//! its default and pushes a [`Diagnostic`](crate::diag::Diagnostic); it cannot abort the parse.
//! This keeps the generated readers branch-free of error propagation, and real files depend on
//! this leniency. A caller that wants to reject malformed input instead passes a
//! [`StrictSink`](crate::diag::StrictSink); the *loader* is what stops, at the next chunk
//! boundary, not the field read.

use crate::{
    cursor::{ChunkIter, Ctx, Cursor},
    diag::DiagKind,
    engine::Engine,
    sink::Sink,
    wire,
};
use alloc::{borrow::Cow, vec::Vec};

/// A borrowed or owned run of bytes from a file.
///
/// **Not a string.** LCF records no encoding anywhere in the file, so this crate does not pretend
/// to know one; see the crate-level docs. The bytes are whatever the authoring machine's codepage
/// produced - commonly Shift-JIS or CP1252, but nothing in the format says which.
///
/// Borrows from the input buffer until [`into_owned`](DbStr::into_owned) is called, so loading a
/// database copies no text at all.
#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DbStr<'a>(Cow<'a, [u8]>);

impl<'a> DbStr<'a> {
    /// Borrow a run of bytes.
    #[must_use]
    #[inline]
    pub const fn from_bytes(bytes: &'a [u8]) -> Self {
        Self(Cow::Borrowed(bytes))
    }

    /// Take ownership of a run of bytes.
    #[must_use]
    #[inline]
    pub const fn from_vec(bytes: Vec<u8>) -> Self {
        Self(Cow::Owned(bytes))
    }

    /// The bytes, exactly as they appeared in the file.
    #[must_use]
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Length in bytes.
    #[must_use]
    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the field is empty. Empty is also the default for every string field, which is
    /// what makes the writer elide it.
    #[must_use]
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Detach from the input buffer.
    #[must_use]
    pub fn into_owned(self) -> DbStr<'static> {
        DbStr(Cow::Owned(self.0.into_owned()))
    }

    /// Interpret as UTF-8, if it happens to be valid.
    ///
    /// Returns `None` for anything else - which includes most non-English games. Provided for
    /// the common case of a filename or an identifier that is incidentally ASCII; it is not an
    /// encoding layer and is not a substitute for one.
    #[must_use]
    #[inline]
    pub fn as_utf8(&self) -> Option<&str> {
        core::str::from_utf8(&self.0).ok()
    }
}

impl<'a> From<&'a [u8]> for DbStr<'a> {
    #[inline]
    fn from(bytes: &'a [u8]) -> Self {
        Self::from_bytes(bytes)
    }
}

/// Enumerate a struct's own string fields, by name.
///
/// Translation tools need to ask "what text does this actor hold, and what is each piece called",
/// and doing it by hand means naming all 127 fields of `Terms`.
///
/// One struct, its own string fields, no recursion and no context chain.
/// Nesting is something the caller already has typed access to, so the visitor
/// does not need to reinvent it.
pub trait Strings<'a> {
    /// Call `f` once per string field, with the field's name and value.
    fn visit_strings(&self, f: &mut dyn FnMut(&'static str, &DbStr<'a>));
}

/// A run of booleans stored one byte per element.
///
/// The wire form is one byte per boolean. Only the separate flags nundles are truly bit-packed.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BitArray<'a>(Cow<'a, [u8]>);

impl<'a> BitArray<'a> {
    /// Borrow the raw bytes.
    #[must_use]
    #[inline]
    pub const fn from_bytes(bytes: &'a [u8]) -> Self {
        Self(Cow::Borrowed(bytes))
    }

    /// Take ownership of the raw bytes, one per boolean.
    #[must_use]
    #[inline]
    pub const fn from_vec(bytes: Vec<u8>) -> Self {
        Self(Cow::Owned(bytes))
    }

    /// Number of booleans.
    #[must_use]
    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether there are no booleans.
    #[must_use]
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The boolean at `index`, or `None` if out of range.
    #[must_use]
    #[inline]
    pub fn get(&self, index: usize) -> Option<bool> {
        self.0.get(index).map(|&b| b != 0)
    }

    /// Iterate the booleans.
    pub fn iter(&self) -> impl Iterator<Item = bool> + '_ {
        self.0.iter().map(|&b| b != 0)
    }

    /// The underlying bytes.
    #[must_use]
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Detach from the input buffer.
    #[must_use]
    pub fn into_owned(self) -> BitArray<'static> {
        BitArray(Cow::Owned(self.0.into_owned()))
    }
}

/// A field that can be read from a chunk payload and written back.
pub trait LcfField<'a>: Sized {
    /// Decode from a chunk payload sliced to exactly this field's bytes.
    ///
    /// `owner` and `id` are only used to locate diagnostics.
    fn read(data: &'a [u8], ctx: &mut Ctx<'_>, owner: &'static str, id: u32) -> Self;

    /// The value this field holds when its chunk is absent and the schema gives no explicit
    /// default.
    ///
    /// Engine-dependent because a nested struct's own defaults are, and because the writer
    /// elides any field equal to its default - so getting this wrong changes the bytes emitted,
    /// not just the value observed.
    fn field_default(engine: Engine) -> Self;

    /// Encoded length in bytes, which the two-pass writer emits as the chunk's length prefix.
    fn lcf_size(&self, engine: Engine) -> u32;

    /// Whether `self` would be written differently from `other`, which is how the writer decides to
    /// elide a field equal to its default.
    ///
    /// Plain `!=` except for floats, which compare by bits: `-0.0 == 0.0`, so `!=` would drop a
    /// stored `-0.0` and break the byte-for-byte round trip.
    #[inline]
    fn lcf_differs(&self, other: &Self) -> bool
    where
        Self: PartialEq,
    {
        self != other
    }

    /// Number of elements, for array-like fields.
    ///
    /// Consulted only by companion `count`/`size` chunks and by their elision test.
    /// Scalar fields never have a companion chunk, so the default of
    /// `0` is never observed for them.
    #[inline]
    fn element_count(&self) -> u32 {
        0
    }

    /// Append the encoded payload.
    ///
    /// # Errors
    ///
    /// Propagates the sink's error. Infallible for `Vec<u8>`.
    fn write<S: Sink>(&self, sink: &mut S, engine: Engine) -> Result<(), S::Error>;
}

/// Report that a payload had bytes the field did not consume.
#[inline]
fn check_consumed(cur: &Cursor<'_>, ctx: &mut Ctx<'_>, owner: &'static str, id: u32) {
    if !cur.is_empty() {
        ctx.report(owner, id, cur.offset(), DiagKind::TrailingBytes);
    }
}

impl<'a> LcfField<'a> for i32 {
    #[inline]
    fn field_default(_engine: Engine) -> Self {
        0
    }

    #[inline]
    fn read(data: &'a [u8], ctx: &mut Ctx<'_>, owner: &'static str, id: u32) -> Self {
        let mut cur = Cursor::new(data);
        let value = cur.varint_lenient(ctx, owner, id);
        check_consumed(&cur, ctx, owner, id);
        value
    }

    #[inline]
    fn lcf_size(&self, _engine: Engine) -> u32 {
        wire::varint_len(*self as u32)
    }

    #[inline]
    fn write<S: Sink>(&self, sink: &mut S, _engine: Engine) -> Result<(), S::Error> {
        wire::write_varint(sink, *self as u32)
    }
}

impl<'a> LcfField<'a> for bool {
    #[inline]
    fn field_default(_engine: Engine) -> Self {
        false
    }

    #[inline]
    fn read(data: &'a [u8], ctx: &mut Ctx<'_>, owner: &'static str, id: u32) -> Self {
        <i32 as LcfField>::read(data, ctx, owner, id) > 0
    }

    #[inline]
    fn lcf_size(&self, _engine: Engine) -> u32 {
        1
    }

    #[inline]
    fn write<S: Sink>(&self, sink: &mut S, _engine: Engine) -> Result<(), S::Error> {
        sink.write(&[u8::from(*self)])
    }
}

/// Generate a codec for a fixed-width little-endian primitive.
macro_rules! impl_fixed_field {
    ($ty:ty, $size:literal, $read:ident) => {
        impl<'a> LcfField<'a> for $ty {
            #[inline]
            fn field_default(_engine: Engine) -> Self {
                <$ty>::default()
            }

            #[inline]
            fn read(data: &'a [u8], ctx: &mut Ctx<'_>, owner: &'static str, id: u32) -> Self {
                let mut cur = Cursor::new(data);
                match cur.$read() {
                    Some(value) => {
                        check_consumed(&cur, ctx, owner, id);
                        value
                    }
                    None => {
                        ctx.report(owner, id, cur.offset(), DiagKind::ShortChunk);
                        Self::default()
                    }
                }
            }

            #[inline]
            fn lcf_size(&self, _engine: Engine) -> u32 {
                $size
            }

            #[inline]
            fn lcf_differs(&self, other: &Self) -> bool {
                self.to_le_bytes() != other.to_le_bytes()
            }

            #[inline]
            fn write<S: Sink>(&self, sink: &mut S, _engine: Engine) -> Result<(), S::Error> {
                sink.write(&self.to_le_bytes())
            }
        }
    };
}

impl_fixed_field!(i16, 2, i16_le);
impl_fixed_field!(u32, 4, u32_le);
impl_fixed_field!(f64, 8, f64_le);

impl<'a> LcfField<'a> for u8 {
    #[inline]
    fn field_default(_engine: Engine) -> Self {
        0
    }

    #[inline]
    fn read(data: &'a [u8], ctx: &mut Ctx<'_>, owner: &'static str, id: u32) -> Self {
        match data.first() {
            Some(&byte) => byte,
            None => {
                ctx.report(owner, id, 0, DiagKind::ShortChunk);
                0
            }
        }
    }

    #[inline]
    fn lcf_size(&self, _engine: Engine) -> u32 {
        1
    }

    #[inline]
    fn write<S: Sink>(&self, sink: &mut S, _engine: Engine) -> Result<(), S::Error> {
        sink.write(&[*self])
    }
}

impl<'a> LcfField<'a> for i8 {
    #[inline]
    fn field_default(_engine: Engine) -> Self {
        0
    }

    #[inline]
    fn read(data: &'a [u8], ctx: &mut Ctx<'_>, owner: &'static str, id: u32) -> Self {
        <u8 as LcfField>::read(data, ctx, owner, id) as Self
    }

    #[inline]
    fn lcf_size(&self, _engine: Engine) -> u32 {
        1
    }

    #[inline]
    fn write<S: Sink>(&self, sink: &mut S, _engine: Engine) -> Result<(), S::Error> {
        sink.write(&[*self as u8])
    }
}

impl<'a> LcfField<'a> for DbStr<'a> {
    #[inline]
    fn field_default(_engine: Engine) -> Self {
        Self::from_bytes(&[])
    }

    #[inline]
    fn read(data: &'a [u8], _ctx: &mut Ctx<'_>, _owner: &'static str, _id: u32) -> Self {
        // The chunk length *is* the string length; there is no inner prefix. Zero-copy.
        Self::from_bytes(data)
    }

    #[inline]
    #[allow(clippy::cast_possible_truncation)]
    fn lcf_size(&self, _engine: Engine) -> u32 {
        self.0.len() as u32
    }

    #[inline]
    #[allow(clippy::cast_possible_truncation)]
    fn element_count(&self) -> u32 {
        self.0.len() as u32
    }

    #[inline]
    fn write<S: Sink>(&self, sink: &mut S, _engine: Engine) -> Result<(), S::Error> {
        sink.write(&self.0)
    }
}

impl<'a> LcfField<'a> for BitArray<'a> {
    #[inline]
    fn field_default(_engine: Engine) -> Self {
        Self::from_bytes(&[])
    }

    #[inline]
    fn read(data: &'a [u8], _ctx: &mut Ctx<'_>, _owner: &'static str, _id: u32) -> Self {
        Self::from_bytes(data)
    }

    #[inline]
    #[allow(clippy::cast_possible_truncation)]
    fn lcf_size(&self, _engine: Engine) -> u32 {
        self.0.len() as u32
    }

    #[inline]
    #[allow(clippy::cast_possible_truncation)]
    fn element_count(&self) -> u32 {
        self.0.len() as u32
    }

    #[inline]
    fn write<S: Sink>(&self, sink: &mut S, _engine: Engine) -> Result<(), S::Error> {
        sink.write(&self.0)
    }
}

/// A primitive that appears inside a raw packed array.
///
/// Packed arrays carry no element count: the count is the chunk length divided by the element
/// size. A payload that is not a whole multiple is truncated and reported as [`DiagKind::RaggedArray`].
pub trait PackedPrim: Sized + Copy {
    /// Bytes per element on the wire.
    const SIZE: usize;
    /// Decode one element from exactly `SIZE` bytes.
    fn decode(bytes: &[u8]) -> Self;
    /// Encode one element.
    fn encode(self, out: &mut [u8; 8]) -> usize;
}

impl PackedPrim for u8 {
    const SIZE: usize = 1;
    #[inline]
    fn decode(bytes: &[u8]) -> Self {
        bytes[0]
    }
    #[inline]
    fn encode(self, out: &mut [u8; 8]) -> usize {
        out[0] = self;
        1
    }
}

impl PackedPrim for i16 {
    const SIZE: usize = 2;
    #[inline]
    fn decode(bytes: &[u8]) -> Self {
        Self::from_le_bytes([bytes[0], bytes[1]])
    }
    #[inline]
    fn encode(self, out: &mut [u8; 8]) -> usize {
        out[..2].copy_from_slice(&self.to_le_bytes());
        2
    }
}

impl PackedPrim for i32 {
    const SIZE: usize = 4;
    #[inline]
    fn decode(bytes: &[u8]) -> Self {
        Self::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
    }
    #[inline]
    fn encode(self, out: &mut [u8; 8]) -> usize {
        out[..4].copy_from_slice(&self.to_le_bytes());
        4
    }
}

impl PackedPrim for u32 {
    const SIZE: usize = 4;
    #[inline]
    fn decode(bytes: &[u8]) -> Self {
        Self::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
    }
    #[inline]
    fn encode(self, out: &mut [u8; 8]) -> usize {
        out[..4].copy_from_slice(&self.to_le_bytes());
        4
    }
}

/// A raw packed array of a fixed-width primitive.
///
/// A newtype rather than a bare `Vec<T>` so it cannot collide with the indexed struct list
/// codec: `Vec<i16>` and `Vec<Learning>` are both "a `Vec`" to the type system but entirely
/// different wire formats, and making the distinction a type keeps the schema honest about which
/// one a field is.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Packed<T>(pub Vec<T>);

impl<T> core::ops::Deref for Packed<T> {
    type Target = Vec<T>;
    #[inline]
    fn deref(&self) -> &Vec<T> {
        &self.0
    }
}

impl<'a, T: PackedPrim> LcfField<'a> for Packed<T> {
    #[inline]
    fn field_default(_engine: Engine) -> Self {
        Self(Vec::new())
    }

    fn read(data: &'a [u8], ctx: &mut Ctx<'_>, owner: &'static str, id: u32) -> Self {
        let count = data.len() / T::SIZE;
        if data.len() % T::SIZE != 0 {
            ctx.report(owner, id, 0, DiagKind::RaggedArray);
        }
        let mut out = Vec::with_capacity(count);
        for i in 0..count {
            out.push(T::decode(&data[i * T::SIZE..]));
        }
        Self(out)
    }

    #[inline]
    #[allow(clippy::cast_possible_truncation)]
    fn lcf_size(&self, _engine: Engine) -> u32 {
        (self.0.len() * T::SIZE) as u32
    }

    #[inline]
    #[allow(clippy::cast_possible_truncation)]
    fn element_count(&self) -> u32 {
        self.0.len() as u32
    }

    fn write<S: Sink>(&self, sink: &mut S, _engine: Engine) -> Result<(), S::Error> {
        let mut buf = [0u8; 8];
        for &value in &self.0 {
            let n = value.encode(&mut buf);
            sink.write(&buf[..n])?;
        }
        Ok(())
    }
}

/// A struct serialised as a chunk stream.
///
/// Implemented by the schema macro for every one of the format's chunked structs.
pub trait LcfStruct<'a>: Sized {
    /// Name used to locate diagnostics, e.g. `"Actor"`.
    const NAME: &'static str;

    /// The value every field holds before any chunk is read.
    ///
    /// Engine-dependent because ~30 fields have different 2000 and 2003 defaults.
    /// Replaces `StructDefault<S>::make(bool)` and `rpg::Actor::Setup(bool)`.
    fn defaults(engine: Engine) -> Self;

    /// Apply one chunk.
    ///
    /// Returns the id of the field that consumed it - which is the chunk's own id, except for a
    /// companion size/count chunk, which reports the id of the sibling it belongs to. `None`
    /// means no field claimed it; [`read_stream`](LcfStruct::read_stream) then keeps it verbatim
    /// as an [`UnknownChunk`] anchored to whatever was recognised last.
    fn apply_chunk(&mut self, id: u32, data: &'a [u8], ctx: &mut Ctx<'_>) -> Option<u32>;

    /// The chunks no field claimed, kept so they can be written back.
    fn unknown_chunks(&self) -> &UnknownChunks;

    /// The chunks no field claimed, mutably.
    fn unknown_chunks_mut(&mut self) -> &mut UnknownChunks;

    /// Encoded length of the struct's body, excluding any terminating zero.
    ///
    /// Derived from [`write_body`](LcfStruct::write_body) rather than generated separately.
    fn body_size(&self, engine: Engine) -> u32 {
        let mut counter = crate::sink::CountSink::new();
        // `CountSink::Error` is `Infallible`, so this cannot fail.
        let _ = self.write_body(&mut counter, engine);
        counter.len() as u32
    }

    /// Append the struct's body, excluding any terminating zero.
    ///
    /// # Errors
    ///
    /// Propagates the sink's error.
    fn write_body<S: Sink>(&self, sink: &mut S, engine: Engine) -> Result<(), S::Error>;

    /// Read a whole struct from a chunk stream.
    fn read_stream(chunks: &mut ChunkIter<'a>, ctx: &mut Ctx<'_>) -> Self {
        let mut obj = Self::defaults(ctx.engine);
        // See `UnknownChunk::anchor`: 0 terminates a chunk stream, so it is free to mean "no
        // recognised chunk has been seen yet".
        let mut anchor = 0u32;
        while let Some(chunk) = chunks.next(ctx, Self::NAME) {
            match obj.apply_chunk(chunk.id, chunk.data, ctx) {
                Some(field_id) => anchor = field_id,
                None => {
                    // Still a diagnostic - the chunk is preserved, not understood.
                    ctx.report(Self::NAME, chunk.id, chunk.offset, DiagKind::UnknownChunk);
                    obj.unknown_chunks_mut().0.push(UnknownChunk {
                        data: chunk.data.to_vec(),
                        id: chunk.id,
                        anchor,
                    });
                }
            }
            if ctx.should_abort() {
                break;
            }
        }
        obj
    }
}

/// Any chunked struct is also a field: a nested struct's payload is just another chunk stream.
impl<'a, T: LcfStruct<'a>> LcfField<'a> for T {
    #[inline]
    fn field_default(engine: Engine) -> Self {
        T::defaults(engine)
    }

    #[inline]
    fn read(data: &'a [u8], ctx: &mut Ctx<'_>, _owner: &'static str, _id: u32) -> Self {
        let mut chunks = ChunkIter::new(data, 0);
        T::read_stream(&mut chunks, ctx)
    }

    #[inline]
    fn lcf_size(&self, engine: Engine) -> u32 {
        // +1 for the terminating zero id, which nested structs always carry.
        self.body_size(engine) + 1
    }

    #[inline]
    fn write<S: Sink>(&self, sink: &mut S, engine: Engine) -> Result<(), S::Error> {
        self.write_body(sink, engine)?;
        sink.write(&[0])
    }
}

// Tests for this module live in `tests/field.rs`: `LcfField`, `DbStr`, `BitArray` and `Packed`
// are all `pub`, so a black-box test has nothing more to reach.

/// A chunked struct that carries a leading id.
pub trait LcfIndexed<'a>: LcfStruct<'a> {
    /// The element's id, as stored ahead of its chunk stream.
    fn id(&self) -> i32;
    /// Set the element's id.
    fn set_id(&mut self, id: i32);
}

/// A list of indexed structs: `varint(count)`, then per element `varint(id)` and a chunk stream.
///
/// A newtype rather than a bare `Vec<T>` for the same reason as [`Packed`]: `Vec<i16>` and
/// `Vec<Learning>` are indistinguishable to the type system but are completely different wire
/// formats, so the schema states which one it means.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct List<T>(pub Vec<T>);

impl<T> core::ops::Deref for List<T> {
    type Target = Vec<T>;
    #[inline]
    fn deref(&self) -> &Vec<T> {
        &self.0
    }
}

impl<T> core::ops::DerefMut for List<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Vec<T> {
        &mut self.0
    }
}

impl<'a, T: LcfIndexed<'a>> List<T> {
    /// Read a list from a shared cursor, leaving it positioned after the last element.
    ///
    /// Needed because `TreeMap` embeds a `MapInfo` list directly in the file stream rather than
    /// inside a length-prefixed chunk, so the caller has to keep
    /// reading from the same position afterwards.
    pub fn read_inline(cur: &mut Cursor<'a>, ctx: &mut Ctx<'_>, owner: &'static str, id: u32) -> Self {
        let count = cur.varint_lenient(ctx, owner, id);

        // The count is attacker-controlled, so it must not reach `with_capacity` unchecked. Each
        // element costs at least two bytes on the wire (an id and a terminator), which bounds how
        // many can possibly be present.
        let max_possible = cur.remaining_len() / 2 + 1;
        let count = (count.max(0) as usize).min(max_possible);
        let mut out = Vec::with_capacity(count);

        for _ in 0..count {
            if cur.is_empty() {
                ctx.report(owner, id, cur.offset(), DiagKind::ShortChunk);
                break;
            }
            let elem_id = cur.varint_lenient(ctx, owner, id);
            let mut chunks = ChunkIter::from_cursor(cur.clone());
            let mut elem = T::read_stream(&mut chunks, ctx);
            elem.set_id(elem_id);
            *cur = chunks.into_cursor();
            out.push(elem);
            if ctx.should_abort() {
                break;
            }
        }

        Self(out)
    }
}

impl<'a, T: LcfIndexed<'a>> LcfField<'a> for List<T> {
    #[inline]
    fn field_default(_engine: Engine) -> Self {
        Self(Vec::new())
    }

    #[inline]
    fn read(data: &'a [u8], ctx: &mut Ctx<'_>, owner: &'static str, id: u32) -> Self {
        let mut cur = Cursor::new(data);
        Self::read_inline(&mut cur, ctx, owner, id)
    }

    #[inline]
    #[allow(clippy::cast_possible_truncation)]
    fn element_count(&self) -> u32 {
        self.0.len() as u32
    }

    #[allow(clippy::cast_possible_truncation)]
    fn lcf_size(&self, engine: Engine) -> u32 {
        let mut total = wire::varint_len(self.0.len() as u32);
        for elem in &self.0 {
            // id + body + terminating zero
            total += wire::varint_len(elem.id() as u32) + elem.body_size(engine) + 1;
        }
        total
    }

    #[allow(clippy::cast_possible_truncation)]
    fn write<S: Sink>(&self, sink: &mut S, engine: Engine) -> Result<(), S::Error> {
        wire::write_varint(sink, self.0.len() as u32)?;
        for elem in &self.0 {
            wire::write_varint(sink, elem.id() as u32)?;
            elem.write_body(sink, engine)?;
            sink.write(&[0])?;
        }
        Ok(())
    }
}

/// A list of structs that carry **no** id: `varint(count)` then bare chunk streams.
///
/// liblcf decides between this and [`List`] with a SFINAE probe for a member named `ID`
/// (`IDChecker<T>`, `src/reader_struct.h:539`), dispatching to `IDReaderT<S, false>` whose
/// `ReadID`/`WriteID` are empty. Exactly one type in the schema needs it - `SaveEasyRpgText`, an
/// EasyRPG extension - but getting it wrong would silently shift every element by one compressed
/// integer, so it is a distinct type rather than a runtime flag.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct UnindexedList<T>(pub Vec<T>);

impl<T> core::ops::Deref for UnindexedList<T> {
    type Target = Vec<T>;
    #[inline]
    fn deref(&self) -> &Vec<T> {
        &self.0
    }
}

impl<T> core::ops::DerefMut for UnindexedList<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Vec<T> {
        &mut self.0
    }
}

impl<'a, T: LcfStruct<'a>> LcfField<'a> for UnindexedList<T> {
    #[inline]
    fn field_default(_engine: Engine) -> Self {
        Self(Vec::new())
    }

    fn read(data: &'a [u8], ctx: &mut Ctx<'_>, owner: &'static str, id: u32) -> Self {
        let mut cur = Cursor::new(data);
        let count = cur.varint_lenient(ctx, owner, id);

        // The count is attacker-controlled; each element costs at least one byte (a terminator),
        // which bounds how many can actually be present.
        let count = (count.max(0) as usize).min(cur.remaining_len() + 1);
        let mut out = Vec::with_capacity(count);

        for _ in 0..count {
            if cur.is_empty() {
                ctx.report(owner, id, cur.offset(), DiagKind::ShortChunk);
                break;
            }
            let mut chunks = ChunkIter::from_cursor(cur);
            out.push(T::read_stream(&mut chunks, ctx));
            cur = chunks.into_cursor();
            if ctx.should_abort() {
                break;
            }
        }

        Self(out)
    }

    #[allow(clippy::cast_possible_truncation)]
    fn lcf_size(&self, engine: Engine) -> u32 {
        let mut total = wire::varint_len(self.0.len() as u32);
        for elem in &self.0 {
            total += elem.body_size(engine) + 1;
        }
        total
    }

    #[allow(clippy::cast_possible_truncation)]
    fn write<S: Sink>(&self, sink: &mut S, engine: Engine) -> Result<(), S::Error> {
        wire::write_varint(sink, self.0.len() as u32)?;
        for elem in &self.0 {
            elem.write_body(sink, engine)?;
            sink.write(&[0])?;
        }
        Ok(())
    }

    #[inline]
    #[allow(clippy::cast_possible_truncation)]
    fn element_count(&self) -> u32 {
        self.0.len() as u32
    }
}

/// A chunk whose id no field of the owning struct claims.
///
/// RPG Maker's forward-compatibility story is "write chunks the reader may not know", so files in
/// the wild carry ids this schema has no field for - an id added by a later `RPG_RT` build, an
/// EasyRPG extension, or a third-party editor. Dropping them makes a load/save cycle lossy for
/// exactly the files whose contents we understand the least, so they are kept verbatim and
/// written back.
///
/// The payload is raw bytes because that is the only honest thing to do: with no schema entry
/// there is nothing to say whether it is an integer, a string, or a nested chunk stream - LCF
/// chunks carry no type tag (see the [schema docs](crate::schema)).
///
/// **It owns its bytes**, unlike every other field kind here. Twenty-odd schema structs borrow
/// nothing and so carry no lifetime parameter; giving them one just to hold unknown chunks would
/// be a far larger break than a copy on a path that is empty for every well-formed file.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct UnknownChunk {
    /// The payload, exactly as it appeared. Never interpreted.
    pub data: Vec<u8>,
    /// The chunk id, as read.
    pub id: u32,
    /// Id of the last *recognised* field seen before this chunk, or `0` if it came before any.
    ///
    /// This is what puts the chunk back where it was found. Anchoring to a neighbouring field
    /// rather than sorting by id matters because the schema's declaration order is liblcf's
    /// emission order, and that is not ascending in seven structs (`Enemy`, `SaveSystem`,
    /// `SavePicture`, ...) - sorting would move unknown chunks across known ones.
    ///
    /// `0` is free to mean "before everything" because a zero id terminates a chunk stream and so
    /// can never be a field's own id.
    pub anchor: u32,
}

/// The unrecognised chunks of one struct, in the order they were read.
///
/// Empty for a well-formed file this schema fully covers - the common case, where it costs one
/// `Vec` header per struct and no allocation at all.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct UnknownChunks(pub Vec<UnknownChunk>);

impl UnknownChunks {
    /// An empty set. Allocates nothing.
    #[must_use]
    #[inline]
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    /// Whether the struct had no unrecognised chunks.
    ///
    /// Spelled out rather than left to `Deref` because the schema macro names it as a
    /// `serde(skip_serializing_if)` path, which needs a real inherent method.
    #[must_use]
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// How many were kept.
    #[must_use]
    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Write back every chunk anchored to `anchor`, header and payload, in read order.
    ///
    /// The generated writer calls this once per field, so the empty-case early-out is what keeps
    /// it to one well-predicted branch per field for files that have no unknown chunks. The
    /// length prefix is recomputed from the payload rather than stored, so a caller that edits
    /// `data` still emits a consistent chunk.
    ///
    /// # Errors
    ///
    /// Propagates the sink's error.
    #[allow(clippy::cast_possible_truncation)]
    pub fn write_anchored<S: Sink>(&self, sink: &mut S, anchor: u32) -> Result<(), S::Error> {
        if self.0.is_empty() {
            return Ok(());
        }
        for chunk in self.0.iter().filter(|chunk| chunk.anchor == anchor) {
            wire::write_varint(sink, chunk.id)?;
            wire::write_varint(sink, chunk.data.len() as u32)?;
            sink.write(&chunk.data)?;
        }
        Ok(())
    }
}

impl core::ops::Deref for UnknownChunks {
    type Target = Vec<UnknownChunk>;
    #[inline]
    fn deref(&self) -> &Vec<UnknownChunk> {
        &self.0
    }
}

impl core::ops::DerefMut for UnknownChunks {
    #[inline]
    fn deref_mut(&mut self) -> &mut Vec<UnknownChunk> {
        &mut self.0
    }
}

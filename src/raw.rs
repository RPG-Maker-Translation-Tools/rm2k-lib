//! Codecs for the handful of types that do not fit the chunked-struct shape: fixed-layout structs
//! with no chunk framing at all ([`Rect`], [`Equipment`], [`Parameters`]), and a few field kinds
//! whose encoding is too irregular for the schema macro ([`EventCommandList`],
//! [`MoveCommandList`], [`StrList`], [`EmptyBlock`], [`DatabaseVersion`]).
//!
//! [`EventCommandList`] and [`MoveCommandList`] are the owning counterparts to the borrowing
//! iterators in [`crate::event`], which is where the actual framing rules are documented. A
//! caller that only wants to *read* commands should prefer those: they work in the core tier and
//! copy nothing.

use crate::{
    cursor::{Ctx, Cursor},
    diag::DiagKind,
    engine::Engine,
    event::{EventCommandIter, MoveCommandIter, move_code},
    field::{DbStr, LcfField, Packed},
    sink::Sink,
    wire,
};
use alloc::vec::Vec;

/// A chunk that exists but carries nothing.
///
/// Three chunks in `Database` are declared but unused (`commoneventD2`, `commoneventD3`,
/// `classD1`, all 2003-only). RPG Maker writes them as a header with a zero length, and files
/// round-trip only if they are written back the same way - hence a real type rather than simply
/// omitting them from the schema.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct EmptyBlock;

impl<'a> LcfField<'a> for EmptyBlock {
    #[inline]
    fn field_default(_engine: Engine) -> Self {
        Self
    }
    #[inline]
    fn read(_data: &'a [u8], _ctx: &mut Ctx<'_>, _owner: &'static str, _id: u32) -> Self {
        Self
    }
    #[inline]
    fn lcf_size(&self, _engine: Engine) -> u32 {
        0
    }
    #[inline]
    fn write<S: Sink>(&self, _sink: &mut S, _engine: Engine) -> Result<(), S::Error> {
        Ok(())
    }
}

/// The database format version, which serialises unlike any other integer.
///
/// Two irregularities, both from `DatabaseVersionField` (`src/reader_struct.h`):
///
/// - A value of `0` writes an **empty** chunk - the header is emitted, the payload is not.
/// - Under RPG Maker 2003 the chunk is *always* written, even at its default. Under 2000 it is
///   written only when non-zero.
///
/// The second rule is why the schema tags this field `db_version` rather than `persist`.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DatabaseVersion(pub i32);

impl<'a> LcfField<'a> for DatabaseVersion {
    #[inline]
    fn field_default(_engine: Engine) -> Self {
        Self(0)
    }

    #[inline]
    fn read(data: &'a [u8], ctx: &mut Ctx<'_>, owner: &'static str, id: u32) -> Self {
        if data.is_empty() {
            return Self(0);
        }
        Self(<i32 as LcfField>::read(data, ctx, owner, id))
    }

    #[inline]
    fn lcf_size(&self, _engine: Engine) -> u32 {
        if self.0 == 0 {
            0
        } else {
            wire::varint_len(self.0 as u32)
        }
    }

    #[inline]
    fn write<S: Sink>(&self, sink: &mut S, _engine: Engine) -> Result<(), S::Error> {
        if self.0 == 0 {
            return Ok(());
        }
        wire::write_varint(sink, self.0 as u32)
    }
}

/// A map-tree rectangle: four raw little-endian 32-bit values, no chunk framing.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Rect {
    pub left: u32,
    pub top: u32,
    pub right: u32,
    pub bottom: u32,
}

impl<'a> LcfField<'a> for Rect {
    #[inline]
    fn field_default(_engine: Engine) -> Self {
        Self::default()
    }

    fn read(data: &'a [u8], ctx: &mut Ctx<'_>, owner: &'static str, id: u32) -> Self {
        let mut cur = Cursor::new(data);
        let mut out = Self::default();
        for slot in [&mut out.left, &mut out.top, &mut out.right, &mut out.bottom] {
            match cur.u32_le() {
                Some(v) => *slot = v,
                None => {
                    ctx.report(owner, id, cur.offset(), DiagKind::ShortChunk);
                    break;
                }
            }
        }
        out
    }

    #[inline]
    fn lcf_size(&self, _engine: Engine) -> u32 {
        16
    }

    fn write<S: Sink>(&self, sink: &mut S, _engine: Engine) -> Result<(), S::Error> {
        for value in [self.left, self.top, self.right, self.bottom] {
            sink.write(&value.to_le_bytes())?;
        }
        Ok(())
    }
}

/// An actor's starting equipment: five raw little-endian 16-bit item ids.
///
/// liblcf refuses to parse this at all unless the payload is exactly ten bytes, skipping the
/// chunk and leaving every id zero, and this is reproduced.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Equipment {
    pub weapon_id: i16,
    pub shield_id: i16,
    pub armor_id: i16,
    pub helmet_id: i16,
    pub accessory_id: i16,
}

impl Equipment {
    pub const SIZE: u32 = 10;
}

impl<'a> LcfField<'a> for Equipment {
    #[inline]
    fn field_default(_engine: Engine) -> Self {
        Self::default()
    }

    fn read(data: &'a [u8], ctx: &mut Ctx<'_>, owner: &'static str, id: u32) -> Self {
        let mut out = Self::default();
        if data.len() != Self::SIZE as usize {
            ctx.report(owner, id, 0, DiagKind::ShortChunk);
            return out;
        }
        let mut cur = Cursor::new(data);
        for slot in [
            &mut out.weapon_id,
            &mut out.shield_id,
            &mut out.armor_id,
            &mut out.helmet_id,
            &mut out.accessory_id,
        ] {
            *slot = cur.i16_le().unwrap_or(0);
        }
        out
    }

    #[inline]
    fn lcf_size(&self, _engine: Engine) -> u32 {
        Self::SIZE
    }

    fn write<S: Sink>(&self, sink: &mut S, _engine: Engine) -> Result<(), S::Error> {
        for value in [
            self.weapon_id,
            self.shield_id,
            self.armor_id,
            self.helmet_id,
            self.accessory_id,
        ] {
            sink.write(&value.to_le_bytes())?;
        }
        Ok(())
    }
}

/// Six parallel stat curves, stored end to end with no separators.
///
/// The payload is split six ways by length: each curve gets `len / 6` bytes. There is nothing in the encoding
/// that says how long a curve is, so a payload that is not a multiple of twelve silently loses its tail -
/// as it does in liblcf.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Parameters {
    pub maxhp: Packed<i16>,
    pub maxsp: Packed<i16>,
    pub attack: Packed<i16>,
    pub defense: Packed<i16>,
    pub spirit: Packed<i16>,
    pub agility: Packed<i16>,
}

impl Parameters {
    /// The six curves, in wire order.
    const fn curves(&self) -> [&Packed<i16>; 6] {
        [
            &self.maxhp,
            &self.maxsp,
            &self.attack,
            &self.defense,
            &self.spirit,
            &self.agility,
        ]
    }
}

impl<'a> LcfField<'a> for Parameters {
    #[inline]
    fn field_default(_engine: Engine) -> Self {
        Self::default()
    }

    fn read(data: &'a [u8], ctx: &mut Ctx<'_>, owner: &'static str, id: u32) -> Self {
        let per = data.len() / 6;
        if data.len() % 6 != 0 {
            ctx.report(owner, id, 0, DiagKind::RaggedArray);
        }
        let mut out = Self::default();
        for (index, slot) in [
            &mut out.maxhp,
            &mut out.maxsp,
            &mut out.attack,
            &mut out.defense,
            &mut out.spirit,
            &mut out.agility,
        ]
        .into_iter()
        .enumerate()
        {
            let start = index * per;
            let chunk = data.get(start..start + per).unwrap_or(&[]);
            *slot = <Packed<i16> as LcfField>::read(chunk, ctx, owner, id);
        }
        out
    }

    fn lcf_size(&self, engine: Engine) -> u32 {
        self.curves().iter().map(|c| c.lcf_size(engine)).sum()
    }

    fn write<S: Sink>(&self, sink: &mut S, engine: Engine) -> Result<(), S::Error> {
        for curve in self.curves() {
            curve.write(sink, engine)?;
        }
        Ok(())
    }
}

/// One event command, owning its string and parameters.
///
/// The borrowing counterpart is [`EventCommandRef`](crate::event::EventCommandRef), which needs
/// no allocator.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EventCommand<'a> {
    /// The command code.
    pub code: i32,
    /// Nesting depth inside conditional branches and loops.
    pub indent: i32,
    /// The string operand, verbatim.
    pub string: DbStr<'a>,
    /// The integer operands.
    pub parameters: Vec<i32>,
}

impl EventCommand<'_> {
    fn wire_len(&self) -> u32 {
        crate::event::event_command_len(self.code, self.indent, self.string.len(), &self.parameters)
    }

    fn write_one<S: Sink>(&self, sink: &mut S) -> Result<(), S::Error> {
        wire::write_varint(sink, self.code as u32)?;
        wire::write_varint(sink, self.indent as u32)?;
        wire::write_varint(sink, self.string.len() as u32)?;
        sink.write(self.string.as_bytes())?;
        wire::write_varint(sink, self.parameters.len() as u32)?;
        for &param in &self.parameters {
            wire::write_varint(sink, param as u32)?;
        }
        Ok(())
    }
}

/// A command list, terminated on the wire by four zero bytes.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EventCommandList<'a>(pub Vec<EventCommand<'a>>);

impl<'a> core::ops::Deref for EventCommandList<'a> {
    type Target = Vec<EventCommand<'a>>;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> LcfField<'a> for EventCommandList<'a> {
    #[inline]
    fn field_default(_engine: Engine) -> Self {
        Self(Vec::new())
    }

    fn read(data: &'a [u8], ctx: &mut Ctx<'_>, _owner: &'static str, _id: u32) -> Self {
        let mut out = Vec::new();
        let mut iter = EventCommandIter::new(data, 0);
        while let Some(cmd) = iter.next(ctx) {
            out.push(EventCommand {
                code: cmd.code,
                indent: cmd.indent,
                string: DbStr::from_bytes(cmd.string),
                parameters: cmd.params().collect(),
            });
        }
        Self(out)
    }

    fn lcf_size(&self, _engine: Engine) -> u32 {
        // Four zero bytes terminate the list.
        self.0.iter().map(EventCommand::wire_len).sum::<u32>() + 4
    }

    fn write<S: Sink>(&self, sink: &mut S, _engine: Engine) -> Result<(), S::Error> {
        for cmd in &self.0 {
            cmd.write_one(sink)?;
        }
        sink.write(&[0, 0, 0, 0])
    }

    #[inline]
    fn element_count(&self) -> u32 {
        self.0.len() as u32
    }
}

/// One move-route command, owning its string.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MoveCommand<'a> {
    /// The command code. See [`move_code`](crate::event::move_code).
    pub code: i32,
    /// The string operand, verbatim; empty for codes that carry none.
    pub string: DbStr<'a>,
    /// First integer operand.
    pub parameter_a: i32,
    /// Second integer operand.
    pub parameter_b: i32,
    /// Third integer operand.
    pub parameter_c: i32,
}

impl MoveCommand<'_> {
    /// How many integer operands this command's code carries.
    const fn int_count(&self) -> usize {
        match self.code {
            move_code::SWITCH_ON | move_code::SWITCH_OFF | move_code::CHANGE_GRAPHIC => 1,
            move_code::PLAY_SOUND_EFFECT => 3,
            _ => 0,
        }
    }

    /// Whether this command's code carries a string operand.
    const fn has_string(&self) -> bool {
        matches!(self.code, move_code::CHANGE_GRAPHIC | move_code::PLAY_SOUND_EFFECT)
    }

    fn wire_len(&self) -> u32 {
        let mut total = wire::varint_len(self.code as u32);
        if self.has_string() {
            total += wire::varint_len(self.string.len() as u32) + self.string.len() as u32;
        }
        for value in [self.parameter_a, self.parameter_b, self.parameter_c]
            .into_iter()
            .take(self.int_count())
        {
            total += wire::varint_len(value as u32);
        }
        total
    }

    fn write_one<S: Sink>(&self, sink: &mut S) -> Result<(), S::Error> {
        wire::write_varint(sink, self.code as u32)?;
        if self.has_string() {
            wire::write_varint(sink, self.string.len() as u32)?;
            sink.write(self.string.as_bytes())?;
        }
        for value in [self.parameter_a, self.parameter_b, self.parameter_c]
            .into_iter()
            .take(self.int_count())
        {
            wire::write_varint(sink, value as u32)?;
        }
        Ok(())
    }
}

/// A move route's commands, running to the end of the chunk with no count and no terminator.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MoveCommandList<'a>(pub Vec<MoveCommand<'a>>);

impl<'a> core::ops::Deref for MoveCommandList<'a> {
    type Target = Vec<MoveCommand<'a>>;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> LcfField<'a> for MoveCommandList<'a> {
    #[inline]
    fn field_default(_engine: Engine) -> Self {
        Self(Vec::new())
    }

    fn read(data: &'a [u8], ctx: &mut Ctx<'_>, _owner: &'static str, _id: u32) -> Self {
        let mut out = Vec::new();
        let mut iter = MoveCommandIter::new(data, 0);
        while let Some(cmd) = iter.next(ctx) {
            out.push(MoveCommand {
                code: cmd.code,
                string: DbStr::from_bytes(cmd.string),
                parameter_a: cmd.parameter_a,
                parameter_b: cmd.parameter_b,
                parameter_c: cmd.parameter_c,
            });
        }
        Self(out)
    }

    fn lcf_size(&self, _engine: Engine) -> u32 {
        self.0.iter().map(MoveCommand::wire_len).sum()
    }

    fn write<S: Sink>(&self, sink: &mut S, _engine: Engine) -> Result<(), S::Error> {
        for cmd in &self.0 {
            cmd.write_one(sink)?;
        }
        Ok(())
    }

    #[inline]
    fn element_count(&self) -> u32 {
        self.0.len() as u32
    }
}

/// A sparse list of strings, run-length encoded over its empty entries.
///
/// The oddest encoding in the format, and the only reason a 64-bit compressed integer exists at
/// all. Each entry is a length followed by that many bytes - but a length **above `u32::MAX`**
/// is not a length at all: it is the sentinel `0x8_0000_0000 - gap`, meaning "skip `gap` empty
/// entries". Used by exactly one field in the schema, an EasyRPG extension to `SaveSystem`.
///
/// The encoding does not actually save space for short runs, and liblcf uses it anyway. Because
/// the sentinel is `2^35 - gap`, it needs all five bytes of a compressed integer no matter how
/// small the gap, while the empty entries it replaces would have cost one byte each - so a run
/// shorter than six *grows*. Reproduced regardless: the goal is byte-identical output, not a
/// better encoding.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct StrList<'a>(pub Vec<DbStr<'a>>);

impl<'a> core::ops::Deref for StrList<'a> {
    type Target = Vec<DbStr<'a>>;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// The sentinel base a gap marker is subtracted from.
const GAP_BASE: u64 = 0x8_0000_0000;

impl<'a> LcfField<'a> for StrList<'a> {
    #[inline]
    fn field_default(_engine: Engine) -> Self {
        Self(Vec::new())
    }

    fn read(data: &'a [u8], ctx: &mut Ctx<'_>, owner: &'static str, id: u32) -> Self {
        let mut out: Vec<DbStr<'a>> = Vec::new();
        let mut cur = Cursor::new(data);

        while !cur.is_empty() {
            let Some(marker) = cur.varint64() else {
                ctx.report(owner, id, cur.offset(), DiagKind::ShortChunk);
                break;
            };

            if marker.value > u64::from(u32::MAX) {
                // A gap: extend with empty entries rather than storing them.
                let gap = GAP_BASE.saturating_sub(marker.value);
                // Bounded so a hostile marker cannot ask for a huge allocation.
                let Ok(gap) = usize::try_from(gap) else {
                    ctx.report(owner, id, cur.offset(), DiagKind::BadVarint);
                    break;
                };
                let Some(target) = out.len().checked_add(gap) else {
                    ctx.report(owner, id, cur.offset(), DiagKind::BadVarint);
                    break;
                };
                if gap > data.len() {
                    ctx.report(owner, id, cur.offset(), DiagKind::BadVarint);
                    break;
                }
                out.resize(target, DbStr::default());
                continue;
            }

            let Some(bytes) = cur.take(marker.value as usize) else {
                ctx.report(owner, id, cur.offset(), DiagKind::ShortChunk);
                break;
            };
            out.push(DbStr::from_bytes(bytes));
        }

        Self(out)
    }

    fn lcf_size(&self, _engine: Engine) -> u32 {
        let mut total = 0u32;
        let mut gap = 0u64;
        for entry in &self.0 {
            if entry.is_empty() {
                gap += 1;
                continue;
            }
            if gap > 0 {
                total += wire::varint64_len(GAP_BASE - gap);
                gap = 0;
            }
            let len = entry.len() as u32;
            total += wire::varint_len(len) + len;
        }
        // A trailing run of empties is dropped, exactly as liblcf's writer drops it.
        total
    }

    fn write<S: Sink>(&self, sink: &mut S, _engine: Engine) -> Result<(), S::Error> {
        let mut gap = 0u64;
        for entry in &self.0 {
            if entry.is_empty() {
                gap += 1;
                continue;
            }
            if gap > 0 {
                wire::write_varint64(sink, GAP_BASE - gap)?;
                gap = 0;
            }
            wire::write_varint(sink, entry.len() as u32)?;
            sink.write(entry.as_bytes())?;
        }
        Ok(())
    }

    #[inline]
    fn element_count(&self) -> u32 {
        self.0.len() as u32
    }
}

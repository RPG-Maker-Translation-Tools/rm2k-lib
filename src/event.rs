//! Allocation-free iteration over event and move-route command lists.
//!
//! [`EventCommandIter`] and [`MoveCommandIter`] walk a command list and hand back each command
//! as a borrowed view into the input buffer - no `Vec`, no string copies, usable with no
//! allocator at all. This is the tool for an embedded consumer that just needs to walk a map's
//! events, match on command codes, and read strings.
//!
//! Event command lists are the format's oddest corner. They carry **no element count and no
//! length prefix of their own** - the list ends at four consecutive zero bytes, which is what a
//! command with `code == 0` looks like. Each command is:
//!
//! ```text
//! varint(code) varint(indent) varint(len) len*u8 varint(n) n*varint
//! ```
//!
//! Move routes are framed differently again: no terminator and no count, they simply run to the
//! end of the enclosing chunk, and each command's payload shape depends on its own code.

use crate::{
    cursor::{ChunkIter, Ctx, Cursor, ListWalker},
    diag::DiagKind,
    wire,
};

/// One event command, borrowed from the input buffer.
#[derive(Clone, Debug)]
pub struct EventCommandRef<'a> {
    /// The command code.
    pub code: i32,
    /// Nesting depth inside conditional branches and loops.
    pub indent: i32,
    /// The command's string operand, **verbatim**. Never transcoded; see the crate docs.
    pub string: &'a [u8],
    /// The raw parameter bytes, decoded lazily by [`params`](EventCommandRef::params).
    params: &'a [u8],
    /// How many parameters are actually present in `params`. Not necessarily what the header
    /// declared: a truncated/corrupted file can declare far more than the remaining bytes could
    /// ever hold, so this is capped to how many were actually scanned out of the buffer (see
    /// [`EventCommandIter::next`]) rather than trusted verbatim from the wire.
    param_count: u32,
}

impl<'a> EventCommandRef<'a> {
    /// Iterate the command's integer parameters.
    ///
    /// Decoded on demand rather than collected: most consumers look at one or two parameters of
    /// the handful of command codes they care about, and never touch the rest.
    #[must_use]
    #[inline]
    pub const fn params(&self) -> ParamIter<'a> {
        ParamIter {
            cur: Cursor::new(self.params),
            left: self.param_count,
        }
    }

    /// The parameter at `index`, or `None` if the command has fewer.
    ///
    /// `O(index)`, because compressed integers are not fixed width and cannot be indexed
    /// directly. Fine for the small indices real code uses; iterate with
    /// [`params`](EventCommandRef::params) if you want them all.
    #[must_use]
    #[inline]
    pub fn param(&self, index: usize) -> Option<i32> {
        self.params().nth(index)
    }

    /// How many parameters are actually present (see [`EventCommandRef`]'s
    /// `param_count` field docs — capped to what the buffer could actually hold,
    /// not trusted verbatim from the header).
    #[must_use]
    #[inline]
    pub const fn param_count(&self) -> u32 {
        self.param_count
    }

    /// The undecoded parameter bytes.
    ///
    /// For callers that need to carry the parameters somewhere before decoding them - the C API
    /// hands this span back so a later `rm2k_event_command_param` can decode on demand without
    /// keeping the iterator alive.
    #[must_use]
    #[inline]
    pub const fn raw_params(&self) -> &'a [u8] {
        self.params
    }
}

/// Lazily decodes an event command's integer parameters.
#[derive(Clone, Debug)]
pub struct ParamIter<'a> {
    cur: Cursor<'a>,
    left: u32,
}

impl Iterator for ParamIter<'_> {
    type Item = i32;

    #[inline]
    fn next(&mut self) -> Option<i32> {
        if self.left == 0 {
            return None;
        }
        self.left -= 1;
        // A truncated tail yields 0
        Some(self.cur.varint().map_or(0, |v| v.value as i32))
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.left as usize, Some(self.left as usize))
    }
}

impl ExactSizeIterator for ParamIter<'_> {}

/// Walks an event command list.
///
/// Not an [`Iterator`], for the same reason as
/// [`ChunkIter`](crate::cursor::ChunkIter): advancing needs a [`Ctx`] to report recovery into,
/// and `Iterator::next` has nowhere to put one.
#[derive(Clone, Debug)]
pub struct EventCommandIter<'a> {
    cur: Cursor<'a>,
    done: bool,
}

impl<'a> EventCommandIter<'a> {
    /// Walk the command list in a chunk payload beginning at file offset `base`.
    #[must_use]
    #[inline]
    pub const fn new(data: &'a [u8], base: u32) -> Self {
        Self {
            cur: Cursor::with_base(data, base),
            done: false,
        }
    }

    /// Bytes left after the list terminated.
    #[must_use]
    #[inline]
    pub fn tail(&self) -> &'a [u8] {
        self.cur.remaining()
    }

    /// Yield the next command, or `None` at the terminator or end of payload.
    #[must_use]
    pub fn next(&mut self, ctx: &mut Ctx<'_>) -> Option<EventCommandRef<'a>> {
        if self.done || self.cur.is_empty() {
            self.done = true;
            return None;
        }

        // A zero code is the terminator. The list ends with four zero bytes, being a command
        // whose code, indent, string length and parameter count are all zero.
        if self.cur.peek() == Some(0) {
            self.cur.skip(4);
            self.done = true;
            return None;
        }

        let offset = self.cur.offset();
        let Some(code) = self.cur.varint() else {
            self.recover(ctx, offset);
            return None;
        };
        let Some(indent) = self.cur.varint() else {
            self.recover(ctx, offset);
            return None;
        };
        let Some(string) = self.cur.lstring() else {
            self.recover(ctx, offset);
            return None;
        };
        let Some(param_count) = self.cur.varint() else {
            self.recover(ctx, offset);
            return None;
        };

        // Parameters are compressed integers, so their extent has to be found by scanning. Doing
        // it once here keeps `ParamIter` a plain decode with no bounds bookkeeping.
        //
        // `param_count.value` is attacker-controlled and must not be trusted verbatim: a
        // corrupted/truncated file can declare far more parameters than the remaining bytes could
        // ever hold. `ParamIter` implements `ExactSizeIterator` off of this count, so a caller
        // doing `.collect::<Vec<_>>()` (or anything else that pre-reserves off `size_hint`) would
        // otherwise try to allocate for the declared count before reading a single parameter -
        // this loop already knows the *actual* count (it stops the moment `varint()` runs out of
        // data), so track that instead of the raw header value.
        let params_start = self.cur.pos();
        let mut actual_count = 0u32;
        for _ in 0..param_count.value {
            if self.cur.varint().is_none() {
                ctx.report("EventCommand", 0, self.cur.offset(), DiagKind::ShortChunk);
                break;
            }
            actual_count += 1;
        }
        let params_end = self.cur.pos();
        let params = self.cur.buf().get(params_start..params_end).unwrap_or(&[]);

        Some(EventCommandRef {
            code: code.value as i32,
            indent: indent.value as i32,
            string,
            params,
            param_count: actual_count,
        })
    }

    /// Resynchronize after a malformed command.
    ///
    /// There is nothing better available than scan forward for 4 zero bytes: the list has no framing to fall back on.
    fn recover(&mut self, ctx: &mut Ctx<'_>, offset: u32) {
        ctx.report("EventCommand", 0, offset, DiagKind::EventListCorrupt);
        let mut zeros = 0;
        while let Some(byte) = self.cur.u8() {
            zeros = if byte == 0 { zeros + 1 } else { 0 };
            if zeros == 4 {
                break;
            }
        }
        self.done = true;
    }
}

/// One move-route command, borrowed from the input buffer.
///
/// Unlike an event command, a move command's payload shape is decided by its own code:
/// most codes carry nothing at all, two carry one integer, and
/// two carry a string followed by one or three integers.
#[derive(Clone, Debug, Default)]
pub struct MoveCommandRef<'a> {
    /// The command code. See [`move_code`] for the ones that carry operands.
    pub code: i32,
    /// The string operand, verbatim. Empty for every code that does not carry one.
    pub string: &'a [u8],
    /// First integer operand, or `0`.
    pub parameter_a: i32,
    /// Second integer operand, or `0`.
    pub parameter_b: i32,
    /// Third integer operand, or `0`.
    pub parameter_c: i32,
}

/// The move-route command codes whose payload is not empty.
///
/// Only these four change how many bytes a move command occupies, which is why they are the only
/// ones the codec needs to name.
pub mod move_code {
    /// Turns a switch on; carries one integer.
    pub const SWITCH_ON: i32 = 32;
    /// Turns a switch off; carries one integer.
    pub const SWITCH_OFF: i32 = 33;
    /// Changes the event's sprite; carries a string and one integer.
    pub const CHANGE_GRAPHIC: i32 = 34;
    /// Plays a sound effect; carries a string and three integers.
    pub const PLAY_SOUND_EFFECT: i32 = 35;
}

/// Walks a move route's command list.
///
/// The list has neither a count nor a terminator: it runs to the end of the enclosing chunk.
#[derive(Clone, Debug)]
pub struct MoveCommandIter<'a> {
    cur: Cursor<'a>,
}

impl<'a> MoveCommandIter<'a> {
    /// Walk the move commands in a chunk payload beginning at file offset `base`.
    #[must_use]
    #[inline]
    pub const fn new(data: &'a [u8], base: u32) -> Self {
        Self {
            cur: Cursor::with_base(data, base),
        }
    }

    /// Yield the next command, or `None` at the end of the payload.
    #[must_use]
    pub fn next(&mut self, ctx: &mut Ctx<'_>) -> Option<MoveCommandRef<'a>> {
        if self.cur.is_empty() {
            return None;
        }

        let offset = self.cur.offset();
        let code = self.cur.varint()?.value as i32;
        let mut out = MoveCommandRef {
            code,
            ..MoveCommandRef::default()
        };

        let ok = match code {
            move_code::SWITCH_ON | move_code::SWITCH_OFF => self.read_ints(&mut out, 1),
            move_code::CHANGE_GRAPHIC => self.read_string(&mut out) && self.read_ints(&mut out, 1),
            move_code::PLAY_SOUND_EFFECT => self.read_string(&mut out) && self.read_ints(&mut out, 3),
            _ => true,
        };

        if !ok {
            ctx.report("MoveCommand", 0, offset, DiagKind::ShortChunk);
        }
        Some(out)
    }

    fn read_string(&mut self, out: &mut MoveCommandRef<'a>) -> bool {
        match self.cur.lstring() {
            Some(s) => {
                out.string = s;
                true
            }
            None => false,
        }
    }

    fn read_ints(&mut self, out: &mut MoveCommandRef<'a>, count: usize) -> bool {
        for slot in [&mut out.parameter_a, &mut out.parameter_b, &mut out.parameter_c]
            .into_iter()
            .take(count)
        {
            match self.cur.varint() {
                Some(v) => *slot = v.value as i32,
                None => return false,
            }
        }
        true
    }
}

/// Encoded length of one event command, including its four-byte terminator contribution of zero.
///
/// Exposed so a caller sizing a buffer for [`SliceSink`](crate::sink::SliceSink) can do it
/// without allocating.
#[must_use]
pub fn event_command_len(code: i32, indent: i32, string_len: usize, params: &[i32]) -> u32 {
    let mut total = wire::varint_len(code as u32)
        + wire::varint_len(indent as u32)
        + wire::varint_len(string_len as u32)
        + string_len as u32
        + wire::varint_len(params.len() as u32);
    for &p in params {
        total += wire::varint_len(p as u32);
    }
    total
}

/// The chunk ids on the route from a file root down to a command list.
///
/// Reaching an event command means walking `Map` -> `Event` -> `EventPage` -> commands, and the core
/// tier has no typed structs to do it with - those need `alloc`. Rather than make an embedded
/// caller hardcode four magic numbers, the handful actually on that route are named here.
///
/// Deliberately not the full set: liblcf generates 2 960 lines of chunk-id enums across four
/// `chunks.h` files, and nothing in the core tier needs the other ~1 190 ids. If you are reaching
/// for a chunk that is not here, you want the `alloc` tier's typed structs instead.
pub mod path {
    /// `Map::events` - a list of `Event`.
    pub const MAP_EVENTS: u32 = 0x51;
    /// `Event::pages` - a list of `EventPage`.
    pub const EVENT_PAGES: u32 = 0x05;
    /// `EventPage::event_commands`.
    pub const EVENT_PAGE_COMMANDS: u32 = 0x34;

    /// `Database::commonevents` - a list of `CommonEvent`.
    pub const DATABASE_COMMON_EVENTS: u32 = 0x19;
    /// `CommonEvent::event_commands`.
    pub const COMMON_EVENT_COMMANDS: u32 = 0x16;

    /// `Database::troops` - a list of `Troop`.
    pub const DATABASE_TROOPS: u32 = 0x0F;
    /// `Troop::pages` - a list of `TroopPage`.
    pub const TROOP_PAGES: u32 = 0x0B;
    /// `TroopPage::event_commands`.
    pub const TROOP_PAGE_COMMANDS: u32 = 0x0C;
}

/// Where a command list was found.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommandListAt<'a> {
    /// Id of the owning event, common event or troop.
    pub owner_id: i32,
    /// Id of the owning page, or `0` for a common event, which has no pages.
    pub page_id: i32,
    /// The command list's payload.
    pub commands: &'a [u8],
    /// File-absolute offset of `commands`.
    pub offset: u32,
}

impl<'a> CommandListAt<'a> {
    /// Walk the commands.
    ///
    /// Not called `iter`: the result is not an [`Iterator`], because advancing needs a [`Ctx`] to
    /// report recovery into and `Iterator::next` has nowhere to put one.
    #[must_use]
    #[inline]
    pub const fn commands(&self) -> EventCommandIter<'a> {
        EventCommandIter::new(self.commands, self.offset)
    }
}

/// Which nesting shape a root's command lists have.
#[derive(Clone, Copy, Debug)]
enum Shape {
    /// Root -> list -> page -> commands (maps, troops).
    Paged { outer: u32, pages: u32, commands: u32 },
    /// Root -> list -> commands (common events).
    Flat { outer: u32, commands: u32 },
}

/// Finds every event command list under a file root, without allocating.
///
/// This is the piece that makes the core tier usable for real work. liblcf cannot express it at
/// all: reaching one map's commands there means parsing the entire `rpg::Map` into `std::vector`s
/// first, copying every string in the file on the way. Here the whole traversal is spans into the
/// caller's buffer, so it runs on a microcontroller against a memory-mapped file.
///
/// ```
/// # use rm2k::{event::CommandLists, Ctx, Engine, NullSink};
/// # let map_body: &[u8] = &[0];
/// let mut sink = NullSink;
/// let mut ctx = Ctx::new(Engine::R2K, &mut sink);
/// let mut lists = CommandLists::in_map(map_body, 0, &mut ctx);
/// while let Some(list) = lists.next(&mut ctx) {
///     let mut commands = list.commands();
///     while let Some(command) = commands.next(&mut ctx) {
///         let _ = (command.code, command.string);
///     }
/// }
/// ```
#[derive(Clone, Debug)]
pub struct CommandLists<'a> {
    shape: Shape,
    outer: Option<ListWalker<'a>>,
    inner: Option<ListWalker<'a>>,
    owner_id: i32,
    name: &'static str,
}

impl<'a> CommandLists<'a> {
    /// Every command list in a map, from its root chunk stream.
    ///
    /// `body` is the map's chunk stream - what
    /// [`load_map`](crate::file::load_map) parses, minus the file header.
    #[must_use]
    pub fn in_map(body: &'a [u8], base: u32, ctx: &mut Ctx<'_>) -> Self {
        Self::start(
            body,
            base,
            ctx,
            "Map",
            Shape::Paged {
                outer: path::MAP_EVENTS,
                pages: path::EVENT_PAGES,
                commands: path::EVENT_PAGE_COMMANDS,
            },
        )
    }

    /// Every common event's command list, from a database's chunk stream.
    #[must_use]
    pub fn in_common_events(body: &'a [u8], base: u32, ctx: &mut Ctx<'_>) -> Self {
        Self::start(
            body,
            base,
            ctx,
            "CommonEvent",
            Shape::Flat {
                outer: path::DATABASE_COMMON_EVENTS,
                commands: path::COMMON_EVENT_COMMANDS,
            },
        )
    }

    /// Every troop page's command list, from a database's chunk stream.
    #[must_use]
    pub fn in_battle_events(body: &'a [u8], base: u32, ctx: &mut Ctx<'_>) -> Self {
        Self::start(
            body,
            base,
            ctx,
            "Troop",
            Shape::Paged {
                outer: path::DATABASE_TROOPS,
                pages: path::TROOP_PAGES,
                commands: path::TROOP_PAGE_COMMANDS,
            },
        )
    }

    fn start(body: &'a [u8], base: u32, ctx: &mut Ctx<'_>, name: &'static str, shape: Shape) -> Self {
        let outer_id = match shape {
            Shape::Paged { outer, .. } | Shape::Flat { outer, .. } => outer,
        };
        let outer = ChunkIter::find(body, base, outer_id, ctx, name)
            .map(|chunk| ListWalker::new(chunk.data, chunk.offset, ctx, name));
        Self {
            shape,
            outer,
            inner: None,
            owner_id: 0,
            name,
        }
    }

    /// Yield the next command list.
    #[must_use]
    pub fn next(&mut self, ctx: &mut Ctx<'_>) -> Option<CommandListAt<'a>> {
        loop {
            // Drain the current owner's pages before advancing to the next owner.
            if let Some(inner) = self.inner.as_mut() {
                if let Some(page) = inner.next(ctx) {
                    let Shape::Paged { commands, .. } = self.shape else {
                        unreachable!("inner walker only exists for a paged shape")
                    };
                    if let Some(chunk) = page.find(commands, ctx, self.name) {
                        return Some(CommandListAt {
                            owner_id: self.owner_id,
                            page_id: page.id,
                            commands: chunk.data,
                            offset: chunk.offset,
                        });
                    }
                    continue;
                }
                self.inner = None;
            }

            let owner = self.outer.as_mut()?.next(ctx)?;
            self.owner_id = owner.id;

            match self.shape {
                Shape::Flat { commands, .. } => {
                    if let Some(chunk) = owner.find(commands, ctx, self.name) {
                        return Some(CommandListAt {
                            owner_id: owner.id,
                            page_id: 0,
                            commands: chunk.data,
                            offset: chunk.offset,
                        });
                    }
                }
                Shape::Paged { pages, .. } => {
                    if let Some(chunk) = owner.find(pages, ctx, self.name) {
                        self.inner = Some(ListWalker::new(chunk.data, chunk.offset, ctx, self.name));
                    }
                }
            }
        }
    }
}

//! The `LMT` root, the only root that is not a chunk stream.

use crate::{
    cursor::{ChunkIter, Ctx, Cursor},
    engine::Engine,
    field::{LcfStruct, List},
    rpg::{MapInfo, Start},
    {sink::Sink, wire},
};
use alloc::vec::Vec;

#[derive(Clone, Debug, PartialEq)]
pub struct TreeMap<'a> {
    pub maps: List<MapInfo<'a>>,
    pub tree_order: Vec<i32>,
    pub active_node: i32,
    pub start: Start,
}

impl TreeMap<'_> {
    #[must_use]
    pub fn defaults(engine: Engine) -> Self {
        Self {
            maps: List(Vec::new()),
            tree_order: Vec::new(),
            active_node: 0,
            start: Start::defaults(engine),
        }
    }
}

impl<'a> TreeMap<'a> {
    #[must_use]
    pub fn read_inline(cur: &mut Cursor<'a>, ctx: &mut Ctx<'_>) -> Self {
        const NAME: &str = "TreeMap";

        let maps = List::read_inline(cur, ctx, NAME, 0);

        let count = cur.varint_lenient(ctx, NAME, 0);
        // Each entry is at least one byte, which bounds a hostile count.
        let count = (count.max(0) as usize).min(cur.remaining_len());
        let mut tree_order = Vec::with_capacity(count);
        for _ in 0..count {
            tree_order.push(cur.varint_lenient(ctx, NAME, 0));
        }

        let active_node = cur.varint_lenient(ctx, NAME, 0);

        let mut chunks = ChunkIter::from_cursor(cur.clone());
        let start = Start::read_stream(&mut chunks, ctx);
        *cur = chunks.into_cursor();

        Self {
            maps,
            tree_order,
            active_node,
            start,
        }
    }

    pub fn write_inline<S: Sink>(&self, sink: &mut S, engine: Engine) -> Result<(), S::Error> {
        crate::field::LcfField::write(&self.maps, sink, engine)?;

        wire::write_varint(sink, self.tree_order.len() as u32)?;
        for &id in &self.tree_order {
            wire::write_varint(sink, id as u32)?;
        }

        wire::write_varint(sink, self.active_node as u32)?;

        self.start.write_body(sink, engine)?;
        sink.write(&[0])
    }
}

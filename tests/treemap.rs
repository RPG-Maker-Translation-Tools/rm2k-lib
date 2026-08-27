//! Tests for `rm2k::rpg::TreeMap` - the hand-written root type for `LMT` files, which is not a
//! chunk stream and so is not built from the schema macros.

use rm2k::{Ctx, Cursor, Engine, NullSink, rpg::TreeMap};

#[test]
fn round_trips_with_maps_and_an_order() {
    let mut tree = TreeMap::defaults(Engine::R2K);
    tree.tree_order = vec![1, 2, 3];
    tree.active_node = 2;

    let mut buf = Vec::new();
    tree.write_inline(&mut buf, Engine::R2K).unwrap();

    let mut sink = NullSink;
    let mut ctx = Ctx::new(Engine::R2K, &mut sink);
    let mut cur = Cursor::new(&buf);
    assert_eq!(TreeMap::read_inline(&mut cur, &mut ctx), tree);
    assert!(cur.is_empty(), "the whole payload must be consumed");
}

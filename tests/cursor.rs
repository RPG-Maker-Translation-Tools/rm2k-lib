//! Tests for `rm2k::cursor` - `Cursor`, `Ctx`, `Chunk` and `ChunkIter`.

use rm2k::{
    ChunkIter, Ctx, Cursor, Engine, NullSink, SliceSink,
    diag::{DiagKind, Diagnostic, StrictSink},
};

fn ctx_null(sink: &mut NullSink) -> Ctx<'_> {
    Ctx::new(Engine::R2K, sink)
}

#[test]
fn chunk_iter_frames_and_terminates() {
    // id 0x01, len 3, "abc" | id 0x02, len 1, 0x07 | terminator 0x00
    let buf = [0x01, 0x03, b'a', b'b', b'c', 0x02, 0x01, 0x07, 0x00];
    let mut sink = NullSink;
    let mut ctx = ctx_null(&mut sink);
    let mut iter = ChunkIter::new(&buf, 0);

    let c1 = iter.next(&mut ctx, "T").unwrap();
    assert_eq!(c1.id, 1);
    assert_eq!(c1.data, b"abc");
    assert_eq!(c1.offset, 2);

    let c2 = iter.next(&mut ctx, "T").unwrap();
    assert_eq!(c2.id, 2);
    assert_eq!(c2.data, &[0x07]);

    assert!(iter.next(&mut ctx, "T").is_none());
}

#[test]
fn chunk_payload_is_sliced_so_a_field_cannot_overrun() {
    // A field that greedily reads varints out of chunk 1 must not reach chunk 2's bytes.
    let buf = [0x01, 0x01, 0x05, 0x02, 0x01, 0x63, 0x00];
    let mut sink = NullSink;
    let mut ctx = ctx_null(&mut sink);
    let mut iter = ChunkIter::new(&buf, 0);

    let c1 = iter.next(&mut ctx, "T").unwrap();
    let mut cur = c1.cursor();
    assert_eq!(cur.varint().unwrap().value, 5);
    assert!(cur.varint().is_none(), "must not see the following chunk");
}

#[test]
fn overrunning_chunk_is_truncated_and_reported() {
    // Chunk 1 claims 99 bytes but only 2 follow.
    let buf = [0x01, 0x63, 0xAA, 0xBB];
    let mut slots = [Diagnostic::EMPTY; 4];
    let mut sink = SliceSink::new(&mut slots);
    // Scoped so the context releases its borrow of `sink` before the assertions read it.
    {
        let mut ctx = Ctx::new(Engine::R2K, &mut sink);
        let mut iter = ChunkIter::new(&buf, 0);

        let c = iter.next(&mut ctx, "T").unwrap();
        assert_eq!(c.data, &[0xAA, 0xBB]);
        assert!(iter.next(&mut ctx, "T").is_none());
    }
    assert_eq!(sink.as_slice().len(), 1);
    assert_eq!(sink.as_slice()[0].kind, DiagKind::ChunkOverrun);
}

#[test]
fn offsets_stay_file_absolute_inside_a_nested_chunk() {
    let buf = [0x01, 0x02, 0x09, 0x08, 0x00];
    let mut sink = NullSink;
    let mut ctx = ctx_null(&mut sink);
    let mut iter = ChunkIter::new(&buf, 100);
    let c = iter.next(&mut ctx, "T").unwrap();
    assert_eq!(c.offset, 102);
    let mut cur = c.cursor();
    assert_eq!(cur.offset(), 102);
    let _ = cur.u8();
    assert_eq!(cur.offset(), 103);
}

#[test]
fn lstring_returns_raw_bytes() {
    // Length 4 followed by Shift-JIS bytes that are not valid UTF-8; they come back verbatim.
    let buf = [0x04, 0x82, 0xA0, 0x82, 0xA2];
    let mut cur = Cursor::new(&buf);
    assert_eq!(cur.lstring(), Some(&[0x82, 0xA0, 0x82, 0xA2][..]));
}

#[test]
fn strict_sink_latches_first_problem() {
    let buf = [0x01, 0x63, 0xAA];
    let mut sink = StrictSink::new();
    let mut ctx = Ctx::new(Engine::R2K, &mut sink);
    let mut iter = ChunkIter::new(&buf, 0);
    let _ = iter.next(&mut ctx, "T");
    assert!(ctx.should_abort());
    assert_eq!(ctx.latched().unwrap().kind, DiagKind::ChunkOverrun);
}

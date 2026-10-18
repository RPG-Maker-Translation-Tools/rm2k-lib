//! Tests for `rm2k::cursor` - `Cursor`, `Ctx`, `Chunk` and `ChunkIter`.

use rm2k::{
    ChunkIter, Ctx, Cursor, Engine, ListWalker, NullSink, SliceSink,
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

#[test]
fn a_reborrowed_context_reports_into_the_same_sink() {
    let mut slots = [Diagnostic::EMPTY; 2];
    let mut sink = SliceSink::new(&mut slots);
    {
        let mut ctx = Ctx::new(Engine::R2K, &mut sink);
        let mut inner = ctx.reborrow();
        inner.report("T", 1, 0, DiagKind::ShortChunk);
    }
    assert_eq!(sink.as_slice().len(), 1);
}

#[test]
fn ctx_debug_names_the_engine() {
    let mut sink = NullSink;
    let ctx = ctx_null(&mut sink);
    assert_eq!(format!("{ctx:?}"), "Ctx { engine: R2K, .. }");
}

#[test]
fn an_overlong_varint_reads_as_zero_and_is_reported() {
    let buf = [0x80, 0x80, 0x80, 0x80, 0x80, 0x01];
    let mut slots = [Diagnostic::EMPTY; 2];
    let mut sink = SliceSink::new(&mut slots);
    let value = {
        let mut ctx = Ctx::new(Engine::R2K, &mut sink);
        let mut cur = Cursor::new(&buf);
        cur.varint_lenient(&mut ctx, "T", 1)
    };
    assert_eq!(value, 0);
    assert_eq!(sink.as_slice()[0].kind, DiagKind::BadVarint);
}

#[test]
fn fixed_width_little_endian_reads_consume_their_bytes() {
    let buf = [0xFF, 0xFF, 0xFF, 0xFF, 0, 0, 0, 0, 0, 0, 0xF8, 0x3F];
    let mut cur = Cursor::new(&buf);
    assert_eq!(cur.i32_le(), Some(-1));
    assert_eq!(cur.f64_le(), Some(1.5));
    assert_eq!(cur.i32_le(), None);
}

#[test]
fn an_iterator_exposes_where_it_stopped_and_what_it_left() {
    let buf = [0x01, 0x01, 0x05, 0x00, 0xAA];
    let mut sink = NullSink;
    let mut ctx = ctx_null(&mut sink);
    let mut iter = ChunkIter::new(&buf, 0);

    assert!(iter.next(&mut ctx, "T").is_some());
    assert_eq!(iter.cursor().offset(), 3);
    assert!(iter.next(&mut ctx, "T").is_none());
    assert_eq!(iter.tail(), &[0xAA]);
}

#[test]
fn an_overlong_chunk_id_ends_the_stream_and_is_reported() {
    let buf = [0x80, 0x80, 0x80, 0x80, 0x80, 0x01, 0x01, 0x00];
    let mut slots = [Diagnostic::EMPTY; 2];
    let mut sink = SliceSink::new(&mut slots);
    {
        let mut ctx = Ctx::new(Engine::R2K, &mut sink);
        let mut iter = ChunkIter::new(&buf, 0);
        assert!(iter.next(&mut ctx, "T").is_none());
        assert!(iter.next(&mut ctx, "T").is_none(), "a failed stream stays finished");
    }
    assert_eq!(sink.as_slice()[0].kind, DiagKind::BadVarint);
}

#[test]
fn a_list_walker_yields_each_element_with_its_chunks() {
    // count 2 | id 7, chunk 1 "a", terminator | id 9, terminator
    let body = [0x02, 0x07, 0x01, 0x01, 0x61, 0x00, 0x09, 0x00];
    let mut sink = NullSink;
    let mut ctx = ctx_null(&mut sink);
    let mut walker = ListWalker::new(&body, 0, &mut ctx, "T");
    assert_eq!(walker.remaining(), 2);

    let first = walker.next(&mut ctx).unwrap();
    assert_eq!(first.id, 7);
    let chunk = first.chunks().next(&mut ctx, "T").unwrap();
    assert_eq!(chunk.id, 1);
    assert_eq!(chunk.data, b"a");

    let second = walker.next(&mut ctx).unwrap();
    assert_eq!(second.id, 9);
    assert!(second.chunks().next(&mut ctx, "T").is_none());

    assert!(walker.next(&mut ctx).is_none());
    assert_eq!(walker.remaining(), 0);
}

#[test]
fn an_overlong_list_element_id_reads_as_zero_and_is_reported() {
    let body = [0x01, 0x80, 0x80, 0x80, 0x80, 0x80, 0x01, 0x00];
    let mut slots = [Diagnostic::EMPTY; 2];
    let mut sink = SliceSink::new(&mut slots);
    let id = {
        let mut ctx = Ctx::new(Engine::R2K, &mut sink);
        let mut walker = ListWalker::new(&body, 0, &mut ctx, "T");
        walker.next(&mut ctx).unwrap().id
    };
    assert_eq!(id, 0);
    assert_eq!(sink.as_slice()[0].kind, DiagKind::BadVarint);
}

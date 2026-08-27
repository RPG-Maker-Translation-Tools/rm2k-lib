//! Tests for `rm2k::field` - the `LcfField` codec impls for primitives, `DbStr`, `BitArray` and
//! `Packed`.

use rm2k::{
    BitArray, Ctx, DbStr, Engine, LcfField, NullSink, Packed,
    diag::{DiagKind, Diagnostic, SliceSink},
};

fn ctx(sink: &mut NullSink) -> Ctx<'_> {
    Ctx::new(Engine::R2K, sink)
}

#[test]
fn i32_field_is_a_varint() {
    let mut sink = NullSink;
    assert_eq!(<i32 as LcfField>::read(&[0x81, 0x00], &mut ctx(&mut sink), "T", 1), 128);
    assert_eq!(128i32.lcf_size(Engine::R2K), 2);
}

#[test]
fn bool_field_is_a_varint_but_writes_one_byte() {
    let mut sink = NullSink;
    assert!(<bool as LcfField>::read(&[0x01], &mut ctx(&mut sink), "T", 1));
    assert!(!<bool as LcfField>::read(&[0x00], &mut ctx(&mut sink), "T", 1));
    assert_eq!(true.lcf_size(Engine::R2K), 1);
}

#[test]
fn dbstr_borrows_the_input_verbatim() {
    let buf = [0x82u8, 0xA0, 0x82, 0xA2];
    let mut sink = NullSink;
    let s = <DbStr as LcfField>::read(&buf, &mut ctx(&mut sink), "T", 1);
    assert_eq!(s.as_bytes(), &buf);
    assert!(s.as_utf8().is_none(), "Shift-JIS must not be mistaken for UTF-8");
    assert_eq!(s.lcf_size(Engine::R2K), 4);
}

#[test]
fn bit_array_is_one_byte_per_bool() {
    let buf = [0u8, 1, 0, 1];
    let mut sink = NullSink;
    let a = <BitArray as LcfField>::read(&buf, &mut ctx(&mut sink), "T", 1);
    assert_eq!(a.len(), 4);
    assert_eq!(a.iter().collect::<Vec<_>>(), [false, true, false, true]);
}

#[test]
fn packed_array_derives_count_from_chunk_length() {
    let buf = [0x01u8, 0x00, 0x02, 0x00, 0xFF, 0xFF];
    let mut sink = NullSink;
    let a = <Packed<i16> as LcfField>::read(&buf, &mut ctx(&mut sink), "T", 1);
    assert_eq!(&a.0, &[1, 2, -1]);
    assert_eq!(a.lcf_size(Engine::R2K), 6);
}

#[test]
fn packed_array_round_trips() {
    let a = Packed(vec![1i32, -2, 3]);
    let mut out = Vec::new();
    a.write(&mut out, Engine::R2K).unwrap();
    assert_eq!(out.len() as u32, a.lcf_size(Engine::R2K));
    let mut sink = NullSink;
    let b = <Packed<i32> as LcfField>::read(&out, &mut ctx(&mut sink), "T", 1);
    assert_eq!(a, b);
}

#[test]
fn ragged_packed_array_is_truncated_and_reported() {
    let buf = [0x01u8, 0x00, 0x02];
    let mut slots = [Diagnostic::EMPTY; 2];
    let mut sink = SliceSink::new(&mut slots);
    // Scoped so the context releases its borrow of `sink` before the assertions read it.
    let a = {
        let mut c = Ctx::new(Engine::R2K, &mut sink);
        <Packed<i16> as LcfField>::read(&buf, &mut c, "T", 1)
    };
    assert_eq!(&a.0, &[1]);
    assert_eq!(sink.as_slice()[0].kind, DiagKind::RaggedArray);
}

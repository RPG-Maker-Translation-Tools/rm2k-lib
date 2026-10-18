//! Tests for `rm2k::field` - the `LcfField` codec impls for primitives, `DbStr`, `BitArray` and
//! `Packed`.

use rm2k::{
    BitArray, Ctx, DbStr, Engine, LcfField, LcfStruct, NullSink, Packed,
    diag::{DiagKind, DiagSink, Diagnostic, SliceSink, StrictSink},
    field::{List, UnindexedList},
    rpg::{Learning, SaveEasyRpgText},
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

#[test]
fn float_default_elision_compares_bits() {
    // `-0.0 == 0.0`, but the two encode differently; eliding the field would lose the sign.
    assert!(<f64 as LcfField>::lcf_differs(&-0.0, &0.0));
    assert!(!<f64 as LcfField>::lcf_differs(&1.5, &1.5));
    assert!(<i32 as LcfField>::lcf_differs(&1, &0));
    assert!(!<i32 as LcfField>::lcf_differs(&7, &7));
}

#[test]
fn dbstr_converts_from_a_borrowed_slice() {
    let s = DbStr::from(&b"ab"[..]);
    assert_eq!(s.as_bytes(), b"ab");
}

#[test]
fn bit_array_reports_emptiness_and_indexes_by_bit() {
    assert!(BitArray::from_bytes(&[]).is_empty());
    let bits = BitArray::from_bytes(&[0, 3]);
    assert!(!bits.is_empty());
    assert_eq!(bits.get(1), Some(true));
    assert_eq!(bits.get(2), None);
}

#[test]
fn a_scalar_has_no_element_count() {
    assert_eq!(<i32 as LcfField>::element_count(&7), 0);
}

#[test]
fn every_scalar_defaults_to_zero() {
    assert_eq!(<i32 as LcfField>::field_default(Engine::R2K), 0);
    assert!(!<bool as LcfField>::field_default(Engine::R2K));
    assert_eq!(<i16 as LcfField>::field_default(Engine::R2K), 0);
    assert_eq!(<u32 as LcfField>::field_default(Engine::R2K), 0);
    assert_eq!(<f64 as LcfField>::field_default(Engine::R2K), 0.0);
    assert_eq!(<u8 as LcfField>::field_default(Engine::R2K), 0);
    assert_eq!(<i8 as LcfField>::field_default(Engine::R2K), 0);
}

#[test]
fn a_short_fixed_width_field_keeps_its_default_and_reports() {
    let mut slots = [Diagnostic::EMPTY; 2];
    let mut sink = SliceSink::new(&mut slots);
    let value = {
        let mut ctx = Ctx::new(Engine::R2K, &mut sink);
        <u32 as LcfField>::read(&[1, 2], &mut ctx, "T", 1)
    };
    assert_eq!(value, 0);
    assert_eq!(sink.as_slice()[0].kind, DiagKind::ShortChunk);
}

#[test]
fn a_missing_byte_field_keeps_its_default_and_reports() {
    let mut slots = [Diagnostic::EMPTY; 2];
    let mut sink = SliceSink::new(&mut slots);
    let value = {
        let mut ctx = Ctx::new(Engine::R2K, &mut sink);
        <u8 as LcfField>::read(&[], &mut ctx, "T", 1)
    };
    assert_eq!(value, 0);
    assert_eq!(sink.as_slice()[0].kind, DiagKind::ShortChunk);
}

#[test]
fn fixed_width_fields_size_and_write_little_endian() {
    assert_eq!(2i16.lcf_size(Engine::R2K), 2);
    assert_eq!(4u32.lcf_size(Engine::R2K), 4);
    assert_eq!(1.0f64.lcf_size(Engine::R2K), 8);
    let mut out = Vec::new();
    0x0102u32.write(&mut out, Engine::R2K).unwrap();
    assert_eq!(out, [2, 1, 0, 0]);
}

#[test]
fn byte_fields_size_and_write_one_byte() {
    assert_eq!(7u8.lcf_size(Engine::R2K), 1);
    assert_eq!(7i8.lcf_size(Engine::R2K), 1);
    let mut out = Vec::new();
    7u8.write(&mut out, Engine::R2K).unwrap();
    assert_eq!(out, [7]);
}

#[test]
fn a_string_counts_its_bytes() {
    assert_eq!(<DbStr as LcfField>::element_count(&DbStr::from_bytes(b"abc")), 3);
}

#[test]
fn packed_arrays_deref_and_count_their_elements() {
    let packed = Packed(vec![1i16, 2, 3]);
    assert_eq!(packed.len(), 3);
    assert_eq!(packed.element_count(), 3);
}

#[test]
fn a_u32_array_round_trips() {
    let a = Packed(vec![0xDEAD_BEEFu32, 1]);
    let mut out = Vec::new();
    a.write(&mut out, Engine::R2K).unwrap();
    let mut sink = NullSink;
    let b = <Packed<u32> as LcfField>::read(&out, &mut ctx(&mut sink), "T", 1);
    assert_eq!(a, b);
}

#[test]
fn an_indexed_list_counts_its_elements() {
    let mut learning = Learning::defaults(Engine::R2K);
    learning.id = 1;
    assert_eq!(List(vec![learning]).element_count(), 1);
}

#[test]
fn an_unindexed_list_starts_empty_and_derefs_to_its_vec() {
    let mut list = <UnindexedList<SaveEasyRpgText<'static>> as LcfField>::field_default(Engine::R2K);
    assert!(list.is_empty());
    list.push(SaveEasyRpgText::defaults(Engine::R2K));
    assert_eq!(list.len(), 1);
    assert_eq!(list.element_count(), 1);
}

#[test]
fn an_unindexed_list_round_trips_without_element_ids() {
    let mut first = SaveEasyRpgText::defaults(Engine::R2K);
    first.text = DbStr::from_bytes(&[0x82, 0xA0]);
    let list = UnindexedList(vec![first, SaveEasyRpgText::defaults(Engine::R2K)]);

    let mut out = Vec::new();
    list.write(&mut out, Engine::R2K).unwrap();
    assert_eq!(out.len() as u32, list.lcf_size(Engine::R2K));

    let mut sink = NullSink;
    let back = <UnindexedList<SaveEasyRpgText<'_>> as LcfField>::read(&out, &mut ctx(&mut sink), "T", 1);
    assert_eq!(back, list);
}

#[test]
fn an_unindexed_list_that_declares_more_elements_than_it_holds_stops_and_reports() {
    // count 3 | one empty element, which is just its terminator
    let buf = [0x03, 0x00];
    let mut slots = [Diagnostic::EMPTY; 4];
    let mut sink = SliceSink::new(&mut slots);
    let list = {
        let mut ctx = Ctx::new(Engine::R2K, &mut sink);
        <UnindexedList<SaveEasyRpgText<'_>> as LcfField>::read(&buf, &mut ctx, "T", 1)
    };
    assert_eq!(list.len(), 1);
    assert_eq!(sink.as_slice()[0].kind, DiagKind::ShortChunk);
}

#[test]
fn a_strict_load_stops_an_unindexed_list_at_the_first_problem() {
    // count 2 | element 1: unknown chunk 0x7F holding 0x41, then its terminator | element 2: empty
    let buf = [0x02, 0x7F, 0x01, 0x41, 0x00, 0x00];
    let mut sink = StrictSink::new();
    let list = {
        let mut ctx = Ctx::new(Engine::R2K, &mut sink);
        <UnindexedList<SaveEasyRpgText<'_>> as LcfField>::read(&buf, &mut ctx, "T", 1)
    };
    assert_eq!(list.len(), 1);
    assert!(sink.first().is_some());
}

#[test]
fn a_fixed_width_field_with_spare_bytes_reports_them() {
    let mut slots = [Diagnostic::EMPTY; 2];
    let mut sink = SliceSink::new(&mut slots);
    let value = {
        let mut ctx = Ctx::new(Engine::R2K, &mut sink);
        <u32 as LcfField>::read(&[1, 0, 0, 0, 9], &mut ctx, "T", 1)
    };
    assert_eq!(value, 1);
    assert_eq!(sink.as_slice()[0].kind, DiagKind::TrailingBytes);
}

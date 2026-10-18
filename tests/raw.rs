//! Tests for `rm2k::raw` - the hand-written `LcfField` impls that the schema macros build on
//! top of (`Rect`, `Equipment`, `Parameters`, the event/move command lists, `StrList`, ...).

use rm2k::{
    Ctx, DbStr, Engine, LcfField, NullSink, Packed,
    diag::{DiagKind, Diagnostic, SliceSink},
    event::move_code,
    raw::{
        DatabaseVersion, EmptyBlock, Equipment, EventCommand, EventCommandList, MoveCommand, MoveCommandList,
        Parameters, Rect, StrList,
    },
};

fn ctx(sink: &mut NullSink) -> Ctx<'_> {
    Ctx::new(Engine::R2K, sink)
}

fn round_trip<'a, T>(value: &T, buf: &'a mut Vec<u8>) -> T
where
    T: LcfField<'a> + core::fmt::Debug,
{
    value.write(buf, Engine::R2K).unwrap();
    assert_eq!(buf.len() as u32, value.lcf_size(Engine::R2K), "size must predict write");
    let mut sink = NullSink;
    let mut c = Ctx::new(Engine::R2K, &mut sink);
    T::read(buf, &mut c, "T", 1)
}

#[test]
fn rect_is_four_le_u32s() {
    let rect = Rect {
        left: 1,
        top: 2,
        right: 3,
        bottom: 4,
    };
    let mut buf = Vec::new();
    assert_eq!(round_trip(&rect, &mut buf), rect);
    assert_eq!(buf.len(), 16);
    assert_eq!(&buf[..4], &[1, 0, 0, 0]);
}

#[test]
fn equipment_rejects_a_payload_that_is_not_ten_bytes() {
    let mut sink = NullSink;
    let short = <Equipment as LcfField>::read(&[1, 0, 2, 0], &mut ctx(&mut sink), "T", 1);
    assert_eq!(short, Equipment::default(), "liblcf skips the whole chunk");
}

#[test]
fn equipment_round_trips() {
    let eq = Equipment {
        weapon_id: 1,
        shield_id: 2,
        armor_id: 3,
        helmet_id: 4,
        accessory_id: 5,
    };
    let mut buf = Vec::new();
    assert_eq!(round_trip(&eq, &mut buf), eq);
}

#[test]
fn parameters_split_the_payload_six_ways() {
    let params = Parameters {
        maxhp: Packed(vec![1, 2]),
        maxsp: Packed(vec![3, 4]),
        attack: Packed(vec![5, 6]),
        defense: Packed(vec![7, 8]),
        spirit: Packed(vec![9, 10]),
        agility: Packed(vec![11, 12]),
    };
    let mut buf = Vec::new();
    assert_eq!(round_trip(&params, &mut buf), params);
    assert_eq!(buf.len(), 6 * 2 * 2);
}

#[test]
fn empty_block_writes_a_header_and_no_payload() {
    let mut buf = Vec::new();
    EmptyBlock.write(&mut buf, Engine::R2K).unwrap();
    assert!(buf.is_empty());
    assert_eq!(EmptyBlock.lcf_size(Engine::R2K), 0);
}

#[test]
fn database_version_zero_has_an_empty_payload() {
    let mut buf = Vec::new();
    DatabaseVersion(0).write(&mut buf, Engine::R2K).unwrap();
    assert!(buf.is_empty());
    assert_eq!(DatabaseVersion(0).lcf_size(Engine::R2K), 0);

    let mut sink = NullSink;
    assert_eq!(
        <DatabaseVersion as LcfField>::read(&[], &mut ctx(&mut sink), "T", 1),
        DatabaseVersion(0)
    );
}

#[test]
fn database_version_nonzero_round_trips() {
    let mut buf = Vec::new();
    assert_eq!(round_trip(&DatabaseVersion(1), &mut buf), DatabaseVersion(1));
    assert_eq!(buf, vec![1]);
}

#[test]
fn event_command_list_round_trips() {
    let list = EventCommandList(vec![
        EventCommand {
            code: 10110,
            indent: 0,
            string: DbStr::from_bytes(b"hello"),
            parameters: vec![1, 2, 3],
        },
        EventCommand {
            code: 20110,
            indent: 1,
            string: DbStr::from_bytes(&[0x82, 0xA0]),
            parameters: Vec::new(),
        },
    ]);
    let mut buf = Vec::new();
    let back = round_trip(&list, &mut buf);
    assert_eq!(back, list);
    assert_eq!(&buf[buf.len() - 4..], &[0, 0, 0, 0], "four-byte terminator");
}

#[test]
fn empty_event_command_list_is_just_the_terminator() {
    let list = EventCommandList::default();
    let mut buf = Vec::new();
    assert_eq!(round_trip(&list, &mut buf), list);
    assert_eq!(buf, vec![0, 0, 0, 0]);
}

#[test]
fn move_command_list_round_trips_per_code_payloads() {
    let list = MoveCommandList(vec![
        MoveCommand {
            code: 0,
            ..MoveCommand::default()
        },
        MoveCommand {
            code: move_code::SWITCH_ON,
            parameter_a: 7,
            ..MoveCommand::default()
        },
        MoveCommand {
            code: move_code::PLAY_SOUND_EFFECT,
            string: DbStr::from_bytes(b"bell"),
            parameter_a: 1,
            parameter_b: 2,
            parameter_c: 3,
        },
    ]);
    let mut buf = Vec::new();
    assert_eq!(round_trip(&list, &mut buf), list);
}

#[test]
fn str_list_run_length_encodes_gaps() {
    let list = StrList(vec![
        DbStr::from_bytes(b"a"),
        DbStr::default(),
        DbStr::default(),
        DbStr::from_bytes(b"b"),
    ]);
    let mut buf = Vec::new();
    let back = round_trip(&list, &mut buf);
    assert_eq!(back, list);
    // "a" (2 bytes) + gap marker (5) + "b" (2). Note the gap marker is *bigger* than the two
    // zero-length entries it replaces would have been - see the note on `StrList`.
    assert_eq!(buf.len(), 9);
}

#[test]
fn str_list_drops_a_trailing_gap_as_liblcf_does() {
    let list = StrList(vec![DbStr::from_bytes(b"a"), DbStr::default()]);
    let mut buf = Vec::new();
    let back = round_trip(&list, &mut buf);
    assert_eq!(back.0.len(), 1, "the trailing empty is not written back");
}

#[test]
fn str_list_rejects_an_absurd_gap() {
    // A gap marker claiming a huge run must not turn into a huge allocation. `StrList`'s gap
    // encoding is `GAP_BASE - run_length` where `GAP_BASE` is 0x8_0000_0000 (an internal, private
    // constant) - this writes a marker just under that base directly, rather than reaching for
    // the constant itself.
    let mut buf = Vec::new();
    // One entry past the load's budget (`ENTRY_BUDGET`, 2^20), so the run is refused rather than expanded.
    rm2k::wire::write_varint64(&mut buf, 0x8_0000_0000 - (1 << 20) - 1).unwrap();
    let mut slots = [Diagnostic::EMPTY; 2];
    let mut sink = SliceSink::new(&mut slots);
    let list = {
        let mut ctx = Ctx::new(Engine::R2K, &mut sink);
        <StrList as LcfField>::read(&buf, &mut ctx, "T", 1)
    };
    assert!(list.0.is_empty());
    assert_eq!(sink.as_slice()[0].kind, DiagKind::EntryBudgetExhausted);
}

#[test]
fn a_rect_missing_its_last_field_keeps_what_it_read() {
    let mut slots = [Diagnostic::EMPTY; 2];
    let mut sink = SliceSink::new(&mut slots);
    let rect = {
        let mut ctx = Ctx::new(Engine::R2K, &mut sink);
        <Rect as LcfField>::read(&[1, 0, 0, 0, 2, 0, 0, 0, 3], &mut ctx, "T", 1)
    };
    assert_eq!((rect.left, rect.top, rect.right, rect.bottom), (1, 2, 0, 0));
    assert_eq!(sink.as_slice()[0].kind, DiagKind::ShortChunk);
}

#[test]
fn a_string_list_with_a_truncated_marker_is_reported() {
    let mut slots = [Diagnostic::EMPTY; 2];
    let mut sink = SliceSink::new(&mut slots);
    let list = {
        let mut ctx = Ctx::new(Engine::R2K, &mut sink);
        <StrList as LcfField>::read(&[0x80], &mut ctx, "T", 1)
    };
    assert!(list.is_empty());
    assert_eq!(sink.as_slice()[0].kind, DiagKind::ShortChunk);
}

#[test]
fn a_string_list_entry_longer_than_its_chunk_is_reported() {
    // claims five bytes, carries two
    let buf = [0x05, 0x41, 0x42];
    let mut slots = [Diagnostic::EMPTY; 2];
    let mut sink = SliceSink::new(&mut slots);
    let list = {
        let mut ctx = Ctx::new(Engine::R2K, &mut sink);
        <StrList as LcfField>::read(&buf, &mut ctx, "T", 1)
    };
    assert!(list.is_empty());
    assert_eq!(sink.as_slice()[0].kind, DiagKind::ShortChunk);
}

#[test]
fn a_string_list_gap_longer_than_its_chunk_is_read_in_full() {
    // A marker for 100 empty entries, then "b", in a chunk far shorter than the run it expands to. The writer
    // compresses runs, so this is what a real file with a long run of empties looks like.
    let mut buf = Vec::new();
    rm2k::wire::write_varint64(&mut buf, (1u64 << 35) - 100).unwrap();
    rm2k::wire::write_varint(&mut buf, 1).unwrap();
    buf.push(b'b');
    let mut slots = [Diagnostic::EMPTY; 2];
    let mut sink = SliceSink::new(&mut slots);
    let list = {
        let mut ctx = Ctx::new(Engine::R2K, &mut sink);
        <StrList as LcfField>::read(&buf, &mut ctx, "T", 1)
    };
    assert_eq!(list.len(), 101);
    assert!(list[..100].iter().all(DbStr::is_empty));
    assert_eq!(list[100], DbStr::from_bytes(b"b"));
    assert!(sink.as_slice().is_empty(), "nothing was lost, so nothing is reported");
}

#[test]
fn a_gap_marker_at_the_base_is_reported_not_skipped() {
    // `GAP_BASE - 0` names an empty run. The writer never emits one, so the marker is corrupt.
    let mut buf = Vec::new();
    rm2k::wire::write_varint64(&mut buf, 0x8_0000_0000).unwrap();
    buf.push(0x01);
    buf.push(b'b');
    let mut slots = [Diagnostic::EMPTY; 2];
    let mut sink = SliceSink::new(&mut slots);
    let list = {
        let mut ctx = Ctx::new(Engine::R2K, &mut sink);
        <StrList as LcfField>::read(&buf, &mut ctx, "T", 1)
    };
    assert!(list.is_empty());
    assert_eq!(sink.as_slice()[0].kind, DiagKind::BadVarint);
}

#[test]
fn a_long_run_between_strings_survives_a_save_and_load() {
    // The round trip that used to drop "b": the run is longer than the chunk it is written into.
    let mut list = StrList(vec![DbStr::from_bytes(b"a")]);
    list.0.extend(core::iter::repeat_n(DbStr::default(), 100));
    list.0.push(DbStr::from_bytes(b"b"));
    let mut buf = Vec::new();
    let back = round_trip(&list, &mut buf);
    assert_eq!(back, list);
}

#[test]
fn list_wrappers_deref_to_their_vectors_and_count_elements() {
    let strings = StrList(vec![DbStr::from_bytes(b"a")]);
    assert_eq!(strings.len(), 1);
    assert_eq!(strings.element_count(), 1);

    let moves = MoveCommandList(Vec::new());
    assert!(moves.is_empty());
    assert_eq!(moves.element_count(), 0);

    let events = EventCommandList(Vec::new());
    assert_eq!(events.element_count(), 0);
}

#[test]
fn a_string_list_gap_past_the_address_space_is_rejected() {
    // A gap of 2^33 empty entries. On a 32-bit target it cannot even be a `usize`; on a 64-bit host
    // it is caught by the budget instead. Either way it must be rejected, not allocated.
    let mut buf = Vec::new();
    rm2k::wire::write_varint64(&mut buf, (1u64 << 35) - (1u64 << 33)).unwrap();
    let mut slots = [Diagnostic::EMPTY; 2];
    let mut sink = SliceSink::new(&mut slots);
    let list = {
        let mut ctx = Ctx::new(Engine::R2K, &mut sink);
        <StrList as LcfField>::read(&buf, &mut ctx, "T", 1)
    };
    assert!(list.is_empty());
    assert_eq!(sink.as_slice()[0].kind, DiagKind::EntryBudgetExhausted);
}

// Builds 20,000 runs of 40,000 empty entries; too slow under Miri, and the budget is plain safe code.
#[cfg_attr(miri, ignore)]
#[test]
fn a_string_list_stops_at_the_entry_budget_instead_of_growing_without_bound() {
    // Each marker is a run of 40,000 empty entries and fits the input easily. Unbounded, 20,000 of them would
    // be 800 million entries. The budget admits 26 runs (1,040,000 entries); a 27th would pass 1,048,576.
    let mut buf = Vec::new();
    for _ in 0..20_000 {
        rm2k::wire::write_varint64(&mut buf, 0x8_0000_0000 - 40_000).unwrap();
    }
    let mut slots = [Diagnostic::EMPTY; 2];
    let mut sink = SliceSink::new(&mut slots);
    let list = {
        let mut ctx = Ctx::new(Engine::R2K, &mut sink);
        <StrList as LcfField>::read(&buf, &mut ctx, "T", 1)
    };
    assert_eq!(list.len(), 26 * 40_000);
    assert_eq!(sink.as_slice()[0].kind, DiagKind::EntryBudgetExhausted);
}

// Builds 2^20 + 1 one-byte entries; too slow under Miri, and the budget is plain safe code.
#[cfg_attr(miri, ignore)]
#[test]
fn a_string_list_of_single_entries_stops_at_the_entry_budget() {
    // One entry more than the budget allows. The list keeps the budget's worth and reports the rest.
    let buf: Vec<u8> = (0..=(1u32 << 20)).flat_map(|_| [0x01, 0x61]).collect();
    let mut slots = [Diagnostic::EMPTY; 2];
    let mut sink = SliceSink::new(&mut slots);
    let list = {
        let mut ctx = Ctx::new(Engine::R2K, &mut sink);
        <StrList as LcfField>::read(&buf, &mut ctx, "T", 1)
    };
    assert_eq!(list.len(), 1 << 20);
    assert_eq!(sink.as_slice()[0].kind, DiagKind::EntryBudgetExhausted);
}

// Two runs of 600,000 entries; too slow under Miri, and the budget is plain safe code.
#[cfg_attr(miri, ignore)]
#[test]
fn the_entry_budget_is_shared_by_repeated_and_nested_string_lists() {
    // The first run fits. The second is read through a reborrow, as a nested struct's would be, and must still
    // draw on the same budget: 1,200,000 entries would pass 1,048,576.
    let run = || {
        let mut buf = Vec::new();
        rm2k::wire::write_varint64(&mut buf, 0x8_0000_0000 - 600_000).unwrap();
        buf
    };
    let first_run = run();
    let second_run = run();
    let mut slots = [Diagnostic::EMPTY; 2];
    let mut sink = SliceSink::new(&mut slots);
    let (first, second) = {
        let mut ctx = Ctx::new(Engine::R2K, &mut sink);
        let first = <StrList as LcfField>::read(&first_run, &mut ctx, "T", 1);
        let mut nested = ctx.reborrow();
        let second = <StrList as LcfField>::read(&second_run, &mut nested, "T", 1);
        (first.len(), second.len())
    };
    assert_eq!(first, 600_000);
    assert_eq!(second, 0);
    assert_eq!(sink.as_slice()[0].kind, DiagKind::EntryBudgetExhausted);
}

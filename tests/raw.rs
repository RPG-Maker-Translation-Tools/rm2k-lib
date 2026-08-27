//! Tests for `rm2k::raw` - the hand-written `LcfField` impls that the schema macros build on
//! top of (`Rect`, `Equipment`, `Parameters`, the event/move command lists, `StrList`, ...).

use rm2k::{
    Ctx, DbStr, Engine, LcfField, NullSink, Packed,
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
    rm2k::wire::write_varint64(&mut buf, 0x8_0000_0000 - 1_000_000).unwrap();
    let mut sink = NullSink;
    let list = <StrList as LcfField>::read(&buf, &mut ctx(&mut sink), "T", 1);
    assert!(list.0.is_empty());
}

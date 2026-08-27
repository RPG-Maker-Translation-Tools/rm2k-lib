//! Tests for `rm2k::event` - zero-copy event and move-command iteration.

use rm2k::{
    Ctx, Engine, EventCommandIter, MoveCommandIter, NullSink,
    diag::{DiagKind, Diagnostic, SliceSink},
    event::{event_command_len, move_code},
};

/// `code=10110 indent=0 string="hi" params=[1,2]`, then the four-zero terminator.
const ONE_COMMAND: &[u8] = &[
    0xCE, 0x7E, // varint 10110
    0x00, // indent 0
    0x02, b'h', b'i', // string
    0x02, 0x01, 0x02, // two parameters
    0x00, 0x00, 0x00, 0x00, // terminator
];

fn ctx(sink: &mut NullSink) -> Ctx<'_> {
    Ctx::new(Engine::R2K, sink)
}

#[test]
fn decodes_a_command_without_allocating() {
    let mut sink = NullSink;
    let mut c = ctx(&mut sink);
    let mut it = EventCommandIter::new(ONE_COMMAND, 0);

    let cmd = it.next(&mut c).unwrap();
    assert_eq!(cmd.code, 10110);
    assert_eq!(cmd.indent, 0);
    assert_eq!(cmd.string, b"hi");
    assert_eq!(cmd.param_count(), 2);
    let mut params = [0i32; 2];
    for (slot, value) in params.iter_mut().zip(cmd.params()) {
        *slot = value;
    }
    assert_eq!(params, [1, 2]);
    assert_eq!(cmd.param(1), Some(2));
    assert_eq!(cmd.param(9), None);

    assert!(it.next(&mut c).is_none(), "terminator ends the list");
}

#[test]
fn strings_come_back_verbatim() {
    // A Shift-JIS command string must not be touched.
    let buf = [0x01, 0x00, 0x04, 0x82, 0xA0, 0x82, 0xA2, 0x00, 0, 0, 0, 0];
    let mut sink = NullSink;
    let mut c = ctx(&mut sink);
    let mut it = EventCommandIter::new(&buf, 0);
    assert_eq!(it.next(&mut c).unwrap().string, &[0x82, 0xA0, 0x82, 0xA2]);
}

#[test]
fn empty_list_is_just_a_terminator() {
    let mut sink = NullSink;
    let mut c = ctx(&mut sink);
    let mut it = EventCommandIter::new(&[0, 0, 0, 0], 0);
    assert!(it.next(&mut c).is_none());
}

#[test]
fn truncated_command_recovers_and_reports() {
    // Declares a 99-byte string but the payload ends.
    let buf = [0x01, 0x00, 0x63, 0xAA];
    let mut slots = [Diagnostic::EMPTY; 2];
    let mut sink = SliceSink::new(&mut slots);
    // Scoped so the context releases its borrow of `sink` before the assertion reads it.
    {
        let mut c = Ctx::new(Engine::R2K, &mut sink);
        let mut it = EventCommandIter::new(&buf, 0);
        assert!(it.next(&mut c).is_none());
    }
    assert_eq!(sink.as_slice()[0].kind, DiagKind::EventListCorrupt);
}

#[test]
fn move_commands_with_no_payload() {
    // Three bare direction commands, running to the end of the chunk.
    let buf = [0x00, 0x01, 0x02];
    let mut sink = NullSink;
    let mut c = ctx(&mut sink);
    let mut it = MoveCommandIter::new(&buf, 0);
    for expected in 0..3 {
        assert_eq!(it.next(&mut c).unwrap().code, expected);
    }
    assert!(it.next(&mut c).is_none());
}

#[test]
fn move_command_payloads_follow_the_code() {
    // play_sound_effect: string "s" then three integers.
    let buf = [35, 0x01, b's', 0x0A, 0x14, 0x1E];
    let mut sink = NullSink;
    let mut c = ctx(&mut sink);
    let mut it = MoveCommandIter::new(&buf, 0);
    let cmd = it.next(&mut c).unwrap();
    assert_eq!(cmd.code, move_code::PLAY_SOUND_EFFECT);
    assert_eq!(cmd.string, b"s");
    assert_eq!((cmd.parameter_a, cmd.parameter_b, cmd.parameter_c), (10, 20, 30));
    assert!(it.next(&mut c).is_none());
}

#[test]
fn command_length_matches_what_was_decoded() {
    let mut sink = NullSink;
    let mut c = ctx(&mut sink);
    let mut it = EventCommandIter::new(ONE_COMMAND, 0);
    let cmd = it.next(&mut c).unwrap();
    let len = event_command_len(cmd.code, cmd.indent, cmd.string.len(), &[1, 2]);
    assert_eq!(len as usize, ONE_COMMAND.len() - 4);
}

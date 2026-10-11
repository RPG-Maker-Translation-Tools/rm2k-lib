//! Regression tests for what fuzzing (`fuzz/`) found: hostile input that used to allocate gigabytes
//! or loop for billions of iterations, and must be an ordinary bounded result instead.

use rm2k::{field::LcfField, wire, Ctx, Engine, EventCommandIter, EventCommandList, NullSink};

fn varint(value: u32) -> Vec<u8> {
    let mut out = Vec::new();
    wire::write_varint(&mut out, value).unwrap();
    out
}

/// One event command - code 1, no indent, no string - declaring `declared` parameters but
/// carrying only `present`, then the list terminator.
fn command_list(declared: u32, present: &[u32]) -> Vec<u8> {
    let mut list = vec![1, 0, 0];
    list.extend(varint(declared));
    for &value in present {
        list.extend(varint(value));
    }
    list.extend([0, 0, 0, 0]);
    list
}

/// A command can declare `u32::MAX` parameters in five bytes. `ParamIter` is exact-size, so
/// collecting it used to allocate (and zero-fill) for all four billion.
#[test]
fn a_command_cannot_claim_more_parameters_than_it_holds() {
    let list = command_list(u32::MAX, &[5, 7]);

    let mut sink = NullSink;
    let mut ctx = Ctx::new(Engine::R2K, &mut sink);
    let mut iter = EventCommandIter::new(&list, 0);
    let command = iter.next(&mut ctx).expect("the command itself is fine");

    // It holds what is there, which includes the terminator's four zero bytes: the list has no
    // framing to tell them apart from parameters.
    assert_eq!(command.param_count(), 6);
    assert_eq!(command.params().collect::<Vec<_>>(), [5, 7, 0, 0, 0, 0]);
}

#[test]
fn the_owned_command_list_does_not_allocate_for_claimed_parameters() {
    let list = command_list(u32::MAX, &[5, 7]);

    let mut sink = NullSink;
    let mut ctx = Ctx::new(Engine::R2K, &mut sink);
    let commands = EventCommandList::read(&list, &mut ctx, "test", 0);

    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].parameters, [5, 7, 0, 0, 0, 0]);
}

/// A command that declares exactly as many parameters as it has is unaffected.
#[test]
fn honest_commands_are_unchanged() {
    let list = command_list(3, &[1, 2, 3]);

    let mut sink = NullSink;
    let mut ctx = Ctx::new(Engine::R2K, &mut sink);
    let commands = EventCommandList::read(&list, &mut ctx, "test", 0);

    assert_eq!(commands[0].parameters, [1, 2, 3]);
}

#![no_main]
use libfuzzer_sys::fuzz_target;
use rm2k::{Ctx, Engine, EventCommandIter, MoveCommandIter, NullSink};

/// The allocation-free command iterators, over arbitrary bytes as a chunk payload.
fuzz_target!(|data: &[u8]| {
    let mut sink = NullSink;
    let mut ctx = Ctx::new(Engine::R2K, &mut sink);

    let mut events = EventCommandIter::new(data, 0);
    while let Some(command) = events.next(&mut ctx) {
        let _ = (command.code, command.indent, command.string.len(), command.param_count());
        for param in command.params() {
            let _ = param;
        }
        let _ = command.param(3);
    }
    let _ = events.tail();

    let mut moves = MoveCommandIter::new(data, 0);
    while let Some(command) = moves.next(&mut ctx) {
        let _ = command;
    }
});

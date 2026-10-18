#![no_main]
//! Drives the C ABI the way a C caller does. Each loader's output must save, and its save must
//! load and save to the same bytes. Then every walker is run over the input's body, in every
//! event scope, and every accessor is read, so the sanitizer checks the pointers and lengths they
//! lend out.
use libfuzzer_sys::fuzz_target;
use rm2k_capi::{
    Rm2kBytes, Rm2kChunk, Rm2kChunkIter, Rm2kCommandList, Rm2kCommandLists, Rm2kDbList, Rm2kDiagnostic,
    Rm2kDiagnostics, Rm2kEngine, Rm2kError, Rm2kEventCommand, Rm2kEventCommands, Rm2kEventScope, Rm2kListItem,
    Rm2kListWalker, Rm2kNamed, rm2k_buffer_free, rm2k_chunk_find, rm2k_chunk_iter_init, rm2k_chunk_iter_next,
    rm2k_command_lists_init, rm2k_command_lists_next, rm2k_database_free, rm2k_database_list_get,
    rm2k_database_list_len, rm2k_database_load, rm2k_database_save, rm2k_diagnostic_kind_name, rm2k_diagnostics_count,
    rm2k_diagnostics_free, rm2k_diagnostics_get, rm2k_diagnostics_new, rm2k_event_command_param,
    rm2k_event_commands_init, rm2k_event_commands_next, rm2k_list_walker_init, rm2k_list_walker_next, rm2k_map_free,
    rm2k_map_load, rm2k_map_save, rm2k_save_free, rm2k_save_load, rm2k_save_save, rm2k_split_header,
    rm2k_tree_map_free, rm2k_tree_map_load, rm2k_tree_map_save,
};
use std::{hint::black_box, ptr, slice};

/// Copies out the buffer a `*_save` call returned, then frees it.
fn saved(save: impl FnOnce(*mut usize) -> *mut u8) -> Vec<u8> {
    let mut len = 0usize;
    let ptr = save(&mut len as *mut usize);
    assert!(!ptr.is_null(), "a loaded value must save");
    // SAFETY: `ptr` is a buffer of `len` initialized bytes from a save call, freed once below.
    let bytes = unsafe { slice::from_raw_parts(ptr, len) }.to_vec();
    unsafe { rm2k_buffer_free(ptr, len) };
    bytes
}

/// Reads the bytes an accessor lent out, so the sanitizer checks that the pointer and length are valid.
fn touch(bytes: Rm2kBytes) {
    if bytes.len == 0 {
        return;
    }
    // SAFETY: the accessor that produced `bytes` lent `len` readable bytes from a live buffer.
    let lent = unsafe { slice::from_raw_parts(bytes.data, bytes.len) };
    black_box(lent.iter().fold(0u8, |sum, &byte| sum ^ byte));
}

/// Loads, saves, reloads and saves again, for both engines. The two saves must match.
macro_rules! round_trip {
    ($load:ident, $save:ident, $free:ident, $data:expr, $diag:expr) => {{
        let mut err = Rm2kError::Ok;
        let handle = unsafe { $load($data.as_ptr(), $data.len(), $diag, &mut err) };
        if !handle.is_null() {
            for engine in [Rm2kEngine::R2K, Rm2kEngine::R2K3] {
                let first = saved(|len| unsafe { $save(handle, engine, true, len) });
                let reloaded = unsafe { $load(first.as_ptr(), first.len(), ptr::null_mut(), &mut err) };
                assert!(!reloaded.is_null(), "own output must load");
                let second = saved(|len| unsafe { $save(reloaded, engine, true, len) });
                assert!(first == second, "{engine:?}: save is not stable");
                unsafe { $free(reloaded) };
            }
            unsafe { $free(handle) };
        }
    }};
}

/// Walks the chunks, the indexed list and the event commands under `body`, reading every
/// parameter of every command, including one past the end.
fn walk_body(body: &[u8], diag: *mut Rm2kDiagnostics) {
    let mut chunks = std::mem::MaybeUninit::<Rm2kChunkIter>::uninit();
    let mut chunk = Rm2kChunk {
        id: 0,
        offset: 0,
        data: Rm2kBytes {
            data: ptr::null(),
            len: 0,
        },
    };
    if unsafe { rm2k_chunk_iter_init(chunks.as_mut_ptr(), body.as_ptr(), body.len(), 0) } {
        while unsafe { rm2k_chunk_iter_next(chunks.as_mut_ptr(), diag, &mut chunk) } {
            touch(chunk.data);
        }
    }

    let _ = unsafe { rm2k_chunk_find(body.as_ptr(), body.len(), 0, 0x51, diag, &mut chunk) };

    let mut walker = std::mem::MaybeUninit::<Rm2kListWalker>::uninit();
    if unsafe { rm2k_list_walker_init(walker.as_mut_ptr(), body.as_ptr(), body.len(), 0, diag) } {
        let mut item = Rm2kListItem {
            id: 0,
            offset: 0,
            body: Rm2kBytes {
                data: ptr::null(),
                len: 0,
            },
        };
        while unsafe { rm2k_list_walker_next(walker.as_mut_ptr(), diag, &mut item) } {
            touch(item.body);
        }
    }

    for scope in [
        Rm2kEventScope::MapEvents,
        Rm2kEventScope::CommonEvents,
        Rm2kEventScope::BattleEvents,
    ] {
        let mut lists = std::mem::MaybeUninit::<Rm2kCommandLists>::uninit();
        if !unsafe { rm2k_command_lists_init(lists.as_mut_ptr(), scope, body.as_ptr(), body.len(), 0, diag) } {
            continue;
        }
        let mut list = Rm2kCommandList {
            owner_id: 0,
            page_id: 0,
            offset: 0,
            commands: Rm2kBytes {
                data: ptr::null(),
                len: 0,
            },
        };
        while unsafe { rm2k_command_lists_next(lists.as_mut_ptr(), diag, &mut list) } {
            let mut commands = std::mem::MaybeUninit::<Rm2kEventCommands>::uninit();
            if !unsafe {
                rm2k_event_commands_init(
                    commands.as_mut_ptr(),
                    list.commands.data,
                    list.commands.len,
                    list.offset,
                )
            } {
                continue;
            }
            let mut command = Rm2kEventCommand {
                code: 0,
                indent: 0,
                param_count: 0,
                string: Rm2kBytes {
                    data: ptr::null(),
                    len: 0,
                },
                raw_params: Rm2kBytes {
                    data: ptr::null(),
                    len: 0,
                },
            };
            while unsafe { rm2k_event_commands_next(commands.as_mut_ptr(), diag, &mut command) } {
                touch(command.string);
                for index in 0..=command.param_count as usize {
                    let mut value = 0i32;
                    black_box(unsafe { rm2k_event_command_param(&command, index, &mut value) });
                }
            }
        }
    }
}

fuzz_target!(|data: &[u8]| {
    let diag = rm2k_diagnostics_new();

    round_trip!(rm2k_database_load, rm2k_database_save, rm2k_database_free, data, diag);
    round_trip!(rm2k_map_load, rm2k_map_save, rm2k_map_free, data, diag);
    round_trip!(rm2k_tree_map_load, rm2k_tree_map_save, rm2k_tree_map_free, data, diag);
    round_trip!(rm2k_save_load, rm2k_save_save, rm2k_save_free, data, diag);

    // The database's named lists, from a loaded database.
    let mut err = Rm2kError::Ok;
    let database = unsafe { rm2k_database_load(data.as_ptr(), data.len(), diag, &mut err) };
    if !database.is_null() {
        for list in [
            Rm2kDbList::Actors,
            Rm2kDbList::Skills,
            Rm2kDbList::Items,
            Rm2kDbList::Enemies,
            Rm2kDbList::Troops,
            Rm2kDbList::States,
            Rm2kDbList::Classes,
            Rm2kDbList::Animations,
            Rm2kDbList::Chipsets,
            Rm2kDbList::Terrains,
            Rm2kDbList::Attributes,
            Rm2kDbList::CommonEvents,
        ] {
            let count = unsafe { rm2k_database_list_len(database, list) };
            for index in 0..count {
                let mut named = Rm2kNamed {
                    id: 0,
                    name: Rm2kBytes {
                        data: ptr::null(),
                        len: 0,
                    },
                };
                assert!(unsafe { rm2k_database_list_get(database, list, index, &mut named) });
                touch(named.name);
            }
        }
        unsafe { rm2k_database_free(database) };
    }

    // The walkers see the same bytes as a file body, after its header.
    let mut header = Rm2kBytes {
        data: ptr::null(),
        len: 0,
    };
    let mut body = Rm2kBytes {
        data: ptr::null(),
        len: 0,
    };
    if unsafe { rm2k_split_header(data.as_ptr(), data.len(), &mut header, &mut body) } {
        walk_body(unsafe { slice::from_raw_parts(body.data, body.len) }, diag);
    }
    // The walkers also see the whole input, as a stream of chunks that does not start with a header.
    walk_body(data, diag);

    let count = unsafe { rm2k_diagnostics_count(diag) };
    for index in 0..count {
        let mut diagnostic = Rm2kDiagnostic {
            offset: 0,
            chunk_id: 0,
            kind: 0,
            struct_name: Rm2kBytes {
                data: ptr::null(),
                len: 0,
            },
        };
        assert!(unsafe { rm2k_diagnostics_get(diag, index, &mut diagnostic) });
        assert!(!rm2k_diagnostic_kind_name(diagnostic.kind).is_null());
        touch(diagnostic.struct_name);
    }
    unsafe { rm2k_diagnostics_free(diag) };
});

//! The C ABI, called from Rust the way a C caller calls it. Miri runs these, so they also check the
//! raw-pointer handling of each entry point they reach.

#[path = "../../tests/common/samples.rs"]
mod samples;

use super::*;
use core::{ffi::CStr, mem::MaybeUninit, ptr, slice};
use rm2k::{
    DbStr, Engine, LcfStruct, SaveOpt,
    field::List,
    file,
    raw::EventCommandList,
    rpg::{Actor, CommonEvent, Database, Map, Save, TreeMap, Troop, TroopPage},
};

fn database_bytes() -> Vec<u8> {
    file::to_vec(|out| {
        file::save_database(
            &Database::defaults(Engine::R2K),
            out,
            Engine::R2K,
            SaveOpt::default(),
            b"",
        )
    })
}

/// The bytes a C-API call lent out. An empty span may carry a null pointer, which `from_raw_parts`
/// forbids, so it is checked first. The caller keeps the lending buffer alive for `'a`.
fn lent<'a>(bytes: Rm2kBytes) -> &'a [u8] {
    if bytes.len == 0 {
        return &[];
    }
    // SAFETY: the call that produced `bytes` lent `len` readable bytes from a buffer the caller still holds.
    unsafe { slice::from_raw_parts(bytes.data, bytes.len) }
}

/// Copies out and frees a buffer a `*_save` call returned.
fn take(ptr: *mut u8, len: usize) -> Vec<u8> {
    assert!(!ptr.is_null(), "save returned no buffer");
    // SAFETY: `ptr` came from a save call that reported `len`, and is freed below, exactly once.
    let bytes = unsafe { slice::from_raw_parts(ptr, len) }.to_vec();
    unsafe { rm2k_buffer_free(ptr, len) };
    bytes
}

fn no_diagnostic() -> Rm2kDiagnostic {
    Rm2kDiagnostic {
        offset: 0,
        chunk_id: 0,
        kind: 0,
        struct_name: Rm2kBytes::EMPTY,
    }
}

#[test]
fn null_and_empty_inputs_are_errors() {
    let mut err = Rm2kError::Ok;
    let handle = unsafe { rm2k_database_load(ptr::null(), 0, ptr::null_mut(), &mut err) };
    assert!(handle.is_null());
    assert_eq!(err, Rm2kError::NullArgument);

    let empty: [u8; 0] = [];
    let handle = unsafe { rm2k_map_load(empty.as_ptr(), 0, ptr::null_mut(), &mut err) };
    assert!(handle.is_null());
    assert_eq!(err, Rm2kError::UnexpectedEof);
}

#[test]
fn null_handles_are_harmless() {
    unsafe {
        rm2k_database_free(ptr::null_mut());
        rm2k_map_free(ptr::null_mut());
        rm2k_tree_map_free(ptr::null_mut());
        rm2k_save_free(ptr::null_mut());
        rm2k_buffer_free(ptr::null_mut(), 0);
        rm2k_diagnostics_free(ptr::null_mut());
    }
    assert_eq!(lent(unsafe { rm2k_database_header(ptr::null()) }), []);
    assert_eq!(unsafe { rm2k_database_engine(ptr::null()) }, Rm2kEngine::R2K);
    assert_eq!(unsafe { rm2k_database_list_len(ptr::null(), Rm2kDbList::Actors) }, 0);
    assert_eq!(unsafe { rm2k_diagnostics_count(ptr::null()) }, 0);
}

/// A file built with the typed API survives load then save through the C API, byte for byte.
macro_rules! round_trip {
    ($name:ident, $value:expr, $save:path, $load:ident, $save_c:ident, $free:ident) => {
        #[test]
        fn $name() {
            let value = $value;
            let bytes = file::to_vec(|out| $save(&value, out, Engine::R2K, SaveOpt::default(), b""));

            let mut err = Rm2kError::Ok;
            let handle = unsafe { $load(bytes.as_ptr(), bytes.len(), ptr::null_mut(), &mut err) };
            assert_eq!(err, Rm2kError::Ok);
            assert!(!handle.is_null());

            let mut len = 0usize;
            let saved = unsafe { $save_c(handle, Rm2kEngine::R2K, true, &mut len) };
            assert_eq!(take(saved, len), bytes);
            unsafe { $free(handle) };
        }
    };
}

round_trip!(
    round_trips_a_database,
    Database::defaults(Engine::R2K),
    file::save_database,
    rm2k_database_load,
    rm2k_database_save,
    rm2k_database_free
);

round_trip!(
    round_trips_a_map,
    Map::defaults(Engine::R2K),
    file::save_map,
    rm2k_map_load,
    rm2k_map_save,
    rm2k_map_free
);

round_trip!(
    round_trips_a_tree_map,
    TreeMap::defaults(Engine::R2K),
    file::save_tree_map,
    rm2k_tree_map_load,
    rm2k_tree_map_save,
    rm2k_tree_map_free
);

round_trip!(
    round_trips_a_save,
    Save::defaults(Engine::R2K),
    file::save_save,
    rm2k_save_load,
    rm2k_save_save,
    rm2k_save_free
);

#[test]
fn preserve_header_controls_the_magic() {
    let canonical = database_bytes();
    let mut altered = canonical.clone();
    // The length prefix is byte 0, so this turns "LcfDataBase" into "XcfDataBase".
    altered[1] = b'X';

    let mut err = Rm2kError::Ok;
    let handle = unsafe { rm2k_database_load(altered.as_ptr(), altered.len(), ptr::null_mut(), &mut err) };
    assert_eq!(err, Rm2kError::Ok);
    assert_eq!(lent(unsafe { rm2k_database_header(handle) }), b"XcfDataBase".as_slice());

    let mut len = 0usize;
    let kept = unsafe { rm2k_database_save(handle, Rm2kEngine::R2K, true, &mut len) };
    assert_eq!(take(kept, len), altered);

    let rewritten = unsafe { rm2k_database_save(handle, Rm2kEngine::R2K, false, &mut len) };
    assert_eq!(take(rewritten, len), canonical);
    unsafe { rm2k_database_free(handle) };
}

#[test]
fn a_bad_magic_is_reported_and_loading_still_succeeds() {
    let mut bytes = database_bytes();
    bytes[1] = b'X';

    let diag = rm2k_diagnostics_new();
    let mut err = Rm2kError::UnexpectedEof;
    let handle = unsafe { rm2k_database_load(bytes.as_ptr(), bytes.len(), diag, &mut err) };
    assert_eq!(err, Rm2kError::Ok);
    assert!(!handle.is_null());

    // The magic is checked before any chunk is read, so it is the first thing reported.
    let count = unsafe { rm2k_diagnostics_count(diag) };
    assert!(count >= 1);
    let mut first = no_diagnostic();
    assert!(unsafe { rm2k_diagnostics_get(diag, 0, &mut first) });
    assert_eq!(first.kind, RM2K_DIAG_BAD_MAGIC);
    assert_ne!(lent(first.struct_name), []);
    assert!(!unsafe { rm2k_diagnostics_get(diag, count, &mut first) });

    unsafe { rm2k_diagnostics_clear(diag) };
    assert_eq!(unsafe { rm2k_diagnostics_count(diag) }, 0);
    unsafe {
        rm2k_database_free(handle);
        rm2k_diagnostics_free(diag);
    }
}

#[test]
fn kind_names_are_static_and_unknown_codes_are_unknown_chunks() {
    let named = unsafe { CStr::from_ptr(rm2k_diagnostic_kind_name(RM2K_DIAG_BAD_MAGIC)) };
    assert_eq!(named.to_str(), Ok("unexpected file magic"));

    let budget = unsafe { CStr::from_ptr(rm2k_diagnostic_kind_name(RM2K_DIAG_ENTRY_BUDGET_EXHAUSTED)) };
    assert_eq!(budget.to_str(), Ok("string list over the load's entry budget"));

    let unknown = unsafe { CStr::from_ptr(rm2k_diagnostic_kind_name(u8::MAX)) };
    assert_eq!(unknown.to_str(), Ok("unknown chunk"));
}

#[test]
fn the_engine_follows_the_ldb_id() {
    assert_eq!(rm2k_engine_from_ldb_id(2003), Rm2kEngine::R2K3);
    assert_eq!(rm2k_engine_from_ldb_id(2000), Rm2kEngine::R2K);

    let bytes = database_bytes();
    let mut err = Rm2kError::Ok;
    let handle = unsafe { rm2k_database_load(bytes.as_ptr(), bytes.len(), ptr::null_mut(), &mut err) };
    assert_eq!(unsafe { rm2k_database_engine(handle) }, Rm2kEngine::R2K);
    unsafe { rm2k_database_free(handle) };
}

#[test]
fn list_accessors_read_the_database_entries() {
    let mut db = Database::defaults(Engine::R2K);
    let mut hero = Actor::defaults(Engine::R2K);
    hero.id = 1;
    hero.name = DbStr::from_bytes(b"Hero");
    db.actors.push(hero);
    let bytes = file::to_vec(|out| file::save_database(&db, out, Engine::R2K, SaveOpt::default(), b""));

    let mut err = Rm2kError::Ok;
    let handle = unsafe { rm2k_database_load(bytes.as_ptr(), bytes.len(), ptr::null_mut(), &mut err) };
    assert_eq!(err, Rm2kError::Ok);

    assert_eq!(unsafe { rm2k_database_list_len(handle, Rm2kDbList::Actors) }, 1);
    let mut named = Rm2kNamed {
        id: 0,
        name: Rm2kBytes::EMPTY,
    };
    assert!(unsafe { rm2k_database_list_get(handle, Rm2kDbList::Actors, 0, &mut named) });
    assert_eq!(named.id, 1);
    assert_eq!(lent(named.name), b"Hero".as_slice());
    assert!(!unsafe { rm2k_database_list_get(handle, Rm2kDbList::Actors, 1, &mut named) });
    unsafe { rm2k_database_free(handle) };
}

#[test]
fn split_header_and_chunk_walks_agree() {
    // A length-prefixed "LcfMapUnit", then one chunk (id 11, payload "z") and the terminator.
    let bytes = b"\x0ALcfMapUnit\x0B\x01z\x00";
    let mut header = Rm2kBytes::EMPTY;
    let mut body = Rm2kBytes::EMPTY;
    assert!(unsafe { rm2k_split_header(bytes.as_ptr(), bytes.len(), &mut header, &mut body) });
    assert_eq!(lent(header), b"LcfMapUnit".as_slice());
    assert_eq!(lent(body), b"\x0B\x01z\x00".as_slice());

    let mut chunk = Rm2kChunk {
        id: 0,
        offset: 0,
        data: Rm2kBytes::EMPTY,
    };
    assert!(unsafe { rm2k_chunk_find(body.data, body.len, 0, 0x0B, ptr::null_mut(), &mut chunk) });
    // The payload sits after the id and length bytes, so its file offset is 2.
    assert_eq!(chunk.offset, 2);
    assert_eq!(lent(chunk.data), b"z".as_slice());
    assert!(!unsafe { rm2k_chunk_find(body.data, body.len, 0, 0x0C, ptr::null_mut(), &mut chunk) });

    let mut iter = MaybeUninit::<Rm2kChunkIter>::uninit();
    assert!(unsafe { rm2k_chunk_iter_init(iter.as_mut_ptr(), body.data, body.len, 0) });
    assert!(unsafe { rm2k_chunk_iter_next(iter.as_mut_ptr(), ptr::null_mut(), &mut chunk) });
    assert_eq!(chunk.id, 0x0B);
    assert!(!unsafe { rm2k_chunk_iter_next(iter.as_mut_ptr(), ptr::null_mut(), &mut chunk) });
}

#[test]
fn an_overrun_chunk_is_cut_short_and_reported() {
    // The chunk claims 9 bytes but only 1 follows.
    let body = b"\x0B\x09a";
    let diag = rm2k_diagnostics_new();

    let mut iter = MaybeUninit::<Rm2kChunkIter>::uninit();
    let mut chunk = Rm2kChunk {
        id: 0,
        offset: 0,
        data: Rm2kBytes::EMPTY,
    };
    assert!(unsafe { rm2k_chunk_iter_init(iter.as_mut_ptr(), body.as_ptr(), body.len(), 0) });
    assert!(unsafe { rm2k_chunk_iter_next(iter.as_mut_ptr(), diag, &mut chunk) });
    assert_eq!(lent(chunk.data), b"a".as_slice());
    assert!(!unsafe { rm2k_chunk_iter_next(iter.as_mut_ptr(), diag, &mut chunk) });

    assert_eq!(unsafe { rm2k_diagnostics_count(diag) }, 1);
    let mut reported = no_diagnostic();
    assert!(unsafe { rm2k_diagnostics_get(diag, 0, &mut reported) });
    assert_eq!(reported.kind, RM2K_DIAG_CHUNK_OVERRUN);
    unsafe { rm2k_diagnostics_free(diag) };
}

#[test]
fn a_list_walker_yields_each_element_and_its_chunks() {
    // Count 1, then element id 5 holding chunk 11 with payload "z".
    let body = b"\x01\x05\x0B\x01z\x00";
    let mut walker = MaybeUninit::<Rm2kListWalker>::uninit();
    assert!(unsafe { rm2k_list_walker_init(walker.as_mut_ptr(), body.as_ptr(), body.len(), 0, ptr::null_mut()) });

    let mut item = Rm2kListItem {
        id: 0,
        offset: 0,
        body: Rm2kBytes::EMPTY,
    };
    assert!(unsafe { rm2k_list_walker_next(walker.as_mut_ptr(), ptr::null_mut(), &mut item) });
    assert_eq!(item.id, 5);

    let mut chunks = MaybeUninit::<Rm2kChunkIter>::uninit();
    assert!(unsafe { rm2k_chunk_iter_init(chunks.as_mut_ptr(), item.body.data, item.body.len, item.offset) });
    let mut chunk = Rm2kChunk {
        id: 0,
        offset: 0,
        data: Rm2kBytes::EMPTY,
    };
    assert!(unsafe { rm2k_chunk_iter_next(chunks.as_mut_ptr(), ptr::null_mut(), &mut chunk) });
    assert_eq!(chunk.id, 0x0B);
    assert_eq!(lent(chunk.data), b"z".as_slice());

    assert!(!unsafe { rm2k_list_walker_next(walker.as_mut_ptr(), ptr::null_mut(), &mut item) });
}

#[test]
fn walks_map_events_through_the_c_api() {
    let map_file = samples::map_with_events();
    let mut header = Rm2kBytes::EMPTY;
    let mut body = Rm2kBytes::EMPTY;
    assert!(unsafe { rm2k_split_header(map_file.as_ptr(), map_file.len(), &mut header, &mut body) });
    assert_eq!(lent(header), b"LcfMapUnit".as_slice());

    let mut lists = MaybeUninit::<Rm2kCommandLists>::uninit();
    assert!(unsafe {
        rm2k_command_lists_init(
            lists.as_mut_ptr(),
            Rm2kEventScope::MapEvents,
            body.data,
            body.len,
            0,
            ptr::null_mut(),
        )
    });

    let mut seen = Vec::new();
    let mut list = Rm2kCommandList {
        owner_id: 0,
        page_id: 0,
        offset: 0,
        commands: Rm2kBytes::EMPTY,
    };
    while unsafe { rm2k_command_lists_next(lists.as_mut_ptr(), ptr::null_mut(), &mut list) } {
        let mut commands = MaybeUninit::<Rm2kEventCommands>::uninit();
        assert!(unsafe {
            rm2k_event_commands_init(
                commands.as_mut_ptr(),
                list.commands.data,
                list.commands.len,
                list.offset,
            )
        });

        let mut command = Rm2kEventCommand {
            code: 0,
            indent: 0,
            param_count: 0,
            string: Rm2kBytes::EMPTY,
            raw_params: Rm2kBytes::EMPTY,
        };
        while unsafe { rm2k_event_commands_next(commands.as_mut_ptr(), ptr::null_mut(), &mut command) } {
            seen.push((list.owner_id, list.page_id, command.code, lent(command.string).to_vec()));

            // `samples::command` gives every command the parameters 1 and 2, and no third.
            assert_eq!(command.param_count, 2);
            let mut value = 0i32;
            assert!(unsafe { rm2k_event_command_param(&command, 0, &mut value) });
            assert_eq!(value, 1);
            assert!(unsafe { rm2k_event_command_param(&command, 1, &mut value) });
            assert_eq!(value, 2);
            assert!(!unsafe { rm2k_event_command_param(&command, 2, &mut value) });
        }
    }

    assert_eq!(
        seen,
        [
            (7, 1, 10110, b"page one".to_vec()),
            (7, 2, 10110, b"page two".to_vec()),
            (9, 1, 10110, vec![0x82, 0xA0, 0x82, 0xA2]),
        ]
    );
}

#[test]
fn the_allocator_hook_is_accepted_under_std() {
    unsafe extern "C" fn alloc(_size: usize, _align: usize) -> *mut u8 {
        ptr::null_mut()
    }
    unsafe extern "C" fn free(_ptr: *mut u8, _size: usize, _align: usize) {}
    // Under `std` the host's hooks are not used, so registering them does nothing.
    unsafe { rm2k_set_allocator(alloc, free) };
}

#[test]
fn every_read_error_maps_to_its_code() {
    assert_eq!(
        Rm2kError::from(ReadError::UnexpectedEof { offset: 0 }),
        Rm2kError::UnexpectedEof
    );
    assert_eq!(
        Rm2kError::from(ReadError::BadMagic {
            expected: "LcfDataBase",
            offset: 0
        }),
        Rm2kError::BadMagic
    );
    assert_eq!(
        Rm2kError::from(ReadError::VarintOverflow { offset: 0 }),
        Rm2kError::VarintOverflow
    );
    assert_eq!(
        Rm2kError::from(ReadError::ChunkOverrun {
            offset: 0,
            chunk_id: 1,
            len: 2
        }),
        Rm2kError::ChunkOverrun
    );
    let diagnostic = Diagnostic {
        offset: 0,
        chunk_id: 0,
        struct_name: "",
        kind: DiagKind::BadMagic,
    };
    assert_eq!(Rm2kError::from(ReadError::Strict(diagnostic)), Rm2kError::Strict);
}

#[test]
fn engines_convert_both_ways() {
    assert_eq!(Rm2kEngine::from(Engine::R2K), Rm2kEngine::R2K);
    assert_eq!(Rm2kEngine::from(Engine::R2K3), Rm2kEngine::R2K3);
    assert_eq!(Engine::from(Rm2kEngine::R2K), Engine::R2K);
    assert_eq!(Engine::from(Rm2kEngine::R2K3), Engine::R2K3);
}

#[test]
fn every_diagnostic_kind_has_its_own_code() {
    let pairs = [
        (DiagKind::UnknownChunk, RM2K_DIAG_UNKNOWN_CHUNK),
        (DiagKind::ShortChunk, RM2K_DIAG_SHORT_CHUNK),
        (DiagKind::TrailingBytes, RM2K_DIAG_TRAILING_BYTES),
        (DiagKind::BadVarint, RM2K_DIAG_BAD_VARINT),
        (DiagKind::ChunkOverrun, RM2K_DIAG_CHUNK_OVERRUN),
        (DiagKind::BadMagic, RM2K_DIAG_BAD_MAGIC),
        (DiagKind::EventListCorrupt, RM2K_DIAG_EVENT_LIST_CORRUPT),
        (DiagKind::RaggedArray, RM2K_DIAG_RAGGED_ARRAY),
        (DiagKind::EntryBudgetExhausted, RM2K_DIAG_ENTRY_BUDGET_EXHAUSTED),
    ];
    for (kind, code) in pairs {
        assert_eq!(diag_kind_index(kind), code);
    }
}

#[test]
fn a_diagnostic_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Rm2kDiagnostic>();
}

#[test]
fn diagnostics_refuse_a_null_output_and_clear_through_null() {
    let mut bad = database_bytes();
    bad[1] = b'X';
    let diag = rm2k_diagnostics_new();
    let mut err = Rm2kError::Ok;
    let handle = unsafe { rm2k_database_load(bad.as_ptr(), bad.len(), diag, &mut err) };
    assert!(unsafe { rm2k_diagnostics_count(diag) } >= 1);

    assert!(!unsafe { rm2k_diagnostics_get(diag, 0, ptr::null_mut()) });
    unsafe { rm2k_diagnostics_clear(ptr::null_mut()) };
    assert!(unsafe { rm2k_diagnostics_count(diag) } >= 1);

    unsafe {
        rm2k_database_free(handle);
        rm2k_diagnostics_free(diag);
    }
}

#[test]
fn loads_accept_a_null_error_out_and_saves_a_null_length_out() {
    let bytes = database_bytes();
    let handle = unsafe { rm2k_database_load(bytes.as_ptr(), bytes.len(), ptr::null_mut(), ptr::null_mut()) };
    assert!(!handle.is_null());

    let mut len = 0usize;
    let tracked = unsafe { rm2k_database_save(handle, Rm2kEngine::R2K, true, &mut len) };
    let untracked = unsafe { rm2k_database_save(handle, Rm2kEngine::R2K, true, ptr::null_mut()) };
    assert_eq!(take(tracked, len), bytes);
    // The untracked save is the same bytes, so the length from the tracked call frees it.
    assert_eq!(take(untracked, len), bytes);

    assert!(unsafe { rm2k_database_save(ptr::null(), Rm2kEngine::R2K, true, &mut len) }.is_null());
    unsafe { rm2k_database_free(handle) };
}

#[test]
fn header_accessors_return_the_magic_each_loader_read() {
    // The magic is a one-byte length, then the bytes it names.
    fn magic(file: &[u8]) -> &[u8] {
        &file[1..=usize::from(file[0])]
    }

    let map =
        file::to_vec(|out| file::save_map(&Map::defaults(Engine::R2K), out, Engine::R2K, SaveOpt::default(), b""));
    let tree = file::to_vec(|out| {
        file::save_tree_map(
            &TreeMap::defaults(Engine::R2K),
            out,
            Engine::R2K,
            SaveOpt::default(),
            b"",
        )
    });
    let save =
        file::to_vec(|out| file::save_save(&Save::defaults(Engine::R2K), out, Engine::R2K, SaveOpt::default(), b""));

    let mut err = Rm2kError::Ok;
    let map_handle = unsafe { rm2k_map_load(map.as_ptr(), map.len(), ptr::null_mut(), &mut err) };
    let tree_handle = unsafe { rm2k_tree_map_load(tree.as_ptr(), tree.len(), ptr::null_mut(), &mut err) };
    let save_handle = unsafe { rm2k_save_load(save.as_ptr(), save.len(), ptr::null_mut(), &mut err) };
    assert_eq!(lent(unsafe { rm2k_map_header(map_handle) }), magic(&map));
    assert_eq!(lent(unsafe { rm2k_tree_map_header(tree_handle) }), magic(&tree));
    assert_eq!(lent(unsafe { rm2k_save_header(save_handle) }), magic(&save));
    unsafe {
        rm2k_map_free(map_handle);
        rm2k_tree_map_free(tree_handle);
        rm2k_save_free(save_handle);
    }

    assert_eq!(lent(unsafe { rm2k_map_header(ptr::null()) }), []);
    assert_eq!(lent(unsafe { rm2k_tree_map_header(ptr::null()) }), []);
    assert_eq!(lent(unsafe { rm2k_save_header(ptr::null()) }), []);
}

#[test]
fn every_named_list_is_reachable_and_refuses_a_null_output() {
    let bytes = database_bytes();
    let mut err = Rm2kError::Ok;
    let handle = unsafe { rm2k_database_load(bytes.as_ptr(), bytes.len(), ptr::null_mut(), &mut err) };
    let lists = [
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
    ];
    for list in lists {
        let len = unsafe { rm2k_database_list_len(handle, list) };
        let mut named = Rm2kNamed {
            id: 0,
            name: Rm2kBytes::EMPTY,
        };
        assert!(!unsafe { rm2k_database_list_get(handle, list, len, &mut named) });
        assert!(!unsafe { rm2k_database_list_get(handle, list, 0, ptr::null_mut()) });
    }
    unsafe { rm2k_database_free(handle) };
}

#[test]
fn split_header_refuses_null_and_truncated_input() {
    let mut header = Rm2kBytes::EMPTY;
    let mut body = Rm2kBytes::EMPTY;
    assert!(!unsafe { rm2k_split_header(ptr::null(), 0, &mut header, &mut body) });

    // Announces a 10-byte magic but holds only three bytes of it.
    let truncated = b"\x0ALcf";
    assert!(!unsafe { rm2k_split_header(truncated.as_ptr(), truncated.len(), &mut header, &mut body) });

    let file = b"\x0ALcfMapUnit\x0B\x01z\x00";
    assert!(unsafe { rm2k_split_header(file.as_ptr(), file.len(), ptr::null_mut(), ptr::null_mut()) });
}

#[test]
fn chunk_and_list_walkers_refuse_null_arguments_and_accept_null_outputs() {
    let chunks = b"\x0B\x01z\x00";
    let mut chunk = Rm2kChunk {
        id: 0,
        offset: 0,
        data: Rm2kBytes::EMPTY,
    };
    let mut iter = MaybeUninit::<Rm2kChunkIter>::uninit();
    assert!(!unsafe { rm2k_chunk_iter_init(ptr::null_mut(), chunks.as_ptr(), chunks.len(), 0) });
    assert!(!unsafe { rm2k_chunk_iter_init(iter.as_mut_ptr(), ptr::null(), 0, 0) });
    assert!(!unsafe { rm2k_chunk_iter_next(ptr::null_mut(), ptr::null_mut(), &mut chunk) });
    assert!(!unsafe { rm2k_chunk_find(ptr::null(), 0, 0, 0x0B, ptr::null_mut(), &mut chunk) });

    assert!(unsafe { rm2k_chunk_iter_init(iter.as_mut_ptr(), chunks.as_ptr(), chunks.len(), 0) });
    assert!(unsafe { rm2k_chunk_iter_next(iter.as_mut_ptr(), ptr::null_mut(), ptr::null_mut()) });
    assert!(unsafe { rm2k_chunk_find(chunks.as_ptr(), chunks.len(), 0, 0x0B, ptr::null_mut(), ptr::null_mut()) });

    let list = b"\x01\x05\x0B\x01z\x00";
    let mut walker = MaybeUninit::<Rm2kListWalker>::uninit();
    assert!(!unsafe { rm2k_list_walker_init(ptr::null_mut(), list.as_ptr(), list.len(), 0, ptr::null_mut()) });
    assert!(!unsafe { rm2k_list_walker_init(walker.as_mut_ptr(), ptr::null(), 0, 0, ptr::null_mut()) });
    assert!(!unsafe { rm2k_list_walker_next(ptr::null_mut(), ptr::null_mut(), ptr::null_mut()) });

    assert!(unsafe { rm2k_list_walker_init(walker.as_mut_ptr(), list.as_ptr(), list.len(), 0, ptr::null_mut()) });
    assert!(unsafe { rm2k_list_walker_next(walker.as_mut_ptr(), ptr::null_mut(), ptr::null_mut()) });
}

/// The map file's body: its one-byte length, the 10-byte `LcfMapUnit` magic, then the chunks.
fn map_body() -> Vec<u8> {
    samples::map_with_events()[11..].to_vec()
}

/// The (owner, page, text) of every command the C walkers find in `body` under `scope`.
fn walk_scope(scope: Rm2kEventScope, body: &[u8]) -> Vec<(i32, i32, Vec<u8>)> {
    let mut seen = Vec::new();
    let mut lists = MaybeUninit::<Rm2kCommandLists>::uninit();
    assert!(unsafe {
        rm2k_command_lists_init(lists.as_mut_ptr(), scope, body.as_ptr(), body.len(), 0, ptr::null_mut())
    });
    let mut list = Rm2kCommandList {
        owner_id: 0,
        page_id: 0,
        offset: 0,
        commands: Rm2kBytes::EMPTY,
    };
    while unsafe { rm2k_command_lists_next(lists.as_mut_ptr(), ptr::null_mut(), &mut list) } {
        let mut commands = MaybeUninit::<Rm2kEventCommands>::uninit();
        assert!(unsafe {
            rm2k_event_commands_init(
                commands.as_mut_ptr(),
                list.commands.data,
                list.commands.len,
                list.offset,
            )
        });
        let mut command = Rm2kEventCommand {
            code: 0,
            indent: 0,
            param_count: 0,
            string: Rm2kBytes::EMPTY,
            raw_params: Rm2kBytes::EMPTY,
        };
        while unsafe { rm2k_event_commands_next(commands.as_mut_ptr(), ptr::null_mut(), &mut command) } {
            seen.push((list.owner_id, list.page_id, lent(command.string).to_vec()));
        }
    }
    seen
}

#[test]
fn common_events_and_battle_events_are_walked_by_their_scopes() {
    let mut db = Database::defaults(Engine::R2K);
    let mut common = CommonEvent::defaults(Engine::R2K);
    common.id = 3;
    common.event_commands = EventCommandList(vec![samples::command(10110, b"common")]);
    db.commonevents = List(vec![common]);

    let mut troop = Troop::defaults(Engine::R2K);
    troop.id = 2;
    let mut page = TroopPage::defaults(Engine::R2K);
    page.id = 1;
    page.event_commands = EventCommandList(vec![samples::command(10110, b"battle")]);
    troop.pages = List(vec![page]);
    db.troops = List(vec![troop]);

    let mut body = Vec::new();
    db.write_body(&mut body, Engine::R2K).unwrap();

    assert_eq!(
        walk_scope(Rm2kEventScope::CommonEvents, &body),
        vec![(3, 0, b"common".to_vec())]
    );
    assert_eq!(
        walk_scope(Rm2kEventScope::BattleEvents, &body),
        vec![(2, 1, b"battle".to_vec())]
    );
}

#[test]
fn event_walkers_refuse_null_arguments_and_accept_null_outputs() {
    let body = map_body();
    let mut lists = MaybeUninit::<Rm2kCommandLists>::uninit();
    assert!(!unsafe {
        rm2k_command_lists_init(
            ptr::null_mut(),
            Rm2kEventScope::MapEvents,
            body.as_ptr(),
            body.len(),
            0,
            ptr::null_mut(),
        )
    });
    assert!(!unsafe {
        rm2k_command_lists_init(
            lists.as_mut_ptr(),
            Rm2kEventScope::MapEvents,
            ptr::null(),
            0,
            0,
            ptr::null_mut(),
        )
    });
    assert!(!unsafe { rm2k_command_lists_next(ptr::null_mut(), ptr::null_mut(), ptr::null_mut()) });

    assert!(unsafe {
        rm2k_command_lists_init(
            lists.as_mut_ptr(),
            Rm2kEventScope::MapEvents,
            body.as_ptr(),
            body.len(),
            0,
            ptr::null_mut(),
        )
    });
    // The first list with no output still advances the walk.
    assert!(unsafe { rm2k_command_lists_next(lists.as_mut_ptr(), ptr::null_mut(), ptr::null_mut()) });
    let mut list = Rm2kCommandList {
        owner_id: 0,
        page_id: 0,
        offset: 0,
        commands: Rm2kBytes::EMPTY,
    };
    assert!(unsafe { rm2k_command_lists_next(lists.as_mut_ptr(), ptr::null_mut(), &mut list) });

    let mut commands = MaybeUninit::<Rm2kEventCommands>::uninit();
    assert!(!unsafe { rm2k_event_commands_init(ptr::null_mut(), body.as_ptr(), body.len(), 0) });
    assert!(!unsafe { rm2k_event_commands_init(commands.as_mut_ptr(), ptr::null(), 0, 0) });
    assert!(!unsafe { rm2k_event_commands_next(ptr::null_mut(), ptr::null_mut(), ptr::null_mut()) });
    assert!(unsafe {
        rm2k_event_commands_init(
            commands.as_mut_ptr(),
            list.commands.data,
            list.commands.len,
            list.offset,
        )
    });
    let mut command = Rm2kEventCommand {
        code: 0,
        indent: 0,
        param_count: 0,
        string: Rm2kBytes::EMPTY,
        raw_params: Rm2kBytes::EMPTY,
    };
    assert!(unsafe { rm2k_event_commands_next(commands.as_mut_ptr(), ptr::null_mut(), &mut command) });
    // A parameter read with a null output still succeeds; it just writes nothing.
    assert!(unsafe { rm2k_event_command_param(&command, 0, ptr::null_mut()) });
    // A command read with no output still advances: re-initialise the same list and read it that way.
    assert!(unsafe {
        rm2k_event_commands_init(
            commands.as_mut_ptr(),
            list.commands.data,
            list.commands.len,
            list.offset,
        )
    });
    assert!(unsafe { rm2k_event_commands_next(commands.as_mut_ptr(), ptr::null_mut(), ptr::null_mut()) });

    // A parameter read refuses a null command and a command with no parameter bytes.
    let mut value = 0i32;
    assert!(!unsafe { rm2k_event_command_param(ptr::null(), 0, &mut value) });
    let bare = Rm2kEventCommand {
        code: 0,
        indent: 0,
        param_count: 1,
        string: Rm2kBytes::EMPTY,
        raw_params: Rm2kBytes::EMPTY,
    };
    assert!(!unsafe { rm2k_event_command_param(&bare, 0, &mut value) });
}

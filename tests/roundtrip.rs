//! End-to-end round-trips over populated structures.
//!
//! The unit tests cover each codec in isolation; these check that a whole file survives the trip
//! through all of them at once. The property that matters is **byte-exactness**: save, load, save
//! again, and the two byte streams must be identical. That is what catches default-elision
//! mistakes, companion-chunk ordering mistakes and flag-packing mistakes, none of which show up
//! in a value comparison.

use rm2k::{
    field::{LcfStruct, List, Packed, UnknownChunks},
    owned::IntoOwned,
    raw::{EventCommand, EventCommandList},
    rpg::{Actor, Database, Learning, Map, Save, TreeMap},
    {DbStr, Diagnostic, Engine, SaveOpt, file},
};

/// Save, reload, re-save. The two byte streams must match exactly.
fn assert_stable<T, S, L>(value: &T, save: S, load: L, engine: Engine)
where
    S: Fn(&T, &mut Vec<u8>, Engine),
    L: for<'a> Fn(&'a [u8]) -> T,
    T: PartialEq + core::fmt::Debug,
{
    let mut first = Vec::new();
    save(value, &mut first, engine);

    let reloaded = load(&first);
    assert_eq!(&reloaded, value, "value changed across a round-trip");

    let mut second = Vec::new();
    save(&reloaded, &mut second, engine);
    assert_eq!(first, second, "re-saving produced different bytes");
}

fn populated_database() -> Database<'static> {
    let mut db = Database::defaults(Engine::R2K);

    db.system.ldb_id = 2000;
    // Deliberately not UTF-8: this is Shift-JIS, and it must survive untouched.
    db.system.system_name = DbStr::from_vec(vec![0x82, 0xA0, 0x82, 0xA2]);

    for id in 1..=3 {
        let mut actor = Actor::defaults(Engine::R2K);
        actor.id = id;
        actor.name = DbStr::from_vec(format!("Actor{id}").into_bytes());
        actor.initial_level = id;
        // Exercises a companion `size` chunk.
        actor.state_ranks = Packed(vec![1, 2, 3, 4, 5]);
        actor.skills = List(vec![Learning {
            id: 1,
            level: 5,
            skill_id: 2,
            unknown_chunks: UnknownChunks::new(),
        }]);
        db.actors.push(actor);
    }

    db
}

#[test]
fn a_populated_database_is_byte_stable() {
    let db = populated_database();
    assert_stable(
        &db,
        |db, out, engine| {
            file::save_database(db, out, engine, SaveOpt::default(), b"").unwrap();
        },
        |bytes| file::load_database(bytes).unwrap().value.into_owned(),
        Engine::R2K,
    );
}

#[test]
fn a_populated_database_is_byte_stable_under_r2k3() {
    let mut db = populated_database();
    db.system.ldb_id = 2003;
    assert_stable(
        &db,
        |db, out, engine| {
            file::save_database(db, out, engine, SaveOpt::default(), b"").unwrap();
        },
        |bytes| file::load_database(bytes).unwrap().value.into_owned(),
        Engine::R2K3,
    );
}

#[test]
fn engine_is_recovered_from_the_database() {
    let mut db = populated_database();
    assert_eq!(Engine::from_ldb_id(db.system.ldb_id), Engine::R2K);
    db.system.ldb_id = 2003;
    assert_eq!(Engine::from_ldb_id(db.system.ldb_id), Engine::R2K3);
}

#[test]
fn a_map_with_events_is_byte_stable() {
    let mut map = Map::defaults(Engine::R2K);
    map.chipset_id = 3;
    map.width = 20;
    map.height = 15;

    let mut event = rm2k::rpg::Event::defaults(Engine::R2K);
    event.id = 1;
    event.name = DbStr::from_bytes(b"EV0001");
    event.x = 4;
    event.y = 5;

    let mut page = rm2k::rpg::EventPage::defaults(Engine::R2K);
    page.id = 1;
    page.event_commands = EventCommandList(vec![
        EventCommand {
            code: 10110,
            indent: 0,
            string: DbStr::from_bytes(b"Hello"),
            parameters: vec![],
        },
        EventCommand {
            code: 20110,
            indent: 0,
            // Shift-JIS again, inside an event command this time.
            string: DbStr::from_vec(vec![0x82, 0xA0]),
            parameters: vec![1, 2, 3],
        },
    ]);
    event.pages = List(vec![page]);
    map.events = List(vec![event]);

    assert_stable(
        &map,
        |m, out, engine| {
            file::save_map(m, out, engine, SaveOpt::default(), b"").unwrap();
        },
        |bytes| file::load_map(bytes).unwrap().value.into_owned(),
        Engine::R2K,
    );
}

#[test]
fn a_tree_map_is_byte_stable() {
    let mut tree = TreeMap::defaults(Engine::R2K);
    tree.active_node = 2;
    tree.tree_order = vec![0, 1, 2];

    let mut info = rm2k::rpg::MapInfo::defaults(Engine::R2K);
    info.id = 1;
    info.name = DbStr::from_bytes(b"World");
    tree.maps = List(vec![info]);

    assert_stable(
        &tree,
        |t, out, engine| {
            file::save_tree_map(t, out, engine, SaveOpt::default(), b"").unwrap();
        },
        |bytes| file::load_tree_map(bytes).unwrap().value.into_owned(),
        Engine::R2K,
    );
}

#[test]
fn a_save_is_byte_stable() {
    let mut save = Save::defaults(Engine::R2K);
    save.title.hero_name = DbStr::from_bytes(b"Hero");
    save.system.save_count = 7;

    assert_stable(
        &save,
        |s, out, engine| {
            file::save_save(s, out, engine, SaveOpt::default(), b"").unwrap();
        },
        |bytes| file::load_save(bytes).unwrap().value.into_owned(),
        Engine::R2K,
    );
}

#[test]
fn a_clean_file_produces_no_diagnostics() {
    let db = populated_database();
    let mut bytes = Vec::new();
    file::save_database(&db, &mut bytes, Engine::R2K, SaveOpt::default(), b"").unwrap();

    let mut diag: Vec<Diagnostic> = Vec::new();
    let loaded = file::load_database_with(&bytes, &mut diag).unwrap();
    assert_eq!(loaded.value, db);
    assert!(diag.is_empty(), "a file this crate wrote must reload cleanly: {diag:?}");
}

#[test]
fn size_always_predicts_the_bytes_written() {
    // The invariant the two-pass writer depends on: if `body_size` and `write_body` ever
    // disagree, every chunk length in the output is wrong.
    for engine in [Engine::R2K, Engine::R2K3] {
        let db = populated_database();
        let mut counted = rm2k::CountSink::new();
        db.write_body(&mut counted, engine).unwrap();
        assert_eq!(counted.len(), u64::from(db.body_size(engine)), "{engine:?}");
    }
}

#[test]
fn truncating_a_file_anywhere_never_panics() {
    // The reader is lenient by design, but leniency must not mean "reads out of bounds".
    let db = populated_database();
    let mut bytes = Vec::new();
    file::save_database(&db, &mut bytes, Engine::R2K, SaveOpt::default(), b"").unwrap();

    for cut in 0..bytes.len() {
        let mut diag: Vec<Diagnostic> = Vec::new();
        // Errors are fine; panics are not.
        let _ = file::load_database_with(&bytes[..cut], &mut diag);
    }
}

#[test]
fn flipping_any_single_byte_never_panics() {
    let db = populated_database();
    let mut bytes = Vec::new();
    file::save_database(&db, &mut bytes, Engine::R2K, SaveOpt::default(), b"").unwrap();

    for index in 0..bytes.len() {
        for mask in [0x01u8, 0x80, 0xFF] {
            let mut corrupted = bytes.clone();
            corrupted[index] ^= mask;
            let mut diag: Vec<Diagnostic> = Vec::new();
            let _ = file::load_database_with(&corrupted, &mut diag);
        }
    }
}

#[test]
fn strings_are_never_transcoded() {
    // The crate's central promise. A Shift-JIS name goes in, the identical bytes come out.
    const SHIFT_JIS: &[u8] = &[0x83, 0x8D, 0x83, 0x76, 0x83, 0x8C]; // "ロプレ"
    let mut db = Database::defaults(Engine::R2K);
    let mut actor = Actor::defaults(Engine::R2K);
    actor.id = 1;
    actor.name = DbStr::from_bytes(SHIFT_JIS);
    db.actors.push(actor);

    let mut bytes = Vec::new();
    file::save_database(&db, &mut bytes, Engine::R2K, SaveOpt::default(), b"").unwrap();
    let loaded = file::load_database(&bytes).unwrap();

    assert_eq!(loaded.value.actors[0].name.as_bytes(), SHIFT_JIS);
    assert!(loaded.value.actors[0].name.as_utf8().is_none());
}

#[test]
fn loading_borrows_and_into_owned_detaches() {
    let db = populated_database();
    let mut bytes = Vec::new();
    file::save_database(&db, &mut bytes, Engine::R2K, SaveOpt::default(), b"").unwrap();

    let detached: Database<'static> = {
        let loaded = file::load_database(&bytes).unwrap();
        // Borrowed: the name points into `bytes`.
        assert_eq!(loaded.value.actors[0].name.as_bytes(), b"Actor1");
        loaded.value.into_owned()
    };
    drop(bytes);
    assert_eq!(detached.actors[0].name.as_bytes(), b"Actor1");
}

/// Every chunk id present in a struct body, in the order written.
fn chunk_ids(body: &[u8]) -> Vec<u32> {
    let mut sink = rm2k::NullSink;
    let mut ctx = rm2k::Ctx::new(Engine::R2K, &mut sink);
    let mut chunks = rm2k::ChunkIter::new(body, 0);
    let mut ids = Vec::new();
    while let Some(chunk) = chunks.next(&mut ctx, "T") {
        ids.push(chunk.id);
    }
    ids
}

#[test]
fn terms_encounter_and_escape_success_lose_persist_under_r2k3() {
    // liblcf hard-codes this in `Field::isPresentIfDefault` (`src/reader_struct.h:383`): Terms
    // chunks 0x01 and 0x03 are `PersistIfDefault` in the tables, but a 2003 database elides them
    // at their default anyway. They are the only two fields in the schema that do this.
    let terms = rm2k::rpg::Terms::defaults(Engine::R2K);

    let mut as_2k = Vec::new();
    terms.write_body(&mut as_2k, Engine::R2K).unwrap();
    let ids_2k = chunk_ids(&as_2k);
    assert!(ids_2k.contains(&0x01), "0x01 must be written under 2000");
    assert!(ids_2k.contains(&0x03), "0x03 must be written under 2000");

    let mut as_2k3 = Vec::new();
    terms.write_body(&mut as_2k3, Engine::R2K3).unwrap();
    let ids_2k3 = chunk_ids(&as_2k3);
    assert!(!ids_2k3.contains(&0x01), "0x01 must be elided under 2003");
    assert!(!ids_2k3.contains(&0x03), "0x03 must be elided under 2003");

    // Not a length comparison: Terms also has 2003-only fields, so the 2003 body is not
    // uniformly smaller. Only these two ids differ in this direction.
    let only_in_2k: Vec<_> = ids_2k.iter().filter(|id| !ids_2k3.contains(id)).copied().collect();
    assert_eq!(only_in_2k, vec![0x01, 0x03]);
}

#[test]
fn terms_still_writes_those_fields_under_r2k3_when_set() {
    let mut terms = rm2k::rpg::Terms::defaults(Engine::R2K3);
    terms.encounter = DbStr::from_bytes(b"ambush!");

    let mut out = Vec::new();
    terms.write_body(&mut out, Engine::R2K3).unwrap();
    assert!(out.starts_with(&[0x01, 0x07]), "a non-default value is still written");
}

#[test]
fn a_file_carrying_unrecognised_chunks_is_byte_stable() {
    // The case this crate could not previously survive: a file with chunk ids the schema has no
    // field for - a later RPG_RT build, an EasyRPG extension, a third-party editor. They used to
    // be reported and dropped, so writing the file back silently lost them.
    let mut db = populated_database();

    // 0xF0 belongs to no `Actor` field, and 0xF1 to no `Database` field. Anchors chosen to land
    // both mid-body rather than at an end, which is the position a lost chunk would show up in.
    db.actors[0].unknown_chunks.push(rm2k::UnknownChunk {
        data: vec![0xDE, 0xAD, 0xBE, 0xEF],
        id: 0xF0,
        anchor: 0x01, // just after `Actor::name`
    });
    db.unknown_chunks.push(rm2k::UnknownChunk {
        data: vec![0x2A],
        id: 0xF1,
        anchor: 0x0B, // just after `Database::actors`
    });

    let mut bytes = Vec::new();
    file::save_database(&db, &mut bytes, Engine::R2K, SaveOpt::default(), b"").unwrap();

    let mut diag: Vec<Diagnostic> = Vec::new();
    let reloaded = file::load_database_with(&bytes, &mut diag).unwrap().value.into_owned();

    // Still reported - preserved is not understood.
    assert_eq!(diag.len(), 2, "both unknown chunks must be reported: {diag:?}");
    assert!(diag.iter().all(|d| d.kind == rm2k::DiagKind::UnknownChunk));

    assert_eq!(reloaded, db, "an unrecognised chunk must survive the trip");

    let mut second = Vec::new();
    file::save_database(&reloaded, &mut second, Engine::R2K, SaveOpt::default(), b"").unwrap();
    assert_eq!(bytes, second, "re-saving produced different bytes");
}

#[test]
fn a_strict_load_still_rejects_an_unrecognised_chunk() {
    let mut db = populated_database();
    db.actors[0].unknown_chunks.push(rm2k::UnknownChunk {
        data: vec![0x01],
        id: 0xF0,
        anchor: 0x01,
    });

    let mut bytes = Vec::new();
    file::save_database(&db, &mut bytes, Engine::R2K, SaveOpt::default(), b"").unwrap();

    let mut strict = rm2k::StrictSink::new();
    let _ = file::load_database_with(&bytes, &mut strict).unwrap();
    assert_eq!(
        rm2k::DiagSink::first(&strict).map(|d| d.kind),
        Some(rm2k::DiagKind::UnknownChunk)
    );
}

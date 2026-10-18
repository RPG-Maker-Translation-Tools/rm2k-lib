//! Tests for `rm2k::file` - whole-file loading and saving (`load_database`, `save_map`, ...).

use rm2k::{
    Engine, LcfStruct, ReadError, SaveOpt,
    diag::{DiagKind, Diagnostic},
    file::{
        load_database, load_database_with, load_map, load_save, load_tree_map, load_tree_map_with, save_database,
        save_map, save_tree_map,
    },
    rpg::{Database, Map, TreeMap},
    wire,
};

#[test]
fn a_database_round_trips_through_its_header() {
    let mut buf = Vec::new();
    let db = Database::defaults(Engine::R2K);
    save_database(&db, &mut buf, Engine::R2K, SaveOpt::default(), b"").unwrap();

    assert_eq!(buf[0] as usize, wire::LDB_HEADER.len());
    assert_eq!(&buf[1..=wire::LDB_HEADER.len()], wire::LDB_HEADER);

    let loaded = load_database(&buf).unwrap();
    assert_eq!(loaded.header, wire::LDB_HEADER);
    assert_eq!(loaded.value, db);
}

#[test]
fn a_wrong_magic_is_reported_but_still_parses() {
    let mut buf = vec![10u8];
    buf.extend_from_slice(b"NotAnLcfDb");
    buf.push(0);

    let mut diag: Vec<Diagnostic> = Vec::new();
    let loaded = load_database_with(&buf, &mut diag).unwrap();
    assert_eq!(loaded.header, b"NotAnLcfDb");
    assert_eq!(diag[0].kind, DiagKind::BadMagic);
}

#[test]
fn preserve_header_keeps_a_non_canonical_magic() {
    let db = Database::defaults(Engine::R2K);
    let mut buf = Vec::new();
    let opt = SaveOpt { preserve_header: true };
    save_database(&db, &mut buf, Engine::R2K, opt, b"Custom").unwrap();
    assert_eq!(load_database(&buf).unwrap().header, b"Custom");
}

#[test]
fn an_empty_buffer_is_an_error_not_a_panic() {
    assert!(matches!(load_database(&[]), Err(ReadError::UnexpectedEof { .. })));
    assert!(matches!(load_map(&[]), Err(ReadError::UnexpectedEof { .. })));
    assert!(matches!(load_tree_map(&[]), Err(ReadError::UnexpectedEof { .. })));
    assert!(matches!(load_save(&[]), Err(ReadError::UnexpectedEof { .. })));
}

#[test]
fn database_and_save_omit_the_terminating_zero_but_map_does_not() {
    // liblcf's `conditional_zero_writer` quirk (`src/reader_struct_impl.h:97`).
    let mut db_buf = Vec::new();
    save_database(
        &Database::defaults(Engine::R2K),
        &mut db_buf,
        Engine::R2K,
        SaveOpt::default(),
        b"",
    )
    .unwrap();

    let mut map_buf = Vec::new();
    save_map(
        &Map::defaults(Engine::R2K),
        &mut map_buf,
        Engine::R2K,
        SaveOpt::default(),
        b"",
    )
    .unwrap();

    // Compare lengths rather than the last byte: a body can legitimately end in a zero, so only
    // the total says whether a terminator was appended.
    let prefix = 1 + wire::LDB_HEADER.len() as u32;
    assert_eq!(
        db_buf.len() as u32,
        prefix + Database::defaults(Engine::R2K).body_size(Engine::R2K),
        "Database must not append a terminator"
    );
    let prefix = 1 + wire::LMU_HEADER.len() as u32;
    assert_eq!(
        map_buf.len() as u32,
        prefix + Map::defaults(Engine::R2K).body_size(Engine::R2K) + 1,
        "Map must append a terminator"
    );
}

#[test]
fn a_tree_map_round_trips() {
    let tree = TreeMap::defaults(Engine::R2K);
    let mut buf = Vec::new();
    save_tree_map(&tree, &mut buf, Engine::R2K, SaveOpt::default(), b"").unwrap();
    let loaded = load_tree_map(&buf).unwrap();
    assert_eq!(loaded.header, wire::LMT_HEADER);
    assert_eq!(loaded.value, tree);
}

#[test]
fn a_loaded_value_derefs_to_its_contents() {
    let db = Database::defaults(Engine::R2K);
    let mut buf = Vec::new();
    save_database(&db, &mut buf, Engine::R2K, SaveOpt::default(), b"").unwrap();
    let loaded = load_database(&buf).unwrap();
    assert_eq!(loaded.system, db.system);
}

#[test]
fn a_tree_map_with_a_bad_magic_is_reported_but_still_parses() {
    let tree = TreeMap::defaults(Engine::R2K);
    let mut buf = Vec::new();
    let opt = SaveOpt { preserve_header: true };
    save_tree_map(&tree, &mut buf, Engine::R2K, opt, b"Bogus").unwrap();

    let mut diag: Vec<Diagnostic> = Vec::new();
    let loaded = load_tree_map_with(&buf, &mut diag).unwrap();
    assert_eq!(loaded.header, b"Bogus");
    assert_eq!(diag[0].kind, DiagKind::BadMagic);
    assert_eq!(loaded.value, tree);
}

#[test]
fn to_vec_collects_exactly_what_the_save_writes() {
    let db = Database::defaults(Engine::R2K);
    let mut direct = Vec::new();
    save_database(&db, &mut direct, Engine::R2K, SaveOpt::default(), b"").unwrap();

    let via = rm2k::file::to_vec(|out| save_database(&db, out, Engine::R2K, SaveOpt::default(), b""));
    assert_eq!(via, direct);
}

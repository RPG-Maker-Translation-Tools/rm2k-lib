#![cfg(target_arch = "wasm32")]

use js_sys::Array;
use rm2k::{
    Engine, LcfStruct, SaveOpt, file,
    rpg::{Database, Map, Save, TreeMap},
};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_test::wasm_bindgen_test;

#[wasm_bindgen_test]
fn database_round_trips_through_the_wasm_boundary() {
    let value = Database::defaults(Engine::R2K);
    let original = file::to_vec(|out| file::save_database(&value, out, Engine::R2K, SaveOpt::default(), b""));

    let loaded = rm2k_wasm::load_database(&original).unwrap();
    let round_tripped = rm2k_wasm::save_database(loaded.value, rm2k_wasm::Engine::R2K, false, &[]).unwrap();
    assert_eq!(round_tripped, original);
}

#[wasm_bindgen_test]
fn map_round_trips_through_the_wasm_boundary() {
    let value = Map::defaults(Engine::R2K);
    let original = file::to_vec(|out| file::save_map(&value, out, Engine::R2K, SaveOpt::default(), b""));

    let loaded = rm2k_wasm::load_map(&original).unwrap();
    let round_tripped = rm2k_wasm::save_map(loaded.value, rm2k_wasm::Engine::R2K, false, &[]).unwrap();
    assert_eq!(round_tripped, original);
}

#[wasm_bindgen_test]
fn save_round_trips_through_the_wasm_boundary() {
    let value = Save::defaults(Engine::R2K);
    let original = file::to_vec(|out| file::save_save(&value, out, Engine::R2K, SaveOpt::default(), b""));

    let loaded = rm2k_wasm::load_save(&original).unwrap();
    let round_tripped = rm2k_wasm::save_save(loaded.value, rm2k_wasm::Engine::R2K, false, &[]).unwrap();
    assert_eq!(round_tripped, original);
}

#[wasm_bindgen_test]
fn tree_map_round_trips_through_the_wasm_boundary() {
    let value = TreeMap::defaults(Engine::R2K);
    let original = file::to_vec(|out| file::save_tree_map(&value, out, Engine::R2K, SaveOpt::default(), b""));

    let loaded = rm2k_wasm::load_tree_map(&original).unwrap();
    let round_tripped = rm2k_wasm::save_tree_map(loaded.value, rm2k_wasm::Engine::R2K, false, &[]).unwrap();
    assert_eq!(round_tripped, original);
}

#[wasm_bindgen_test]
fn engine_from_ldb_id_is_2003_or_2000() {
    assert!(matches!(rm2k_wasm::engine_from_ldb_id(2003), rm2k_wasm::Engine::R2K3));
    assert!(matches!(rm2k_wasm::engine_from_ldb_id(2000), rm2k_wasm::Engine::R2K));
    assert!(matches!(rm2k_wasm::engine_from_ldb_id(0), rm2k_wasm::Engine::R2K));
}

#[wasm_bindgen_test]
fn loading_fewer_bytes_than_a_header_is_an_error() {
    assert!(rm2k_wasm::load_database(&[]).is_err());
    assert!(rm2k_wasm::load_map(&[]).is_err());
}

#[wasm_bindgen_test]
fn saving_a_value_that_is_not_a_database_is_an_error() {
    let not_a_database = JsValue::from_f64(1.0);
    assert!(rm2k_wasm::save_database(not_a_database, rm2k_wasm::Engine::R2K, false, &[]).is_err());
}

#[wasm_bindgen_test]
fn a_bad_magic_is_a_diagnostic_and_the_header_survives_the_boundary() {
    let mut altered = file::to_vec(|out| {
        file::save_database(
            &Database::defaults(Engine::R2K),
            out,
            Engine::R2K,
            SaveOpt::default(),
            b"",
        )
    });
    // The length prefix is byte 0, so this turns "LcfDataBase" into "XcfDataBase".
    altered[1] = b'X';

    let loaded = rm2k_wasm::load_database(&altered).unwrap();
    assert_eq!(loaded.header, b"XcfDataBase");
    let diagnostics: Array = loaded.diagnostics.unchecked_into();
    assert!(diagnostics.length() >= 1);

    let header = loaded.header.clone();
    let kept = rm2k_wasm::save_database(loaded.value, rm2k_wasm::Engine::R2K, true, &header).unwrap();
    assert_eq!(kept, altered);
}

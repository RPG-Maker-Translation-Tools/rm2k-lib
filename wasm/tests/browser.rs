#![cfg(target_arch = "wasm32")]

//! Browser tests: the same boundary `smoke.rs` covers, but checked against the JavaScript values a
//! page actually receives - the shape of `LoadResult`, the `Uint8Array` header, and the camelCase
//! diagnostics. Run with `wasm-pack test --headless --chrome`.

use js_sys::{Array, Object, Reflect};
use rm2k::{
    Engine, LcfStruct, SaveOpt, file,
    rpg::{Database, Map},
};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

wasm_bindgen_test_configure!(run_in_browser);

fn get(target: &JsValue, key: &str) -> JsValue {
    Reflect::get(target, &JsValue::from_str(key)).unwrap()
}

#[wasm_bindgen_test]
fn load_result_exposes_header_as_bytes_and_value_as_an_object() {
    let db = Database::defaults(Engine::R2K);
    let bytes = file::to_vec(|out| file::save_database(&db, out, Engine::R2K, SaveOpt::default(), b""));

    let loaded = rm2k_wasm::load_database(&bytes).unwrap();
    assert!(loaded.value.is_object());
    assert_eq!(loaded.header, rm2k::wire::LDB_HEADER);
    assert_eq!(Array::from(&loaded.diagnostics).length(), 0);
}

#[wasm_bindgen_test]
fn diagnostics_arrive_as_camel_case_objects() {
    // A wrong magic is recoverable: the file still loads, and the problem is reported.
    let mut bytes = vec![10u8];
    bytes.extend_from_slice(b"NotAnLcfDb");
    bytes.push(0);

    let loaded = rm2k_wasm::load_database(&bytes).unwrap();
    let diagnostics = Array::from(&loaded.diagnostics);
    assert_eq!(diagnostics.length(), 1);

    let first = diagnostics.get(0);
    assert_eq!(
        get(&first, "kind").as_string().as_deref(),
        Some("unexpected file magic")
    );
    assert!(get(&first, "structName").is_string());
    assert!(get(&first, "chunkId").as_f64().is_some());
    assert!(get(&first, "offset").as_f64().is_some());
    assert!(Object::keys(first.unchecked_ref::<Object>()).length() == 4);
}

#[wasm_bindgen_test]
fn a_failed_load_throws_a_real_error_object() {
    let Err(err) = rm2k_wasm::load_database(&[]) else {
        panic!("an empty file must not load");
    };
    let thrown: JsValue = err.into();
    assert!(thrown.is_instance_of::<js_sys::Error>());
    assert!(get(&thrown, "message").as_string().is_some_and(|m| !m.is_empty()));
}

#[wasm_bindgen_test]
fn strings_cross_the_boundary_as_reversible_escapes_not_text() {
    // Shift-JIS, which is not valid UTF-8. JS sees the same reversible `\xNN` escape the JSON tier
    // uses, so the bytes are never decoded as text and nothing is lost.
    let mut db = Database::defaults(Engine::R2K);
    db.system.system_name = rm2k::DbStr::from_vec(vec![0x82, 0xA0]);
    let bytes = file::to_vec(|out| file::save_database(&db, out, Engine::R2K, SaveOpt::default(), b""));

    let loaded = rm2k_wasm::load_database(&bytes).unwrap();
    let name = get(&get(&loaded.value, "system"), "system_name");
    assert_eq!(name.as_string().as_deref(), Some("\\x82\\xa0"));

    let round_tripped = rm2k_wasm::save_database(loaded.value, rm2k_wasm::Engine::R2K, false, &[]).unwrap();
    assert_eq!(round_tripped, bytes);
}

#[wasm_bindgen_test]
fn a_preserved_header_comes_back_from_the_browser_path() {
    let db = Map::defaults(Engine::R2K);
    let bytes = file::to_vec(|out| file::save_map(&db, out, Engine::R2K, SaveOpt { preserve_header: true }, b"Custom"));

    let loaded = rm2k_wasm::load_map(&bytes).unwrap();
    assert_eq!(loaded.header, b"Custom");
    let round_tripped = rm2k_wasm::save_map(loaded.value, rm2k_wasm::Engine::R2K, true, &loaded.header).unwrap();
    assert_eq!(round_tripped, bytes);
}

#[wasm_bindgen_test]
fn engine_from_ldb_id_classifies_the_raw_value() {
    assert!(rm2k_wasm::engine_from_ldb_id(2003) == rm2k_wasm::Engine::R2K3);
    assert!(rm2k_wasm::engine_from_ldb_id(2000) == rm2k_wasm::Engine::R2K);
}

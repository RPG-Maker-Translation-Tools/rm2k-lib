use rm2k::{
    Engine, LcfStruct, SaveOpt, file,
    rpg::{Database, Map, Save, TreeMap},
};
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

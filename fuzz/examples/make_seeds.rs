//! Writes the fuzz corpora from files the library builds itself, so no game files are needed. Run from `fuzz/`:
//! `cargo run --locked --example make_seeds`
#[path = "../../tests/common/samples.rs"]
mod samples;

use rm2k::{
    Engine, LcfStruct, SaveOpt, file,
    rpg::{Database, Map, Save, TreeMap},
};
use std::fs;

const TARGETS: [&str; 7] = ["ldb", "lmu", "lsd", "lmt", "events", "capi", "owned"];

fn write(targets: &[&str], name: &str, bytes: &[u8]) {
    for target in targets {
        fs::write(format!("corpus/{target}/{name}"), bytes).unwrap();
    }
}

fn main() {
    for target in TARGETS {
        fs::create_dir_all(format!("corpus/{target}")).unwrap();
    }

    let database_r2k = file::to_vec(|out| {
        file::save_database(
            &Database::defaults(Engine::R2K),
            out,
            Engine::R2K,
            SaveOpt::default(),
            b"",
        )
    });
    let database_r2k3 = file::to_vec(|out| {
        file::save_database(
            &Database::defaults(Engine::R2K3),
            out,
            Engine::R2K3,
            SaveOpt::default(),
            b"",
        )
    });
    let map =
        file::to_vec(|out| file::save_map(&Map::defaults(Engine::R2K), out, Engine::R2K, SaveOpt::default(), b""));
    let save =
        file::to_vec(|out| file::save_save(&Save::defaults(Engine::R2K), out, Engine::R2K, SaveOpt::default(), b""));
    let tree_map = file::to_vec(|out| {
        file::save_tree_map(
            &TreeMap::defaults(Engine::R2K),
            out,
            Engine::R2K,
            SaveOpt::default(),
            b"",
        )
    });
    let events = samples::map_with_events();

    // `capi` and `owned` load every format, so they get all of them.
    write(&["ldb", "capi", "owned"], "defaults-r2k", &database_r2k);
    write(&["ldb", "capi", "owned"], "defaults-r2k3", &database_r2k3);
    write(&["lmu", "capi", "owned"], "defaults", &map);
    write(&["lmu", "capi", "owned"], "events", &events);
    write(&["lsd", "capi", "owned"], "defaults", &save);
    write(&["lmt", "capi", "owned"], "defaults", &tree_map);
    // `events` takes raw chunk payload. The map's body is the stream its walkers see: the file minus
    // its header, a length byte and the 10-byte "LcfMapUnit".
    write(&["events"], "map-body", &events[11..]);
}

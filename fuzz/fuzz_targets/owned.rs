#![no_main]
//! Detaching a loaded file from its buffer must not change what it saves to: the owned copy must
//! save the same bytes as the borrowed value it came from.
use libfuzzer_sys::fuzz_target;
use rm2k::{Engine, SaveOpt, file, owned::IntoOwned};

/// Saves the borrowed value, detaches it, saves the owned copy, and compares.
macro_rules! detach_is_exact {
    ($load:path, $save:path, $name:literal, $data:expr) => {
        if let Ok(loaded) = $load($data) {
            let header = loaded.header;
            let opt = SaveOpt { preserve_header: true };
            let borrowed = file::to_vec(|out| $save(&loaded.value, out, Engine::R2K, opt, header));
            let owned = loaded.value.into_owned();
            let detached = file::to_vec(|out| $save(&owned, out, Engine::R2K, opt, header));
            assert!(borrowed == detached, "{}: detaching changed the save", $name);
        }
    };
}

fuzz_target!(|data: &[u8]| {
    detach_is_exact!(file::load_database, file::save_database, "database", data);
    detach_is_exact!(file::load_map, file::save_map, "map", data);
    detach_is_exact!(file::load_tree_map, file::save_tree_map, "tree map", data);
    detach_is_exact!(file::load_save, file::save_save, "save", data);
});

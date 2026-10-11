#![no_main]
use libfuzzer_sys::fuzz_target;
use rm2k::{file, Engine, SaveOpt};

/// Whatever loads must save, and saving the reloaded value must give the same bytes again.
fuzz_target!(|data: &[u8]| {
    let Ok(loaded) = file::load_database(data) else {
        return;
    };

    for engine in [Engine::R2K, Engine::R2K3] {
        let mut first = Vec::new();
        let _ = file::save_database(&loaded.value, &mut first, engine, SaveOpt { preserve_header: true }, loaded.header);

        let reloaded = file::load_database(&first).expect("own output must load");
        let mut second = Vec::new();
        let _ = file::save_database(&reloaded.value, &mut second, engine, SaveOpt { preserve_header: true }, reloaded.header);
        assert!(first == second, "{engine:?}: save is not stable");
    }
});

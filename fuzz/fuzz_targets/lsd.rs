#![no_main]
use libfuzzer_sys::fuzz_target;
use rm2k::{diag::DiagKind, file, Engine, SaveOpt};

/// Every input must load and save without crashing. Stability (saving the reloaded value must give
/// the same bytes again) is only checked when the load was clean: recovered input is corrupt, and
/// its output is not required to be stable. Stability belongs in the tests.
fuzz_target!(|data: &[u8]| {
    let mut diag = Vec::new();
    let Ok(loaded) = file::load_save_with(data, &mut diag) else {
        return;
    };
    let recovered = diag.iter().any(|d| !matches!(d.kind, DiagKind::UnknownChunk));

    for engine in [Engine::R2K, Engine::R2K3] {
        let mut first = Vec::new();
        let _ = file::save_save(&loaded.value, &mut first, engine, SaveOpt { preserve_header: true }, loaded.header);
        if recovered {
            continue;
        }

        let reloaded = file::load_save(&first).expect("own output must load");
        let mut second = Vec::new();
        let _ = file::save_save(&reloaded.value, &mut second, engine, SaveOpt { preserve_header: true }, reloaded.header);
        assert!(first == second, "{engine:?}: save is not stable");
    }
});

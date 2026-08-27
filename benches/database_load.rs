use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use rm2k::file;
use std::{hint::black_box, path::PathBuf};

fn game_dir() -> PathBuf {
    let dir = std::env::var_os("RM2K_BENCH_GAME_DIR")
        .unwrap_or_else(|| panic!("RM2K_BENCH_GAME_DIR must be set to a game directory"));
    PathBuf::from(dir)
}

fn bench_database(c: &mut Criterion) {
    let dir = game_dir();
    let ldb_path = dir.join("RPG_RT.ldb");
    let bytes = std::fs::read(&ldb_path).unwrap_or_else(|err| {
        panic!(
            "could not read {}: {err} (set RM2K_BENCH_GAME_DIR to a game directory)",
            ldb_path.display()
        )
    });

    // Matches benches/liblcf_cmp/bench_load.cpp's "ldb" mode: bytes are read into memory once,
    // outside the timed region, then only Load() itself is measured.
    c.bench_function("ldb (single database)", |b| {
        b.iter(|| file::load_database(black_box(&bytes)).unwrap());
    });
}

fn bench_maps(c: &mut Criterion) {
    let dir = game_dir();
    let map_bytes: Vec<Vec<u8>> = std::fs::read_dir(&dir)
        .unwrap_or_else(|err| panic!("could not read {}: {err}", dir.display()))
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .map(str::to_ascii_lowercase)
                .as_deref()
                == Some("lmu")
        })
        .map(|p| std::fs::read(&p).unwrap())
        .collect();
    assert!(!map_bytes.is_empty(), "no .lmu files found in {}", dir.display());

    let mut group = c.benchmark_group("maps");
    // Reports a derived per-file time alongside the per-pass time, directly comparable to the
    // C++ harness's "maps (per file)" line.
    group.throughput(Throughput::Elements(map_bytes.len() as u64));
    group.bench_function("one full pass", |b| {
        b.iter(|| {
            for bytes in &map_bytes {
                let _ = file::load_map(black_box(bytes)).unwrap();
            }
        });
    });
    group.finish();
}

fn config() -> Criterion {
    Criterion::default().sample_size(60)
}

criterion_group! {
    name = benches;
    config = config();
    targets = bench_database, bench_maps
}
criterion_main!(benches);

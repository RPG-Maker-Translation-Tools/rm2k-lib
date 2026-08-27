use rm2k::{
    Engine, SaveOpt,
    diag::{DiagKind, Diagnostic},
    file,
};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

#[derive(Default)]
struct Tally {
    exact: usize,
    differs: usize,
    failed: usize,
    diagnostics: BTreeMap<&'static str, usize>,
}

/// Outcome of one file.
enum Outcome {
    Exact(Engine),
    Differs {
        engine: Engine,
        at: usize,
        original: usize,
        produced: usize,
    },
    Failed(String),
}

fn first_difference(a: &[u8], b: &[u8]) -> usize {
    a.iter()
        .zip(b)
        .position(|(x, y)| x != y)
        .unwrap_or_else(|| a.len().min(b.len()))
}

/// Round-trip one file under a given engine, returning the re-serialised bytes.
///
/// `LDB` recovers its engine from `system.ldb_id`; the other three formats carry no such marker,
/// so the caller tries both and keeps whichever reproduces the input.
fn round_trip(path: &Path, bytes: &[u8], engine: Engine, diag: &mut Vec<Diagnostic>) -> Option<Vec<u8>> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    let opt = SaveOpt { preserve_header: true };
    let mut out = Vec::with_capacity(bytes.len());

    match ext.as_str() {
        "ldb" => {
            let loaded = file::load_database_with(bytes, diag).ok()?;
            file::save_database(&loaded.value, &mut out, engine, opt, loaded.header).ok()?;
        }
        "lmu" => {
            let loaded = file::load_map_with(bytes, diag).ok()?;
            file::save_map(&loaded.value, &mut out, engine, opt, loaded.header).ok()?;
        }
        "lmt" => {
            let loaded = file::load_tree_map_with(bytes, diag).ok()?;
            file::save_tree_map(&loaded.value, &mut out, engine, opt, loaded.header).ok()?;
        }
        "lsd" => {
            let loaded = file::load_save_with(bytes, diag).ok()?;
            file::save_save(&loaded.value, &mut out, engine, opt, loaded.header).ok()?;
        }
        _ => return None,
    }
    Some(out)
}

/// The engine to try first: an LDB says which it is, everything else has to be guessed.
fn preferred_engine(path: &Path, bytes: &[u8]) -> Engine {
    if path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
        == Some("ldb")
    {
        if let Ok(loaded) = file::load_database(bytes) {
            return Engine::from_ldb_id(loaded.value.system.ldb_id);
        }
    }
    Engine::R2K
}

fn check(path: &Path, tally: &mut Tally) -> Outcome {
    let Ok(bytes) = std::fs::read(path) else {
        return Outcome::Failed("could not read".into());
    };

    let first = preferred_engine(path, &bytes);
    let second = if first == Engine::R2K {
        Engine::R2K3
    } else {
        Engine::R2K
    };

    let mut best: Option<(Engine, Vec<u8>)> = None;
    for engine in [first, second] {
        let mut diag = Vec::new();
        let Some(out) = round_trip(path, &bytes, engine, &mut diag) else {
            return Outcome::Failed("parse failed".into());
        };
        for d in &diag {
            *tally.diagnostics.entry(kind_label(d.kind)).or_default() += 1;
        }
        if out == bytes {
            return Outcome::Exact(engine);
        }
        if best.is_none() {
            best = Some((engine, out));
        }
    }

    let (engine, out) = best.expect("at least one attempt ran");
    Outcome::Differs {
        engine,
        at: first_difference(&bytes, &out),
        original: bytes.len(),
        produced: out.len(),
    }
}

fn kind_label(kind: DiagKind) -> &'static str {
    kind.as_str()
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, out);
        } else if matches!(
            path.extension()
                .and_then(|e| e.to_str())
                .map(str::to_ascii_lowercase)
                .as_deref(),
            Some("ldb" | "lmu" | "lmt" | "lsd")
        ) {
            out.push(path);
        }
    }
}

fn main() {
    let Some(root) = std::env::args().nth(1) else {
        eprintln!("usage: fixture_check <directory of RPG Maker games>");
        eprintln!();
        eprintln!("Loads every .ldb/.lmu/.lmt/.lsd found beneath the directory, re-serialises it");
        eprintln!("in memory, and reports whether the bytes match. Nothing is written to disk.");
        std::process::exit(2);
    };

    let mut files = Vec::new();
    collect(Path::new(&root), &mut files);
    files.sort();

    if files.is_empty() {
        eprintln!("no LCF files found beneath {root}");
        std::process::exit(1);
    }

    println!("checking {} files under {root}\n", files.len());

    let mut tally = Tally::default();
    for path in &files {
        let display = path.strip_prefix(&root).unwrap_or(path).display();
        match check(path, &mut tally) {
            Outcome::Exact(engine) => {
                tally.exact += 1;
                if std::env::var_os("VERBOSE").is_some() {
                    println!("EXACT   {display}  ({engine:?})");
                }
            }
            Outcome::Differs {
                engine,
                at,
                original,
                produced,
            } => {
                tally.differs += 1;
                println!(
                    "DIFFERS {display}  ({engine:?}) first difference at 0x{at:x}; {original} bytes in, {produced} out"
                );
            }
            Outcome::Failed(why) => {
                tally.failed += 1;
                println!("FAILED  {display}  {why}");
            }
        }
    }

    println!(
        "\n{} exact, {} differ, {} failed, of {} files",
        tally.exact,
        tally.differs,
        tally.failed,
        files.len()
    );

    if !tally.diagnostics.is_empty() {
        println!("\nrecovered problems (these are expected in real files):");
        for (kind, count) in &tally.diagnostics {
            println!("  {count:6}  {kind}");
        }
    }

    if tally.differs > 0 || tally.failed > 0 {
        std::process::exit(1);
    }
}

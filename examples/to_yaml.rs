use rm2k::file;
use std::path::PathBuf;

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(input) = args.next() else {
        eprintln!("usage: to_yaml <database.ldb> [out.yaml]");
        std::process::exit(2);
    };
    let output = args.next().map(PathBuf::from);

    let bytes = std::fs::read(&input).unwrap_or_else(|err| {
        eprintln!("could not read {input}: {err}");
        std::process::exit(1);
    });

    let loaded = file::load_database(&bytes).unwrap_or_else(|err| {
        eprintln!("could not parse {input} as an LDB database: {err:?}");
        std::process::exit(1);
    });

    let yaml = yaml_serde::to_string(&loaded.value).expect("Database always serialises");

    match output {
        Some(path) => std::fs::write(&path, &yaml).unwrap_or_else(|err| {
            eprintln!("could not write {}: {err}", path.display());
            std::process::exit(1);
        }),
        None => print!("{yaml}"),
    }
}

# Examples

Run any of these with `cargo run --release --example <name> [--features ...] -- <args>`.

## `fixture_check.rs`

Byte-exact round-trip check against real RPG Maker games: loads every `.ldb`/`.lmu`/`.lmt`/`.lsd` under a directory (recursively), writes it straight back out, and compares bytes. Nothing is written to disk.

```bash
cargo run --release --example fixture_check -- "/path/to/games"
```

Each file is reported as `EXACT`, `DIFFERS` (parsed, but the output differs; the first differing offset is shown) or `FAILED` (could not be parsed). Only `DIFFERS`/`FAILED` lines and the summary print unless `VERBOSE=1`. Diagnostics recovered while parsing (see [Leniency](../README.md#leniency)) are tallied after the summary; they are expected on real files. The exit code is `1` if anything differed or failed, so it works as a CI check.

## `dump_text.rs`

Prints every message string from a game using only the allocation-free core tier: `CommandLists` and `EventCommandIter` hand back spans into the file buffer. Strings print as escaped raw bytes, since LCF records no encoding (see [Strings](../README.md#strings)).

```bash
cargo run --release --example dump_text -- "/path/to/game"
```

## `to_yaml.rs`

Converts a database to YAML with the `serde` tier. Without an output path, YAML goes to stdout. Non-UTF-8 text is written as reversible `\xNN` escapes, so reading the YAML back and saving it with `rm2k::file::save_database` reproduces the original bytes.

```bash
cargo run --release --example to_yaml --features serde -- game.ldb [out.yaml]
```

# Examples

Run any of these with `cargo run --release --example <name> [--features ...] -- <args>`.

## `fixture_check.rs`

Byte-exact round-trip check against real RPG Maker games.

```bash
cargo run --release --example fixture_check -- "/path/to/games"
```

The unit and integration tests only ever see files this crate wrote itself, which cannot catch a field whose default is wrong in the same way on both sides, or a chunk RPG Maker emits that the schema does not know about. This does: point it at a directory of real games and it loads every `.ldb`/`.lmu`/`.lmt`/`.lsd` it finds anywhere beneath the directory (it recurses into subdirectories), writes it straight back out, and compares bytes. Nothing is written to disk. A file is reported as:

- `EXACT` - re-serialising reproduced the input byte for byte.
- `DIFFERS` - it parsed, but the output differs; the first differing offset is shown.
- `FAILED` - it could not be parsed at all.

`DIFFERS` is the interesting one. Every difference is either a real bug or a documented divergence from `RPG_RT`, and either way it should be explainable before this crate is trusted with anyone's project files.

By default only `DIFFERS`/`FAILED` lines and the final summary print; set `VERBOSE=1` to also print an `EXACT` line per file. Diagnostics recovered while parsing (see [Leniency](../README.md#leniency)) are tallied by kind and printed after the summary - these are expected on real files, not failures. The process exits with code `1` if anything differed or failed, `0` otherwise, so it can be used as a CI check.

## `dump_text.rs`

Extract every message string from a game, using only the allocation-free core tier.

```bash
cargo run --release --example dump_text -- "/path/to/game"
```

A demonstration of the point of the core tier: the traversal allocates nothing at all. `CommandLists` walks the chunk stream and `EventCommandIter` decodes commands, both handing back spans into the file buffer. The only allocation is the buffer itself and whatever `println!` does.

**Encoding**: strings print as raw bytes, escaped. That is not a limitation being worked around - it is the honest answer, because an LCF file records no encoding. A Japanese game's text is Shift-JIS, a western one's is CP1252, and nothing in the format says which. Decode with `encoding_rs` once you know, or read it from `RPG_RT.ini`'s `[EasyRPG] Encoding` key.

## `to_yaml.rs`

Convert an LCF database to YAML, using the `serde` tier.

```bash
cargo run --release --example to_yaml --features serde -- game.ldb [out.yaml]
```

With no output path, YAML goes to stdout. `DbStr` fields are not UTF-8 text, they are bytes in whatever codepage the game used (see the crate's top-level docs). The `serde` tier represents them with a reversible escape: printable ASCII passes through and every other byte becomes `\xNN`, so the YAML is readable for the common case and lossless for the rest - round-tripping it back through `yaml_serde` and `rm2k::file::save_database` reproduces the original bytes exactly.

## `crates/rm2k-capi/examples/to_json.c`

The C API's example - see [`crates/rm2k-capi/examples/README.md`](../crates/rm2k-capi/examples/README.md).

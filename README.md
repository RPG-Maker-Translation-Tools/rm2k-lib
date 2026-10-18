# rm2k-lib

Reads and writes the binary formats RPG Maker 2000 and 2003 use: `LDB` (database), `LMU` (map), `LMT` (map tree) and `LSD` (save). A from-scratch Rust rewrite of [liblcf](https://github.com/EasyRPG/liblcf), built around three goals: run free-standing under `no_std`, be perfectly round-trip, be **BLAZINGLY 🦀 FAST 🔥🔥**

Round-trip correctness is verified byte-for-byte against real RPG Maker games - see [Verification](#verification) below.

Ports:

- [C API](./capi/) - C bindings, installable via `cargo-c`.
- [WASM](./wasm/) - WebAssembly bindings.

## Installation

```bash
cargo add rm2k-lib
```

## Feature tiers

| Feature           | Pulls in         | Gives you                                                                                                      |
| ----------------- | ---------------- | -------------------------------------------------------------------------------------------------------------- |
| _(none)_          | nothing          | The wire codec, chunk framing, and allocation-free event iteration. Runs on `no_std` with no allocator at all. |
| `alloc`           | `alloc`          | Typed structs - `Database`, `Map`, `TreeMap`, `Save` - and loading/saving them.                                |
| `std` _(default)_ | `alloc`, `std`   | [`IoSink`](https://docs.rs/rm2k-lib/latest/rm2k/sink/struct.IoSink.html) over `std::io::Write`.                |
| `serde`           | `alloc`, `serde` | `Serialize`/`Deserialize` on every schema struct, enum and flag bundle. See [Serde](#serde).                                               |

## Quick example

```rust,no_run
use rm2k::file;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bytes = std::fs::read("RPG_RT.ldb")?;
    let loaded = file::load_database(&bytes)?;

    for actor in loaded.value.actors.iter() {
        println!("{}", actor.name.as_utf8().unwrap_or("<non-UTF-8 name>"));
    }
    Ok(())
}
```

## Strings

**This crate never transcodes text, and never guesses an encoding.** RPG Maker 2k/3 stores strings in whatever legacy codepage the authoring machine used - Shift-JIS for Japanese games, CP1252 for western ones - and an LCF file contains no marker recording which. So strings come out as `&[u8]`, exactly as they sit in the file. Decode with whatever you prefer once you know the codepage, typically from `RPG_RT.ini`'s `[EasyRPG] Encoding` key.

## Leniency

Real files are frequently malformed and RPG Maker itself loads them anyway, so this crate does too. The difference is that recoveries are _reported_: pass a `DiagSink` and you get a structured list of what was wrong and where. Pass `StrictSink` to turn the first problem into an error.

**Chunks this schema does not know are kept, not dropped.** RPG Maker's forward-compatibility story is "write chunks the reader may not understand", so real files carry ids from later `RPG_RT` builds, EasyRPG extensions and third-party editors. Each one is stored verbatim in its struct's `unknown_chunks`, anchored next to the last field that _was_ recognised, and written back in that position - so byte-exactness does not depend on the schema being complete. They are still reported as `DiagKind::UnknownChunk`; preserved is not understood.

## Serde

With the `serde` feature, the schema's structs, enums and flag bundles derive `Serialize` and `Deserialize`, so any serde format can read and write a loaded file. The [`rm2k-wasm`](./wasm/) bindings and the [`to_yaml`](examples/to_yaml.rs) example both use it. Each struct becomes an object keyed by its schema field names (`system.system_name`), each list becomes an array, and each string becomes escaped text: printable ASCII stays as-is, `\` becomes `\\`, and every other byte becomes `\xNN`. Human-readable formats (JSON, YAML, WASM's JS values) get that text; binary formats get the raw bytes. [`src/serde/mod.rs`](src/serde/mod.rs) has the full rules.

## Examples

| Example                                                  | What it shows                                                                                                                                                                  |
| -------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| [`examples/fixture_check.rs`](examples/fixture_check.rs) | Byte-exact round-trip checking against real game files on disk.                                                                                                                |
| [`examples/dump_text.rs`](examples/dump_text.rs)         | Listing the message and choice strings in each map's event commands. The event walk borrows from the file buffer, but each map is still loaded to read its header.                                                                                                    |
| [`examples/to_yaml.rs`](examples/to_yaml.rs)             | Converting a database to YAML with the `serde` tier (`cargo run --example to_yaml --features serde -- game.ldb`).                                                              |
| [`capi/examples/to_json.c`](capi/examples/to_json.c)     | Converting a database's named lists to JSON from C (via [yyjson](https://github.com/ibireme/yyjson), vendored next to the example), using only the C API's owned/"alloc" tier. |

Each example has its own README with usage, output semantics, and (for the C ones) build steps: [`examples/README.md`](examples/README.md) for the Rust examples, [`capi/examples/README.md`](capi/examples/README.md) for `to_json.c`. Run a Rust example with `cargo run --release --example <name> [--features ...] -- <args>`; the C example is not wired into Cargo.

## Verification

[`examples/fixture_check.rs`](examples/fixture_check.rs) ([usage](examples/README.md)) loads every `.ldb`/`.lmu`/`.lmt`/`.lsd` file in a directory, writes it straight back out, and compares bytes. Run against three real, unmodified RPG Maker games (RPG Maker 2000 and 2003, 419 files across all four formats): **419/419 byte-exact, zero diagnostics**. The C API gets the same treatment independently in [`capi/tests/test_c_api.c`](capi/tests/test_c_api.c) ([build steps](capi/tests/README.md)), round-tripping a real 262 KB database through the FFI boundary byte-for-byte.

The games in question:

- [OFF The Game, English Localization v3.0](http://www.mediafire.com/file/sx1kyo89yu0dko1/OFF+English+3.0.zip)
- [Red Tide](https://rpgmaker.net/games/12992/)
- [Metamorphosis](https://rpgmaker.net/games/12270/)

[`benches/`](https://github.com/RPG-Maker-Translation-Tools/rm2k-lib/tree/main/benches) has a real, measured comparison against liblcf: parsing `RPG_RT.ldb` is **~6.2x faster**, and parsing a `.lmu` map file is **~8.8x faster** (AMD Ryzen 5 2600, in-memory parse only, see [`benches/README.md`](https://github.com/RPG-Maker-Translation-Tools/rm2k-lib/blob/main/benches/README.md) for the full methodology, build steps, and raw numbers). Speed was not the goal, though.

## Design notes

- **Zero-copy core tier.** Parsing borrows slices out of the input buffer; nothing is copied unless you ask for it (e.g. via [`IntoOwned::into_owned()`](https://docs.rs/rm2k-lib/latest/rm2k/owned/trait.IntoOwned.html#tymethod.into_owned)).
- **Cleaner codebase.** No legacy C++ shit. Boilerplate enums are generated by declarative `macro_rules!` macros, not some external Python pipelines.
- **`Setup` is opt-in, not automatic.** `RPG_RT`'s post-load fixups (resolving `-1` sentinels, padding stat curves) change the byte representation, so running them automatically on load would make a load/save cycle non-idempotent. Call [`Setup::setup`](https://docs.rs/rm2k-lib/latest/rm2k/trait.Setup.html#tymethod.setup) explicitly when you want them.
- **Diagnostics instead of a global error string.** Recoverable problems are reported through a `DiagSink` you provide, not a process-global mutable buffer.
- **Round-trip fidelity does not depend on schema completeness.** Every struct keeps the chunks no field claimed and re-emits them where they were found, so a file using ids this schema has never seen still round-trips byte-for-byte. See [Leniency](#leniency).

## Acknowledgements

Most of the RPG Maker 2k/3 format handling was written referencing [liblcf](https://github.com/EasyRPG/liblcf), a part of the EasyRPG project, a [MIT-licensed](./COPYING-liblcf) library. However, the implementation that handles the format itself is very different from `liblcf`, both because Rust enforces different idioms and because the logic was made with free-standing usage in mind. This work wouldn't be possible without them.

[yyjson](https://github.com/ibireme/yyjson) (`yyjson.h`/`yyjson.c`, MIT licensed) is vendored in [`capi/examples/yyjson`](capi/examples) for the `to_json.c` example, its license is alongside it.

## Support

[Me](https://github.com/savannstm), the maintainer of this project, is a poor college student from Eastern Europe.

If you could, please consider supporting us through:

- [Boosty](https://boosty.to/mcdeimos) (has lower fees)
- [Ko-fi](https://ko-fi.com/savannstm)

Even if you don't, we will still continue working on the project anyways.

## License

Licensed under [WTFPL](https://www.wtfpl.net/).

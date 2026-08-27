# C API examples

Build the library first, then compile the example against it by hand.

## `to_json.c`

Dumps a database's named lists (actors, skills, items, ...) as JSON, using only the C API's owned/"alloc" tier - `Rm2kDatabase` and the per-list accessors, not the zero-copy walkers.

This is deliberately not a full-fidelity dump of every field of every struct: the C API does not expose one accessor per field (see the comment on `rm2k_database_list_get()` in `rm2k.h` - that would be roughly 1200 functions for ~69 structs). What it exposes, and what this example demonstrates, is what most C tools actually want: "what are the actors called and what are their ids", for each of the twelve named lists a database carries.

JSON is built with [yyjson](https://github.com/ibireme/yyjson) (vendored here as `yyjson/yyjson.c`/`yyjson/yyjson.h`, MIT licensed - see `yyjson/LICENSE`).

**Encoding**: a name's bytes are exactly the bytes the file held - see the note on strings in `rm2k.h`. This writer does not know or guess the game's codepage, so rather than pretending the bytes are UTF-8 (they usually are not), each byte is treated as a Latin-1 code point and re-encoded to UTF-8 one-to-one: `0x00`-`0x7F` pass through as themselves, `0x80`-`0xFF` become the two-byte UTF-8 sequence for U+0080-U+00FF. That is lossless and yyjson-writes as ordinary valid JSON; a consumer who knows the actual codepage (CP1252, Shift-JIS, ...) recovers the original byte from the low 8 bits of each code point and decodes from there.

Built against a genuinely `no_std` `rm2k-capi` (`--no-default-features`), registering `malloc`/`free`-backed hooks via `rm2k_set_allocator` before making any other allocating call - see the top-level README's [C API](../../../README.md#c-api) section for the allocator-hook contract.

```bash
cd crates/rm2k-capi
cargo cinstall --release --no-default-features --prefix="$PWD/target/capi-install"
```

**Linux/macOS:**

```bash
clang -std=c11 -I assets \
    examples/to_json.c \
    examples/yyjson/yyjson.c \
    -o to_json target/release/librm2k.a -lpthread -ldl -lm
./to_json "/path/to/game/RPG_RT.ldb" out.json
```

**Windows:**

```bash
clang -std=c11 -I assets \
    examples/to_json.c \
    examples/yyjson/yyjson.c \
    -o to_json.exe target/release/rm2k.lib -lws2_32 -luserenv -lntdll -lbcrypt
./to_json.exe "C:/path/to/game/RPG_RT.ldb" out.json
```

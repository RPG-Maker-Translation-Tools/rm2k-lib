# C API examples

## `to_json.c`

Dumps a database's named lists (actors, skills, items, ...) as JSON with [yyjson](https://github.com/ibireme/yyjson) (vendored in `yyjson/`, MIT licensed), using the owned-handle layer only (`Rm2kDatabase` and the list accessors). It is not a dump of every field: the C API has no accessor per field, only the id/name pairs of the twelve named lists.

Names are raw bytes in the game's code page, so each byte is written as the Latin-1 code point of the same value; a consumer who knows the code page recovers the original bytes from the low 8 bits of each code point.

It uses a `no_std` build and registers `malloc`/`free`-backed hooks with `rm2k_set_allocator` first (see [`no_std`](../README.md#no_std) in the C API README).

```bash
cd capi
cargo cinstall --release --no-default-features --prefix="$PWD/target/capi-install"
```

```bash
# Linux/macOS
clang -std=c11 -I target/capi-install/include examples/to_json.c examples/yyjson/yyjson.c \
    -o to_json target/capi-install/lib/librm2k.a -lpthread -ldl -lm
./to_json "/path/to/game/RPG_RT.ldb" out.json

# Windows
clang -std=c11 -I target/capi-install/include examples/to_json.c examples/yyjson/yyjson.c \
    -o to_json.exe target/capi-install/lib/rm2k.lib -lws2_32 -luserenv -lntdll -lbcrypt
./to_json.exe "C:/path/to/game/RPG_RT.ldb" out.json
```

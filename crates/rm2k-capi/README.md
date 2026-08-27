# rm2k-capi

C API for [`rm2k-lib`](../..). Produces a shared/static library (`librm2k`), a C header (`rm2k.h`), and a `pkg-config` file. Two layers, mirroring the Rust crate's tiers:

- **Owned handles** - `rm2k_database_load` and friends parse a whole file into an opaque handle you free later. A borrowed `Database<'a>` cannot cross FFI, so loading detaches with `IntoOwned` immediately. This is the layer most consumers want.
- **Zero-copy walkers** - `Rm2kChunkIter`, `Rm2kListWalker`, `Rm2kCommandLists` and `Rm2kEventCommands` are plain structs working on the buffers you provided. They allocate nothing at all, and with a `NULL` diagnostics argument they touch no allocator on any path. This is how a C consumer gets at the format without a handle.

## Building / installing

```bash
cd crates/rm2k-capi
cargo install cargo-c # or cargo binstall cargo-c
cargo cbuild --release
cargo cinstall --release --prefix=/usr/local
```

## API reference

**Errors and engine** - `Rm2kError` mirrors a payload-free subset of `rm2k::ReadError` (unexpected EOF, bad magic, varint overflow, chunk overrun, strict-mode failure, a `NULL` required argument). `Rm2kEngine` (`R2K`/`R2K3`) selects which RPG Maker a save targets.

**Diagnostics** - `Rm2kDiagnostics` is an optional collector for recoverable parse problems (`rm2k_diagnostics_new`/`_free`/`_count`/`_get`/`_clear`); pass `NULL` anywhere a `Rm2kDiagnostics*` is expected to ignore them, which is also the only fully allocation-free option. `rm2k_diagnostic_kind_name` maps a `Rm2kDiagnostic.kind` byte to a fixed, NUL-terminated English label - the one place this API hands back a `char*`, since these are labels this crate owns, not file data.

**Owned handles** - `rm2k_database_load`/`rm2k_map_load`/`rm2k_tree_map_load`/`rm2k_save_load` each parse a whole file into a `Rm2kDatabase`/`Rm2kMap`/`Rm2kTreeMap`/`Rm2kSave` handle, returning `NULL` and writing `*out_error` on failure. The matching `rm2k_*_save` serializes back to a fresh buffer (release with `rm2k_buffer_free`), `rm2k_*_free` releases the handle, and `rm2k_*_header` returns the file's magic string exactly as read. `rm2k_database_engine` reads which RPG Maker a loaded database declares (`system.ldb_id`). `rm2k_database_list_len`/`rm2k_database_list_get` enumerate one of the twelve `Rm2kDbList` lists (actors, skills, items, ...) by id/name pair (`Rm2kNamed`) - a typed accessor per struct field would be roughly 1 200 functions; this covers what a tool actually asks for. Anything else is reachable through the zero-copy walkers below.

**Zero-copy walkers** - plain, fixed-size structs living in caller memory, each initialised in place by a `_init` function and advanced by a `_next` function; they borrow the buffer they were initialised from, which must outlive them, and allocate nothing.

- `rm2k_split_header` skips a whole file's length-prefixed magic, yielding the root chunk stream (`Rm2kBytes` body + header) that every walker below expects as input.
- `Rm2kChunkIter` (`rm2k_chunk_iter_init`/`_next`) walks a raw chunk stream; `rm2k_chunk_find` looks up one chunk by id directly.
- `Rm2kListWalker` (`rm2k_list_walker_init`/`_next`) walks an indexed list, yielding `Rm2kListItem` (id, offset, chunk-stream body).
- `Rm2kCommandLists` (`rm2k_command_lists_init`/`_next`) finds every event-command list under a file root - a map's events, a database's common events, or a database's troop pages, selected by `Rm2kEventScope`. `data` here is a _body_, i.e. already past the file's magic (see `rm2k_split_header`).
- `Rm2kEventCommands` (`rm2k_event_commands_init`/`_next`) walks one command list, yielding `Rm2kEventCommand` (code, indent, string operand, parameter count); `rm2k_event_command_param` decodes one integer parameter by index (`O(index)`, since compressed integers are not fixed width).
- The `chunk_id` module exposes the eight chunk ids `rm2k::event::path` also has (`RM2K_CHUNK_MAP_EVENTS`, `RM2K_CHUNK_EVENT_PAGES`, ...), for a consumer that wants to navigate by hand instead of through `rm2k_command_lists_init`.

## Strings

Byte pointers, never `char*`, and never NUL-terminated. LCF records no encoding anywhere, so there is nothing sensible to convert _to_; see the top-level [README](../../README.md#strings). The bytes are exactly what the file held.

## `no_std`

Disable this crate's default `std` feature for a genuinely `no_std` build:

```bash
cargo cbuild --release --no-default-features
```

- A minimal abort-on-panic handler.
- A global allocator backed by two function-pointer hooks the host registers once via `rm2k_set_allocator`, **before any other allocating `rm2k_*` call** - never a hardcoded `malloc`/`free`, since a freestanding target may not have libc at all:

  ```c
  typedef uint8_t* (*Rm2kAllocFn)(size_t size, size_t align);
  typedef void (*Rm2kFreeFn)(uint8_t* ptr, size_t size, size_t align);
  void rm2k_set_allocator(Rm2kAllocFn alloc, Rm2kFreeFn free);
  ```

  `alloc` returns a pointer to at least `size` bytes aligned to `align` (a power of two), or `NULL` on failure; `free` gets back exactly the `(ptr, size, align)` triple a prior `alloc` call produced. Skipping this call (or calling any other allocating function first) makes every allocation fail, surfacing as `NULL`/`false` returns as if out of memory - it does not crash. `rm2k_set_allocator` only exists in a `no_std` build; the default `std`-linked build uses Rust's own allocator and does not declare or need it.

The zero-copy walkers (`Rm2kChunkIter`, `Rm2kListWalker`, `Rm2kCommandLists`, `Rm2kEventCommands`) never touch the allocator regardless of tier, so a `no_std` consumer that only needs those has nothing to register at all. See [`examples/README.md`](examples/README.md) for `to_json.c`, which exercises the owned-handle tier against a `no_std` build and registers `malloc`/`free`-backed hooks.

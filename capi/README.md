# rm2k-capi

C API for rm2k-lib, which reads and writes the RPG Maker 2000/2003 LCF formats.

## Install

```bash
cargo install cargo-c
cargo cinstall --release --prefix=/usr/local
```

Installs `librm2k` (shared and static), `rm2k.h` and a `pkg-config` file. The header documents every function.

## Overview

Two layers:

- **Owned handles.** `rm2k_database_load`, `rm2k_map_load`, `rm2k_tree_map_load` and `rm2k_save_load` parse a whole file into a `Rm2kDatabase` / `Rm2kMap` / `Rm2kTreeMap` / `Rm2kSave`, returning `NULL` and writing `*out_error` on failure. `rm2k_*_save` serializes back to a fresh buffer (release with `rm2k_buffer_free`), `rm2k_*_free` releases the handle, and `rm2k_*_header` returns the file's magic string as read. `rm2k_database_list_len` / `rm2k_database_list_get` enumerate one of the `Rm2kDbList` lists (actors, skills, items, ...) as id/name pairs. `rm2k_engine_from_ldb_id` classifies a raw `ldb_id` the same way `rm2k_database_engine` does. This is the layer most consumers want.
- **Zero-copy walkers.** `Rm2kChunkIter`, `Rm2kListWalker`, `Rm2kCommandLists` and `Rm2kEventCommands` are plain structs in caller memory, set up by a `_init` function and advanced by `_next`. They borrow the buffer they were initialised from (which must outlive them) and allocate nothing. `rm2k_split_header` skips a file's magic to get the chunk stream they expect, `rm2k_chunk_find` looks up one chunk by id, and the `RM2K_CHUNK_*` constants are the chunk ids for navigating by hand.

**Diagnostics.** `Rm2kDiagnostics` (`rm2k_diagnostics_new`, `_get`, `_free`, ...) optionally collects recoverable parse problems. Pass `NULL` wherever one is expected to ignore them. Strings are byte pointers, never NUL-terminated, except `rm2k_diagnostic_kind_name`, which returns a NUL-terminated English label.

## `no_std`

Built with `--no-default-features`, the owned-handle layer has no allocator of its own: register one with `rm2k_set_allocator` before any other allocating call (every build exports it; with `std` it does nothing). The zero-copy walkers never allocate, so a consumer using only those has nothing to register.

```c
typedef uint8_t* (*Rm2kAllocFn)(size_t size, size_t align);
typedef void (*Rm2kFreeFn)(uint8_t* ptr, size_t size, size_t align);
void rm2k_set_allocator(Rm2kAllocFn alloc, Rm2kFreeFn free);
```

`alloc` returns at least `size` bytes aligned to `align` (a power of two), or `NULL`; `free` gets back exactly the `(ptr, size, align)` that `alloc` produced. Without a registered allocator the first allocation aborts the process, and a `NULL` from `alloc` does the same. Allocation failure is fatal here, never an error return, so out-of-memory never shows up as a `NULL` or `false` result. See [`examples/README.md`](examples/README.md) for a worked example.

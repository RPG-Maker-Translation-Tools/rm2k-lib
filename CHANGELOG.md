## 1.2.0

- **Breaking:** C API diagnostics now include the struct each problem was found in, through a new `struct_name` field on `Rm2kDiagnostic`. Rebuild C code against the new `rm2k.h`.
- Added the `rm2k_engine_from_ldb_id` C function and the `engineFromLdbId` WASM function, which classify a raw `system.ldb_id` as RPG Maker 2000 or 2003.
- Fixed saving a map whose event had a corrupt command: the commands after it were lost on the next load, so a second save changed the file. A command with a zero code now ends its event's command list on load, as other corruption already does.
- The C API's `Rm2kEventCommand` field `_params` is now `raw_params`, matching the Rust method. Read parameters through `rm2k_event_command_param` as before.
- Fixed a crafted file making loading allocate gigabytes: a gap marker in a string list could expand to billions of empty entries, and repeating the chunk multiplied the work. A load now produces at most 1,048,576 string-list entries across all its string lists. Entries past that are dropped and reported as `EntryBudgetExhausted`, a new diagnostic kind (`RM2K_DIAG_ENTRY_BUDGET_EXHAUSTED` in the C API).
- Fixed a string list losing its entries after a run of empty entries longer than the chunk it was written in. Loading dropped the run and everything after it, so a save and load changed the file.
- A corrupt gap marker in a string list is now reported as a bad integer instead of being skipped silently.
- The minimum supported Rust version is now 1.81, up from 1.77, because thiserror's no_std build needs it.
- The C API's no_std build now links on Linux. It failed there because of a missing `rust_eh_personality` symbol.

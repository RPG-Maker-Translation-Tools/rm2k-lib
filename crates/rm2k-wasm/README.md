# rm2k-wasm

WebAssembly bindings for [`rm2k-lib`](https://github.com/savannstm/rm2k-lib), which reads and writes the RPG Maker 2000/2003 LCF binary formats, generated via [`wasm-bindgen`](https://github.com/wasm-bindgen/wasm-bindgen).

The domain model (`Database`, `Map`, `Save`, `TreeMap` and everything nested under them) is close to a hundred structs, so it is not individually wrapped. Instead, loaded values cross the JS boundary as plain JavaScript values via [`serde-wasm-bindgen`](https://github.com/RReverser/serde-wasm-bindgen), reusing `rm2k-lib`'s own `serde` feature. `value` is therefore untyped (`any`) on the TypeScript side by design - treat it as the same object shape you'd get from `rm2k`'s own `serde_json` output, documented in the [root crate's README](https://github.com/savannstm/rm2k-lib#readme).

## Install

```bash
npm install rm2k-wasm
```

## API

```ts
class LoadResult {
    value: any;
    header: Uint8Array;
    // Diagnostic[]-shaped: { offset: number, chunkId: number, structName: string, kind: string }
    diagnostics: any;
}

enum Engine { R2K, R2K3 }

function loadDatabase(bytes: Uint8Array): LoadResult;
function saveDatabase(value: any, engine: Engine, preserveHeader: boolean, header: Uint8Array): Uint8Array;

function loadMap(bytes: Uint8Array): LoadResult;
function saveMap(value: any, engine: Engine, preserveHeader: boolean, header: Uint8Array): Uint8Array;

function loadSave(bytes: Uint8Array): LoadResult;
function saveSave(value: any, engine: Engine, preserveHeader: boolean, header: Uint8Array): Uint8Array;

function loadTreeMap(bytes: Uint8Array): LoadResult;
function saveTreeMap(value: any, engine: Engine, preserveHeader: boolean, header: Uint8Array): Uint8Array;
```

Every `loadX` call always collects recoverable parse diagnostics (see the root crate's "Leniency" docs) rather than silently discarding them - `diagnostics` is usually an empty array. `header` is the file's original magic bytes; pass it back into `saveX` with `preserveHeader: true` to keep a non-canonical header byte-exact. `Engine` picks which RPG Maker's field layout to write, since R2K and R2K3 diverge on a handful of structs.

## Usage

```ts
import init, { loadDatabase, saveDatabase, Engine } from "rm2k-wasm";

await init(); // browsers & Deno: no-arg init() works out of the box

const bytes = new Uint8Array(await Deno.readFile("./RPG_RT.ldb"));
const { value, header, diagnostics } = loadDatabase(bytes);
if (diagnostics.length > 0) console.warn(diagnostics);

value.system.system_name = "..."; // edit the loosely-typed value in place
const out = saveDatabase(value, Engine.R2K, true, header);
```

Under Node/Bun, `init()`'s default `fetch()`-based loading doesn't apply - pass the `.wasm` bytes explicitly instead:

```ts
import { readFile } from "node:fs/promises";
import init, { loadDatabase } from "rm2k-wasm";

await init(
  await readFile(new URL("./rm2k_wasm_bg.wasm", import.meta.resolve("rm2k-wasm"))),
);
```

## Building

```bash
wasm-pack build --release --target web
```

Requires the `wasm32-unknown-unknown` rustup target (`rustup target add wasm32-unknown-unknown`) and `wasm-pack` (`cargo binstall wasm-pack`). Output goes to `pkg/` (gitignored): the compiled `.wasm`, a JS glue module, a `.d.ts`, and the `package.json` this crate publishes to npm from.

## Testing

```bash
wasm-pack test --node
```

Exercises the `serde-wasm-bindgen` conversion boundary against synthetic, guaranteed-valid values (`Setup::defaults`) rather than a real game file - see [`tests/smoke.rs`](https://github.com/savannstm/rm2k-lib/blob/master/crates/rm2k-wasm/tests/smoke.rs).

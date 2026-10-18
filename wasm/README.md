# rm2k-wasm

WebAssembly bindings for [`rm2k-lib`](https://github.com/RPG-Maker-Translation-Tools/rm2k-lib), which reads and writes the RPG Maker 2000/2003 LCF binary formats. Loaded values are plain, untyped (`any`) objects with the same shape as `rm2k`'s serde output, described in the [root README](https://github.com/RPG-Maker-Translation-Tools/rm2k-lib#serde).

> **I won't be continuing to publish this package on NPM.** NPM has made their 2FA completely unbearable, and I don't want to participate in this logging-in shitshow anymore. I'm looking for the first person I meet to whom I'll pass the ownership of this package on NPM, if they're willing to maintain it. Until then, you can always [build from source](#build-from-source).

## Install

```bash
npm install rm2k-wasm
```

## API

```ts
class LoadResult {
  value: any;
  header: Uint8Array;
  // { offset: number, chunkId: number, structName: string, kind: string }[]
  diagnostics: any;
}

enum Engine {
  R2K,
  R2K3,
}

// Classifies a raw `system.ldb_id`: 2003 is R2K3, anything else is R2K.
function engineFromLdbId(ldbId: number): Engine;

function loadDatabase(bytes: Uint8Array): LoadResult;
function loadMap(bytes: Uint8Array): LoadResult;
function loadSave(bytes: Uint8Array): LoadResult;
function loadTreeMap(bytes: Uint8Array): LoadResult;

function saveDatabase(value: any, engine: Engine, preserveHeader: boolean, header: Uint8Array): Uint8Array;
function saveMap(value: any, engine: Engine, preserveHeader: boolean, header: Uint8Array): Uint8Array;
function saveSave(value: any, engine: Engine, preserveHeader: boolean, header: Uint8Array): Uint8Array;
function saveTreeMap(value: any, engine: Engine, preserveHeader: boolean, header: Uint8Array): Uint8Array;
```

Every `loadX` collects recoverable parse problems in `diagnostics` (usually an empty array). `header` is the file's original magic bytes; pass it back to `saveX` with `preserveHeader: true` to keep a non-canonical header byte-exact. `engine` picks which RPG Maker's field layout to write, since R2K and R2K3 differ in a few structs.

Strings use the escaped format from the root README's [Serde](https://github.com/RPG-Maker-Translation-Tools/rm2k-lib#serde) section, so no text is decoded for you. Printable ASCII is literal, every other byte appears as `\xNN`, and a loaded name is the literal text `\x82\xa0` for the bytes Shift-JIS `あ`. Characters you assign are stored as UTF-8, so write other bytes as `\xNN` escapes. A `\` that doesn't start a valid escape must be doubled to `\\`, or the `saveX` call throws.

## Usage

```ts
import { readFile } from "node:fs/promises";
import init, { loadDatabase, saveDatabase, Engine } from "rm2k-wasm";

await init(await readFile(new URL("./rm2k_wasm_bg.wasm", import.meta.resolve("rm2k-wasm"))));

const bytes = new Uint8Array(await readFile("./RPG_RT.ldb"));
const { value, header, diagnostics } = loadDatabase(bytes);
if (diagnostics.length > 0) console.warn(diagnostics);

value.system.system_name = "..."; // edit in place
const out = saveDatabase(value, Engine.R2K, true, header);
```

## Build from source

```bash
wasm-pack build --release --target web   # output in pkg/
```

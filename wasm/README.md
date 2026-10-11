# rm2k-wasm

WebAssembly bindings for [`rm2k-lib`](https://github.com/RPG-Maker-Translation-Tools/rm2k-lib), which reads and writes the RPG Maker 2000/2003 LCF binary formats. Loaded values are plain, untyped (`any`) objects with the same shape as `rm2k`'s `serde_json` output, documented in the [root README](https://github.com/RPG-Maker-Translation-Tools/rm2k-lib#readme).

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

## Usage

```ts
import init, { loadDatabase, saveDatabase, Engine } from "rm2k-wasm";

await init(); // browsers & Deno

const bytes = new Uint8Array(await Deno.readFile("./RPG_RT.ldb"));
const { value, header, diagnostics } = loadDatabase(bytes);
if (diagnostics.length > 0) console.warn(diagnostics);

value.system.system_name = "..."; // edit in place
const out = saveDatabase(value, Engine.R2K, true, header);
```

Under Node/Bun, pass the `.wasm` bytes to `init()` explicitly:

```ts
import { readFile } from "node:fs/promises";
import init, { loadDatabase } from "rm2k-wasm";

await init(await readFile(new URL("./rm2k_wasm_bg.wasm", import.meta.resolve("rm2k-wasm"))));
```

## Build from source

```bash
wasm-pack build --release --target web   # output in pkg/
```

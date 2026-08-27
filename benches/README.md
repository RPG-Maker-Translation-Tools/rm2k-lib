# Benchmarks

A measured comparison against `liblcf`. Two things are timed, both against the same real game ([OFF](https://www.mediafire.com/file/sx1kyo89yu0dko1/OFF_English_3.0.zip/file), RPG Maker 2000, English release):

- **`ldb`** - parsing `RPG_RT.ldb`, the database (one large structured file: ~262 KB).
- **`maps`** - parsing every `.lmu` map file in the game directory (347 files), once per pass.

## Methodology

Both sides read every file into memory once, up front, outside the timed region. Each timed iteration then measures **only the parse call itself** - no disk I/O, so results are not sensitive to page-cache state.

`liblcf`'s public API has no zero-copy, in-memory load entry point - only `Load(std::string_view filename, ...)` or `Load(std::istream&, ...)`. The benchmark uses the `istream` overload, fed by an `istringstream` constructed fresh from the in-memory bytes on every iteration - which copies the buffer. rm2k's `file::load_database`/`load_map` take a `&[u8]` directly and copy nothing. It is a genuine cost of using `liblcf`'s API as a real caller would. It also is not the main story - see [Results](#results).

`liblcf` is built with `LIBLCF_WITH_ICU=OFF` (no ICU dev package was available on the machine this was run on). This does **not** skip encoding conversion - `ReaderUtil::DetectEncodings` just returns a hardcoded `"windows-1252"` instead of running ICU's charset detector, and `Encoder`'s non-ICU path still transcodes every string through Win32's `MultiByteToWideChar`/`WideCharToMultiByte`. So the benchmark still exercises `liblcf`'s real encoding-conversion code path, just without the detection scan - which is itself a supported `liblcf` configuration (`Encoder::Init`, `src/encoder.cpp`), not an approximation.

`rm2k` never transcodes text (see the top-level `README`) - this is exactly the design difference why it's faster.

## Running it

**Rust side** (from this repo):

```bash
cargo bench --bench database_load
# or point it at a different game:
RM2K_BENCH_GAME_DIR="/path/to/game" cargo bench --bench database_load
```

**liblcf side** - liblcf is not vendored in this repo; point the build at your own clone
(a Windows checkout at `C:\Repos\liblcf` is what was used for the numbers here):

```bash
# 1. Build a minimal liblcf static library (no ICU/XML/INI/tools/tests - see Methodology).
git clone --depth 1 --branch 0.8.1 https://github.com/EasyRPG/liblcf /tmp/liblcf
cd /tmp/liblcf

cmake -B build-bench -G Ninja -DCMAKE_BUILD_TYPE=Release \
      -DBUILD_SHARED_LIBS=OFF -DLIBLCF_WITH_ICU=OFF -DLIBLCF_WITH_XML=OFF -DLIBLCF_WITH_INI=OFF \
      -DLIBLCF_ENABLE_TOOLS=OFF -DLIBLCF_ENABLE_TESTS=OFF -DLIBLCF_ENABLE_BENCHMARKS=OFF \
      -DLIBLCF_ENABLE_INSTALL=OFF
cmake --build build-bench --target lcf -j
```

Build this repo's comparison harness against it:

**Linux/macOS:**

```bash
clang++ -std=c++17 -O3 -DNDEBUG \
    -I /tmp/liblcf/build-bench/src -I src -I src/generated \
    benches/liblcf_cmp/bench_load.cpp -o benches/liblcf_cmp/bench_load \
    /tmp/liblcf/build-bench/liblcf.a

./benches/liblcf_cmp/bench_load ldb "/path/to/game/RPG_RT.ldb"
./benches/liblcf_cmp/bench_load maps "/path/to/game"
```

**Windows:**

```bash
clang++ -std=c++17 -O3 -DNDEBUG -fms-runtime-lib=dll \
    -I /tmp/liblcf/build-bench/src -I src -I src/generated \
    benches/liblcf_cmp/bench_load.cpp -o benches/liblcf_cmp/bench_load.exe \
    /tmp/liblcf/build-bench/lcf.lib

./benches/liblcf_cmp/bench_load.exe ldb "C:/path/to/game/RPG_RT.ldb"
./benches/liblcf_cmp/bench_load.exe maps "C:/path/to/game"
```

(`-fms-runtime-lib=dll` matters on Windows: clang++ and liblcf's own CMake build must agree on the CRT, or the link fails with a `RuntimeLibrary` mismatch.)

The C++ harness hand-rolls warmup + timed-iteration + min/mean/median/max/stddev, matching criterion's methodology closely enough to compare directly, since no C++ benchmarking framework was already available and pulling one in just for this felt like overkill.

## Results

Measured on an AMD Ryzen 5 2600 (6-core), Windows, both sides built `-O3`/`release` with LTO.
liblcf commit as checked out in `C:\Repos\liblcf` at benchmark time; rm2k-lib as of this commit.

| Benchmark                                      | rm2k-lib                           | liblcf          | Speedup   |
| ---------------------------------------------- | ---------------------------------- | --------------- | --------- |
| `RPG_RT.ldb` (single database, ~262 KB)        | 1.90 ms (criterion point estimate) | 11.83 ms (mean) | **~6.2x** |
| One map file (`.lmu`, averaged over 347 files) | 0.0234 ms                          | 0.207 ms        | **~8.8x** |
| All 347 maps, one pass                         | 8.12 ms                            | 71.84 ms        | **~8.8x** |

Raw output:

```text
# rm2k-lib (cargo bench --bench database_load)
ldb (single database)   time:   [1.8126 ms 1.9006 ms 1.9888 ms]
maps/one full pass      time:   [7.6593 ms 8.1206 ms 8.6843 ms]
                        thrpt:  [39.957 Kelem/s 42.731 Kelem/s 45.304 Kelem/s]

# liblcf (bench_load.exe)
ldb (single database)    iters=300    min=9.4887ms mean=11.8271ms median=11.5912ms max=20.2499ms stddev=1.3837ms
maps (one full pass)     iters=30     min=63.2384ms mean=71.8446ms median=70.1383ms max=89.1314ms stddev=5.2812ms
maps (per file)          iters=30     min=0.1822ms mean=0.2070ms median=0.2021ms max=0.2569ms stddev=0.0152ms
```

This is not an exhaustive suite, but it is an apples-to-apples benchmark.

# FFI test

`test_ffi.c` exercises the C API against a real game.

Build and run it by hand:

```bash
cd crates/rm2k-capi
cargo cinstall --release --prefix="$PWD/target/capi-install"
```

**Linux/macOS:**

```bash
clang -std=c11 -I crates/rm2k-capi/assets crates/rm2k-capi/tests/test_ffi.c \
    -o test_ffi target/release/librm2k.a -lpthread -ldl -lm
./test_ffi "/path/to/game"
```

**Windows:**

```bash
clang -std=c11 -I crates/rm2k-capi/assets crates/rm2k-capi/tests/test_ffi.c \
    -o test_ffi.exe target/release/rm2k.lib -lws2_32 -luserenv -lntdll -lbcrypt
./test_ffi.exe "C:/path/to/game"
```

With no argument it runs only the tests that need no game files.

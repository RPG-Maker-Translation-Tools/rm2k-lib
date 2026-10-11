# FFI test

`test_ffi.c` exercises the C API against a real game. With no argument it runs only the tests that need no game files.

```bash
cd capi
cargo cinstall --release --prefix="$PWD/target/capi-install"
cd ..
```

From the repository root:

```bash
# Linux/macOS
clang -std=c11 -I capi/target/capi-install/include capi/tests/test_ffi.c \
    -o test_ffi target/release/librm2k.a -lpthread -ldl -lm
./test_ffi "/path/to/game"

# Windows
clang -std=c11 -I capi/target/capi-install/include capi/tests/test_ffi.c \
    -o test_ffi.exe target/release/rm2k.lib -lws2_32 -luserenv -lntdll -lbcrypt
./test_ffi.exe "C:/path/to/game"
```

# FFI test

`test_c_api.c` exercises the C API against a real game. With no argument it runs only the tests that need no game files.

```bash
cd capi
cargo cinstall --release --prefix="$PWD/target/capi-install"
cd ..
```

From the repository root:

```bash
# Linux/macOS
clang -std=c11 -I capi/target/capi-install/include capi/tests/test_c_api.c \
    -o test_c_api capi/target/capi-install/lib/librm2k.a -lpthread -ldl -lm
./test_c_api "/path/to/game"

# Windows
clang -std=c11 -I capi/target/capi-install/include capi/tests/test_c_api.c \
    -o test_c_api.exe capi/target/capi-install/lib/rm2k.lib -lws2_32 -luserenv -lntdll -lbcrypt -lsynchronization
./test_c_api.exe "C:/path/to/game"
```

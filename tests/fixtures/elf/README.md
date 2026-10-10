# ELF dynamic symbol fixture

`symbols.so` is original MIT-licensed test data assembled from `symbols.S` and
`symbols.map` with GNU binutils. It has no libc dependency. A controlled fixture
program may use it as the real runtime counterpart to a generated link-only DSO;
the generated counterpart is never used as a runtime library.
It exercises default/nondefault versions, weak binding, protected visibility,
same-range observations, object and TLS sizes, IFUNC, and undefined imports.

Run `bash tests/fixtures/elf/build.sh` on x86_64 Linux to regenerate it.
The binary and source hashes are recorded in `SHA256SUMS`, and assembler/linker
versions in `tools.txt`. Normal Rust tests read the committed binary and require
neither binutils nor Linux. Tests mutate its tables and check every truncation.

# Experimental glibc provider contract

Version 1 connects generated link-only DSOs to Cargo through a trusted local
descriptor. It supports x86_64 Linux dynamically linked non-PIE Rust executables
and exactly one glibc release from 2.17 through 2.44, every release in the pinned ABI lists (the `RELEASES` list in `src/provider.rs`). It is not an authenticated or relocatable release SDK.
The six-field v1 format is unchanged; existing 2.17 descriptors remain valid.

The descriptor is UTF-8, at most 16 KiB, with LF endings and a final newline.
The first line is `libc-abi-provider-v1`; six tab-separated key/value lines follow.
Unknown, duplicate, missing, empty, and control-character fields fail. Field order
is immaterial to parsing; the exporter uses this order:

```text
libc-abi-provider-v1
target<TAB>x86_64-unknown-linux-gnu
glibc<TAB>2.17
sysroot<TAB>/work/prebuilt
stubs<TAB>/work/full-libraries-prebuilt
gcc-startup<TAB>/usr/lib/gcc/x86_64-linux-gnu/13
compiler-runtime<TAB>/work/compiler-runtime
```

`<TAB>` represents a literal tab. All paths must be absolute, without parent
components. Required files must resolve inside their declared directories;
relative symlinks inside those directories are allowed.

| Directory | Required files and purpose |
| --- | --- |
| sysroot | Exact selected-release `usr/include/features.h`; `usr/lib64/{crt1.o,crti.o,crtn.o,libc_nonshared.a}`, plus `usr/lib64/libpthread_nonshared.a` before 2.28 (glibc 2.28 merged it into `libc_nonshared.a`); real `lib64/ld-linux-x86-64.so.2` |
| stubs | Eight generated DSOs with exact SONAME filenames and `lib{name}.so` symlinks resolving to them; loader uses `libld.so` |
| gcc-startup | Explicit GCC `crtbegin.o`/`crtend.o`, providing compiler startup support |
| compiler-runtime | Real compatible `libgcc_s.so.1` and `libgcc_s.so`; experiment uses CentOS GCC 4.8.5's unwinder |

From the libc-abi-tools checkout, export after generating and independently
checking the complete stubs:

```bash
cargo run --locked --offline --example provider_2_17 -- \
  data/glibc /work/prebuilt /work/full-libraries-prebuilt \
  /usr/lib/gcc/x86_64-linux-gnu/13 /work/compiler-runtime /work/provider.txt
```

The exporter validates all eight DSOs against the pinned ABI, including SONAME,
versions, kinds, and object sizes, before creating a new descriptor. Existing
files are never overwritten. Real inputs retain their separate provenance/licenses.

Consume from the cargo-libc checkout:

```bash
cargo libc build --provider /work/provider.txt --glibc 2.17 \
  --cc /usr/bin/clang-24 --ld /usr/bin/ld.bfd \
  --artifact /work/app/x86_64-unknown-linux-gnu/debug/glibc-hello -- \
  --manifest-path examples/hello/Cargo.toml --target-dir /work/app --locked --offline
```

`--provider` conflicts with `--sysroot`/`--gcc-runtime`. Unsupported or mismatched releases fail without
fallback. Original sysroot mode remains available. The consumer validates paths,
required files, and exact headers; explicitly selects CRT/nonshared files; searches
stubs and the compatible unwinder first; and checks direct executable GLIBC requirements.
It does not revalidate full ABI membership or hash contents on every build.

Explicit Cargo `--target` and encoded target Rust flags preserve host compilation
of build scripts/procedural macros. Descriptor contents enter Cargo's fingerprint;
changing input files in place still requires a fresh target directory or clean
rebuild. Arbitrary native dependency overrides remain outside the prototype scope.

`examples/glibc-two-paths/provider-and-check.sh WORK CARGO_LIBC_REPO CLANG` tests
the selected release's input routes, host build units, a target thread, wrong-version rejection,
ELF requirements, all shared link-map inputs, and runtime dependency paths. Its
SHA-256 logs are evidence, not authenticated SDK manifests.

For 2.22, set `ABI_GLIBC_RELEASE=2.22` and use `--example provider_glibc -- DATA
2.22 SYSROOT STUBS GCC_STARTUP COMPILER_RUNTIME NEW_DESCRIPTOR`. Only the pinned
source-built target route is used. See the [2.22 experiment](GLIBC-2.22.md).

Future release packaging requires relative bundle paths, complete inventories,
integrity validation, provenance/licenses, stable compiler-runtime policy, and
target distribution/kernel qualification in a future contract version.

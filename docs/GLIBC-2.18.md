# glibc 2.18 provider

The experimental x86_64 Linux provider selects GNU glibc 2.18 explicitly.
Matching headers, CRT objects, nonshared archives, loader, and runtime DSOs come
from a pinned source build. This is a generic GNU environment; distribution and
old-kernel qualification remain separate tasks.

Inputs are pinned in [INPUTS.toml](../examples/glibc-2.18/INPUTS.toml):

- Official GNU `glibc-2.18.tar.xz`, SHA-256
  `2cb4e1e381928f1e5e55e71ab1ba8e0ea7ede75ff9709770435bfd018ea257a3`.
- Digest-pinned CentOS 7 builder and archived signature-checked packages,
  including GCC 4.8.5 and Linux 3.10 UAPI headers.
- `--prefix=/usr --libdir=/usr/lib64 --disable-werror --enable-kernel=3.2`.
- No source patches. The official 2.18 Makefile already contains the freestanding
  syscall-header preprocessing fix applied locally to 2.17.
- Compatible real CentOS GCC unwinder and explicit host GCC CRT begin/end objects.

The work tree's `prebuilt` directory supplies only the bootstrap unwinder file;
it is not a 2.18 distribution runtime. The selected target route is `source`.
The checksum pins retrieved HTTPS bytes; the upstream signature has not been
independently verified. Generated stubs supply link-time interfaces, never
runtime implementations.

| Library | Public numeric GLIBC definitions |
| --- | ---: |
| c | 2126 |
| m | 398 |
| ld | 9 |
| resolv | 91 |
| pthread | 235 |
| dl | 9 |
| rt | 47 |
| util | 6 |
| **Total** | **2921** |

All eight native DSOs match GNU readelf and the pinned snapshot with zero ABI
differences. The full stub audit preserves measured defaults, compatibility
versions, native types, binding/visibility, object sizes, alignment, and shared
storage. Exhaustive isolated-library checks pass **2921 accepted versioned
interfaces, 1750 rejected interfaces, and 2711 ordinary default bindings**.
The synthetic TLS relocation fixture also passes; no public numeric TLS rows
are present in the current historical matrix.

Real-runtime tests pass allocation, threads/TLS destruction, unwinding, historical
data views, memcpy and fmemopen compatibility behavior, plus the five dedicated
math/loading/realtime/utility/resolver suites. Cargo preserves host build scripts
and proc macros, audits target link/runtime paths, rejects mismatched release
selection, and fails at the linker on the too-new `memfd_create` import.

The new boundary fixture tests `pthread_getattr_default_np@GLIBC_2.18` and
`pthread_setattr_default_np@GLIBC_2.18`: changing and restoring process-default
thread stack attributes works under 2.18 and 2.22, while 2.17 rejects both names.
The matrix also records four other additions from 2.17 to 2.18: libc's
`__cxa_thread_atexit_impl` and libm's three `__issignaling` variants. All six
additions have exhaustive relocation coverage; the boundary fixture does not
claim to behaviorally test all six.

The six-field v1 descriptor is unchanged and now accepts exactly 2.17, 2.18,
or 2.22. Both repositories implement identical parsers; older consumers reject
2.18 explicitly. Existing provider selections remain exact, with no fallback.

## Reproduction

On an x86_64 Linux build host with the documented tools:

```bash
export ABI_GLIBC_RELEASE=2.18
export RUSTUP_TOOLCHAIN=1.99.0
export ABI_BEHAVIOR_CC=/usr/bin/clang-24
bash examples/glibc-2.18/run.sh /tmp/glibc-2.18-work
bash examples/glibc-two-paths/elf-and-check.sh /tmp/glibc-2.18-work
bash examples/glibc-two-paths/full-and-check.sh /tmp/glibc-2.18-work
bash examples/glibc-two-paths/provider-and-check.sh \
  /tmp/glibc-2.18-work ../cargo-libc /usr/bin/clang-24
```

Use a new absolute work directory. These scripts install into that tree, not the
host system. Local validation used Rust 1.99.0, Clang 18.1.3, and GNU binutils 2.42.
The Cargo example's highest directly imported numeric GLIBC tag was 2.18; this
is an import measurement, not a general minimum-runtime guarantee.

The measured profile `gnu-source-2.18`, input/metadata hashes, and default roles
are committed to [history provenance](../data/glibc-history/PROVENANCE.json) and
[the interface reference](reference/GLIBC-SYMBOLS.md). Additional evidence can be
imported deliberately with `scripts/import-glibc-elf-profile.py` after independent
readelf/snapshot qualification; existing profiles are never silently replaced.
Reference regeneration checks exact membership and hashes for every profile.

Both new 2.18 jobs in the [hosted provider matrix](https://github.com/rustcommons/linux-abi-tools/actions/runs/38066726167)
passed under Rust 1.99.0 and Clang 21/24 on 2026-10-10 at producer commit
`75945c278187899c537af636fdc311a07e27132f`, with pinned consumer commit
`edfca5b9f9fb811069f302e8d2d5ef2f99f7fd33`. All six jobs passed, including
both existing provider releases. Actual distribution/kernel qualification,
native dependency adapters, and authenticated SDK packaging remain open.

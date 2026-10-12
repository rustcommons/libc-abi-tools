# glibc 2.28 provider

The experimental x86_64 Linux provider selects GNU glibc 2.28 explicitly.
Matching headers, CRT objects, nonshared archives, loader, and runtime DSOs come
from a pinned source build. This is a generic GNU environment; distribution and
old-kernel qualification remain separate tasks.

Inputs are pinned in [INPUTS.toml](../examples/glibc-2.28/INPUTS.toml):

- Official GNU `glibc-2.28.tar.xz`, SHA-256
  `b1900051afad76f7a4f73e71413df4826dce085ef8ddb785a945b66d7d513082`.
  On 2026-10-11 its detached signature verified with `gpgv` against the GNU
  keyring (RSA key `16792B4EA25340F8`, Carlos O'Donell). CI checks the SHA-256 only.
- Builder `docker.io/library/debian@sha256:16ee3e11da473f6565ef94a715f1a38b3079e0664913be78869c336a71cb6085` with packages from
  http://archive.debian.org/debian stretch main (signed Release; Valid-Until not enforced): GCC 6.3.0-18+deb9u1, binutils 2.28-5, kernel headers linux-libc-dev 4.9.228-1.
- `--prefix=/usr --libdir=/usr/lib64 --disable-werror --enable-kernel=3.2`; no source patches.
- Compiler runtime: Link: generated libgcc_s stub with GCC 4.8.5's interface; tests run with the real Debian 9 GCC 6.3 libgcc_s.so.1 (requires GLIBC_2.14 at most).

| Library | Public numeric GLIBC definitions |
| --- | ---: |
| c | 2189 |
| m | 1068 |
| ld | 8 |
| resolv | 91 |
| pthread | 256 |
| dl | 9 |
| rt | 47 |
| util | 6 |
| **Total** | **3674** |

All eight native DSOs match GNU readelf and the pinned snapshot with zero ABI
differences. The stub audit preserves measured defaults, compatibility versions,
native types, binding/visibility, object sizes, alignment, and shared storage.
Exhaustive isolated-library checks pass **3674 accepted versioned
interfaces, 997 rejected interfaces, 3382 ordinary default bindings,
and 233 compatibility-only names rejected for new unversioned references**.

## Changes from 2.27

The snapshot adds 80 and removes 0 interfaces in this scope:
Added: `fcntl64@GLIBC_2.28` (c), `renameat2@GLIBC_2.28` (c), `statx@GLIBC_2.28` (c), `thrd_current@GLIBC_2.28` (c), `thrd_equal@GLIBC_2.28` (c), `thrd_sleep@GLIBC_2.28` (c), `thrd_yield@GLIBC_2.28` (c), `daddl@GLIBC_2.28` (m), `ddivl@GLIBC_2.28` (m), `dmull@GLIBC_2.28` (m), `dsubl@GLIBC_2.28` (m), `f32addf128@GLIBC_2.28` (m), `f32addf32x@GLIBC_2.28` (m), `f32addf64@GLIBC_2.28` (m), `f32addf64x@GLIBC_2.28` (m), `f32divf128@GLIBC_2.28` (m), `f32divf32x@GLIBC_2.28` (m), `f32divf64@GLIBC_2.28` (m), `f32divf64x@GLIBC_2.28` (m), `f32mulf128@GLIBC_2.28` (m), `f32mulf32x@GLIBC_2.28` (m), `f32mulf64@GLIBC_2.28` (m), `f32mulf64x@GLIBC_2.28` (m), `f32subf128@GLIBC_2.28` (m), `f32subf32x@GLIBC_2.28` (m), `f32subf64@GLIBC_2.28` (m), `f32subf64x@GLIBC_2.28` (m), `f32xaddf128@GLIBC_2.28` (m), `f32xaddf64@GLIBC_2.28` (m), `f32xaddf64x@GLIBC_2.28` (m), `f32xdivf128@GLIBC_2.28` (m), `f32xdivf64@GLIBC_2.28` (m), `f32xdivf64x@GLIBC_2.28` (m), `f32xmulf128@GLIBC_2.28` (m), `f32xmulf64@GLIBC_2.28` (m), `f32xmulf64x@GLIBC_2.28` (m), `f32xsubf128@GLIBC_2.28` (m), `f32xsubf64@GLIBC_2.28` (m), `f32xsubf64x@GLIBC_2.28` (m), `f64addf128@GLIBC_2.28` (m), and 40 more.

Measured default roles changed for 10 interfaces (`yes` is the default `@@` version, `no` a compatibility `@` version): `llseek@GLIBC_2.2.5` (c) yes to no, `nfsservctl@GLIBC_2.2.5` (c) yes to no, `rpc_createerr@GLIBC_2.2.5` (c) yes to no, `svc_fdset@GLIBC_2.2.5` (c) yes to no, `svc_max_pollfd@GLIBC_2.2.5` (c) yes to no, `svc_pollfd@GLIBC_2.2.5` (c) yes to no, `svcauthdes_stats@GLIBC_2.2.5` (c) yes to no, `ustat@GLIBC_2.2.5` (c) yes to no, `__fcntl@GLIBC_2.2.5` (pthread) yes to no, `fcntl@GLIBC_2.2.5` (pthread) yes to no.
Measured symbol types changed for `wcscmp@GLIBC_2.2.5` (c) FUNC to IFUNC, `wcsncmp@GLIBC_2.2.5` (c) FUNC to IFUNC.
16 interfaces changed between GLOBAL and WEAK binding; the stubs preserve
these, and the exhaustive default checks cover the resulting bindings.

2.28 merged `libpthread_nonshared.a` into `libc_nonshared.a`. The provider contract, both repositories'
link lines, and the CI bundle require and link `libpthread_nonshared.a` only before 2.28. 2.28 also
made `ustat`, `nfsservctl`, `llseek`, and libpthread's `fcntl` compatibility-only.

## Applications and Cargo

Real-runtime tests pass allocation, threads/TLS destruction, unwinding, historical
data views, memcpy and fmemopen compatibility behavior, the pthread default-attribute
and sigvec boundary fixtures (each following this provider's evidence), and the five
dedicated math/loading/realtime/utility/resolver suites. Cargo preserves host build
scripts and proc macros, audits target link/runtime paths, and rejects mismatched
release selection. The cargo-libc build, tier 1 scan, tier 2 local runner under a
Linux 3.10 filter, and `check` all pass. The too-new link tests use `getcpu`,
chosen from the matrix (`too-new-symbol.py`), and fail at the linker as required.
The Rust test program's highest directly imported GLIBC tag was 2.28; this is an
import measurement, not a general minimum-runtime guarantee.

## Reproduction

```bash
export ABI_GLIBC_RELEASE=2.28
export RUSTUP_TOOLCHAIN=1.99.0
export ABI_BEHAVIOR_CC=/usr/bin/clang-24
bash examples/glibc-2.28/run.sh /tmp/glibc-2.28-work
bash examples/glibc-two-paths/elf-and-check.sh /tmp/glibc-2.28-work
bash examples/glibc-two-paths/prepare-and-check.sh /tmp/glibc-2.28-work
bash examples/glibc-two-paths/application-and-check.sh /tmp/glibc-2.28-work
bash examples/glibc-two-paths/provider-and-check.sh \
  /tmp/glibc-2.28-work ../cargo-libc /usr/bin/clang-24
```

Use a new absolute work directory. Local validation on 2026-10-11 used Rust 1.99.0,
Clang 18.1.3, GNU binutils 2.42, and podman 5.8.2 on WSL Ubuntu 24.04, with
cargo-libc `a259c167741c6b1d31eaa3b7c664df8faff3b7a7`. The measured profile `gnu-source-2.28` is
committed to [history provenance](../data/glibc-history/PROVENANCE.json) and
[the interface reference](reference/GLIBC-SYMBOLS.md).

Both Clang 21/24 jobs for 2.28 passed in the [hosted provider run](https://github.com/rustcommons/linux-abi-tools/actions/runs/38155189880) at producer
`27caf3dbf9b7b5b018763629d3a3d2ebba377123` with consumer `a259c167741c6b1d31eaa3b7c664df8faff3b7a7`,
and again in the [complete 84-job run](https://github.com/rustcommons/linux-abi-tools/actions/runs/38158161339) at producer `ae68bf63f5ddde7fc7852e76eab83ad2c1beb2c0`
with consumer `bfe04693f6685770b47c9346e32de556a8029f4e` on 2026-10-11. Distribution/kernel
qualification, native dependency adapters, and authenticated SDK packaging remain open.

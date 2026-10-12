# glibc 2.40 provider

The experimental x86_64 Linux provider selects GNU glibc 2.40 explicitly.
Matching headers, CRT objects, nonshared archives, loader, and runtime DSOs come
from a pinned source build. This is a generic GNU environment; distribution and
old-kernel qualification remain separate tasks.

Inputs are pinned in [INPUTS.toml](../examples/glibc-2.40/INPUTS.toml):

- Official GNU `glibc-2.40.tar.xz`, SHA-256
  `19a890175e9263d748f627993de6f4b1af9cd21e03f080e4bfb3a1fac10205a2`.
  On 2026-10-11 its detached signature verified with `gpgv` against the GNU
  keyring (RSA key `16792B4EA25340F8`, Carlos O'Donell). CI checks the SHA-256 only.
- Builder `docker.io/library/debian@sha256:bc49dc1918ee1a47a93e65b5e4676e8680fb754b133197b92ca52bfe6731d5f0` with packages from
  http://snapshot.debian.org/archive/debian/20260901T000000Z bookworm main (signed Release; Valid-Until not enforced): GCC 12.2.0-14+deb12u1, binutils 2.40-2, kernel headers linux-libc-dev 6.1.176-1.
- `--prefix=/usr --libdir=/usr/lib64 --disable-werror --enable-kernel=3.2`; no source patches.
- Compiler runtime: Link: generated libgcc_s stub with GCC 4.8.5's interface; tests run with the real Debian 12 GCC 12.2 libgcc_s.so.1 (requires GLIBC_2.35 at most).

| Library | Public numeric GLIBC definitions |
| --- | ---: |
| c | 2818 |
| m | 1224 |
| ld | 8 |
| resolv | 67 |
| pthread | 12 |
| dl | 3 |
| rt | 5 |
| util | 1 |
| **Total** | **4138** |

All eight native DSOs match GNU readelf and the pinned snapshot with zero ABI
differences. The stub audit preserves measured defaults, compatibility versions,
native types, binding/visibility, object sizes, alignment, and shared storage.
Exhaustive isolated-library checks pass **4138 accepted versioned
interfaces, 533 rejected interfaces, 3428 ordinary default bindings,
and 415 compatibility-only names rejected for new unversioned references**.

## Changes from 2.39

The snapshot adds 40 and removes 0 interfaces in this scope:
Added: `exp10m1@GLIBC_2.40` (m), `exp10m1f@GLIBC_2.40` (m), `exp10m1f128@GLIBC_2.40` (m), `exp10m1f32@GLIBC_2.40` (m), `exp10m1f32x@GLIBC_2.40` (m), `exp10m1f64@GLIBC_2.40` (m), `exp10m1f64x@GLIBC_2.40` (m), `exp10m1l@GLIBC_2.40` (m), `exp2m1@GLIBC_2.40` (m), `exp2m1f@GLIBC_2.40` (m), `exp2m1f128@GLIBC_2.40` (m), `exp2m1f32@GLIBC_2.40` (m), `exp2m1f32x@GLIBC_2.40` (m), `exp2m1f64@GLIBC_2.40` (m), `exp2m1f64x@GLIBC_2.40` (m), `exp2m1l@GLIBC_2.40` (m), `log10p1@GLIBC_2.40` (m), `log10p1f@GLIBC_2.40` (m), `log10p1f128@GLIBC_2.40` (m), `log10p1f32@GLIBC_2.40` (m), `log10p1f32x@GLIBC_2.40` (m), `log10p1f64@GLIBC_2.40` (m), `log10p1f64x@GLIBC_2.40` (m), `log10p1l@GLIBC_2.40` (m), `log2p1@GLIBC_2.40` (m), `log2p1f@GLIBC_2.40` (m), `log2p1f128@GLIBC_2.40` (m), `log2p1f32@GLIBC_2.40` (m), `log2p1f32x@GLIBC_2.40` (m), `log2p1f64@GLIBC_2.40` (m), `log2p1f64x@GLIBC_2.40` (m), `log2p1l@GLIBC_2.40` (m), `logp1@GLIBC_2.40` (m), `logp1f@GLIBC_2.40` (m), `logp1f128@GLIBC_2.40` (m), `logp1f32@GLIBC_2.40` (m), `logp1f32x@GLIBC_2.40` (m), `logp1f64@GLIBC_2.40` (m), `logp1f64x@GLIBC_2.40` (m), `logp1l@GLIBC_2.40` (m).

Measured default roles changed for 2 interfaces (`yes` is the default `@@` version, `no` a compatibility `@` version): `_dl_mcount_wrapper@GLIBC_2.2.5` (c) yes to no, `_res_opcodes@GLIBC_2.2.5` (resolv) yes to no.
1 interfaces changed between GLOBAL and WEAK binding; the stubs preserve
these, and the exhaustive default checks cover the resulting bindings.

2.40 added the C23 `exp2m1`/`exp10m1`/`log2p1`/`log10p1`/`logp1` families in libm.

## Applications and Cargo

Real-runtime tests pass allocation, threads/TLS destruction, unwinding, historical
data views, memcpy and fmemopen compatibility behavior, the pthread default-attribute
and sigvec boundary fixtures (each following this provider's evidence), and the five
dedicated math/loading/realtime/utility/resolver suites. Cargo preserves host build
scripts and proc macros, audits target link/runtime paths, and rejects mismatched
release selection. The cargo-libc build, tier 1 scan, tier 2 local runner under a
Linux 3.10 filter, and `check` all pass. The too-new link tests use `sched_getattr`,
chosen from the matrix (`too-new-symbol.py`), and fail at the linker as required.
The Rust test program's highest directly imported GLIBC tag was 2.34; this is an
import measurement, not a general minimum-runtime guarantee.

## Reproduction

```bash
export ABI_GLIBC_RELEASE=2.40
export RUSTUP_TOOLCHAIN=1.99.0
export ABI_BEHAVIOR_CC=/usr/bin/clang-24
bash examples/glibc-2.40/run.sh /tmp/glibc-2.40-work
bash examples/glibc-two-paths/elf-and-check.sh /tmp/glibc-2.40-work
bash examples/glibc-two-paths/prepare-and-check.sh /tmp/glibc-2.40-work
bash examples/glibc-two-paths/application-and-check.sh /tmp/glibc-2.40-work
bash examples/glibc-two-paths/provider-and-check.sh \
  /tmp/glibc-2.40-work ../cargo-libc /usr/bin/clang-24
```

Use a new absolute work directory. Local validation on 2026-10-11 used Rust 1.99.0,
Clang 18.1.3, GNU binutils 2.42, and podman 5.8.2 on WSL Ubuntu 24.04, with
cargo-libc `bfe04693f6685770b47c9346e32de556a8029f4e`. The measured profile `gnu-source-2.40` is
committed to [history provenance](../data/glibc-history/PROVENANCE.json) and
[the interface reference](reference/GLIBC-SYMBOLS.md).

Both Clang 21/24 jobs for 2.40 passed in the [complete 84-job hosted run](https://github.com/rustcommons/linux-abi-tools/actions/runs/38158161339) at
producer `ae68bf63f5ddde7fc7852e76eab83ad2c1beb2c0` with consumer `bfe04693f6685770b47c9346e32de556a8029f4e` on 2026-10-11. Distribution/kernel
qualification, native dependency adapters, and authenticated SDK packaging remain open.

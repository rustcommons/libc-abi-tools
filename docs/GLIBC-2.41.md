# glibc 2.41 provider

The experimental x86_64 Linux provider selects GNU glibc 2.41 explicitly.
Matching headers, CRT objects, nonshared archives, loader, and runtime DSOs come
from a pinned source build. This is a generic GNU environment; distribution and
old-kernel qualification remain separate tasks.

Inputs are pinned in [INPUTS.toml](../examples/glibc-2.41/INPUTS.toml):

- Official GNU `glibc-2.41.tar.xz`, SHA-256
  `a5a26b22f545d6b7d7b3dd828e11e428f24f4fac43c934fb071b6a7d0828e901`.
  On 2026-10-11 its detached signature verified with `gpgv` against the GNU
  keyring (RSA key `DC2B16215ED5412A`, Andreas K. Huettel). CI checks the SHA-256 only.
- Builder `docker.io/library/debian@sha256:bc49dc1918ee1a47a93e65b5e4676e8680fb754b133197b92ca52bfe6731d5f0` with packages from
  http://snapshot.debian.org/archive/debian/20260901T000000Z bookworm main (signed Release; Valid-Until not enforced): GCC 12.2.0-14+deb12u1, binutils 2.40-2, kernel headers linux-libc-dev 6.1.176-1.
- `--prefix=/usr --libdir=/usr/lib64 --disable-werror --enable-kernel=3.2`; no source patches.
- Compiler runtime: Real Debian 12 GCC 12.2 libgcc_s.so.1 (requires GLIBC_2.35 at most).

| Library | Public numeric GLIBC definitions |
| --- | ---: |
| c | 2820 |
| m | 1280 |
| ld | 8 |
| resolv | 67 |
| pthread | 12 |
| dl | 3 |
| rt | 5 |
| util | 1 |
| **Total** | **4196** |

All eight native DSOs match GNU readelf and the pinned snapshot with zero ABI
differences. The stub audit preserves measured defaults, compatibility versions,
native types, binding/visibility, object sizes, alignment, and shared storage.
Exhaustive isolated-library checks pass **4196 accepted versioned
interfaces, 475 rejected interfaces, 3486 ordinary default bindings,
and 415 compatibility-only names rejected for new unversioned references**.

## Changes from 2.40

The snapshot adds 58 and removes 0 interfaces in this scope:
Added: `sched_getattr@GLIBC_2.41` (c), `sched_setattr@GLIBC_2.41` (c), `acospi@GLIBC_2.41` (m), `acospif@GLIBC_2.41` (m), `acospif128@GLIBC_2.41` (m), `acospif32@GLIBC_2.41` (m), `acospif32x@GLIBC_2.41` (m), `acospif64@GLIBC_2.41` (m), `acospif64x@GLIBC_2.41` (m), `acospil@GLIBC_2.41` (m), `asinpi@GLIBC_2.41` (m), `asinpif@GLIBC_2.41` (m), `asinpif128@GLIBC_2.41` (m), `asinpif32@GLIBC_2.41` (m), `asinpif32x@GLIBC_2.41` (m), `asinpif64@GLIBC_2.41` (m), `asinpif64x@GLIBC_2.41` (m), `asinpil@GLIBC_2.41` (m), `atan2pi@GLIBC_2.41` (m), `atan2pif@GLIBC_2.41` (m), `atan2pif128@GLIBC_2.41` (m), `atan2pif32@GLIBC_2.41` (m), `atan2pif32x@GLIBC_2.41` (m), `atan2pif64@GLIBC_2.41` (m), `atan2pif64x@GLIBC_2.41` (m), `atan2pil@GLIBC_2.41` (m), `atanpi@GLIBC_2.41` (m), `atanpif@GLIBC_2.41` (m), `atanpif128@GLIBC_2.41` (m), `atanpif32@GLIBC_2.41` (m), `atanpif32x@GLIBC_2.41` (m), `atanpif64@GLIBC_2.41` (m), `atanpif64x@GLIBC_2.41` (m), `atanpil@GLIBC_2.41` (m), `cospi@GLIBC_2.41` (m), `cospif@GLIBC_2.41` (m), `cospif128@GLIBC_2.41` (m), `cospif32@GLIBC_2.41` (m), `cospif32x@GLIBC_2.41` (m), `cospif64@GLIBC_2.41` (m), and 18 more.

No measured default role changed.
Measured symbol types changed for `exp10m1f@GLIBC_2.40` (m) FUNC to IFUNC, `exp10m1f32@GLIBC_2.40` (m) FUNC to IFUNC, `exp2m1f@GLIBC_2.40` (m) FUNC to IFUNC, `exp2m1f32@GLIBC_2.40` (m) FUNC to IFUNC.
5 interfaces changed between GLOBAL and WEAK binding; the stubs preserve
these, and the exhaustive default checks cover the resulting bindings.

2.41 added `sched_getattr`/`sched_setattr` and the C23 `*pi` trigonometric families. Its source is
signed by a new release manager key (Andreas K. Huettel), which the GNU keyring contains.

## Applications and Cargo

Real-runtime tests pass allocation, threads/TLS destruction, unwinding, historical
data views, memcpy and fmemopen compatibility behavior, the pthread default-attribute
and sigvec boundary fixtures (each following this provider's evidence), and the five
dedicated math/loading/realtime/utility/resolver suites. Cargo preserves host build
scripts and proc macros, audits target link/runtime paths, and rejects mismatched
release selection. The cargo-libc build, tier 1 scan, tier 2 local runner under a
Linux 3.10 filter, and `check` all pass. The too-new link tests use `__inet_ntop_chk`,
chosen from the matrix (`too-new-symbol.py`), and fail at the linker as required.
The Rust test program's highest directly imported GLIBC tag was 2.34; this is an
import measurement, not a general minimum-runtime guarantee.

## Reproduction

```bash
export ABI_GLIBC_RELEASE=2.41
export RUSTUP_TOOLCHAIN=1.99.0
export ABI_BEHAVIOR_CC=/usr/bin/clang-24
bash examples/glibc-2.41/run.sh /tmp/glibc-2.41-work
bash examples/glibc-two-paths/elf-and-check.sh /tmp/glibc-2.41-work
bash examples/glibc-two-paths/prepare-and-check.sh /tmp/glibc-2.41-work
bash examples/glibc-two-paths/application-and-check.sh /tmp/glibc-2.41-work
bash examples/glibc-two-paths/provider-and-check.sh \
  /tmp/glibc-2.41-work ../cargo-libc /usr/bin/clang-24
```

Use a new absolute work directory. Local validation on 2026-10-11 used Rust 1.99.0,
Clang 18.1.3, GNU binutils 2.42, and podman 5.8.2 on WSL Ubuntu 24.04, with
cargo-libc `bfe04693f6685770b47c9346e32de556a8029f4e`. The measured profile `gnu-source-2.41` is
committed to [history provenance](../data/glibc-history/PROVENANCE.json) and
[the interface reference](reference/GLIBC-SYMBOLS.md).

Both Clang 21/24 jobs for 2.41 passed in the [complete 84-job hosted run](https://github.com/rustcommons/linux-abi-tools/actions/runs/38158161339) at
producer `ae68bf63f5ddde7fc7852e76eab83ad2c1beb2c0` with consumer `bfe04693f6685770b47c9346e32de556a8029f4e` on 2026-10-11. Distribution/kernel
qualification, native dependency adapters, and authenticated SDK packaging remain open.

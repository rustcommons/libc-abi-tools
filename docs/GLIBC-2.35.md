# glibc 2.35 provider

The experimental x86_64 Linux provider selects GNU glibc 2.35 explicitly.
Matching headers, CRT objects, nonshared archives, loader, and runtime DSOs come
from a pinned source build. This is a generic GNU environment; distribution and
old-kernel qualification remain separate tasks.

Inputs are pinned in [INPUTS.toml](../examples/glibc-2.35/INPUTS.toml):

- Official GNU `glibc-2.35.tar.xz`, SHA-256
  `5123732f6b67ccd319305efd399971d58592122bcc2a6518a1bd2510dd0cf52e`.
  On 2026-10-11 its detached signature verified with `gpgv` against the GNU
  keyring (RSA key `16792B4EA25340F8`, Carlos O'Donell). CI checks the SHA-256 only.
- Builder `docker.io/library/debian@sha256:16ee3e11da473f6565ef94a715f1a38b3079e0664913be78869c336a71cb6085` with packages from
  http://archive.debian.org/debian stretch main (signed Release; Valid-Until not enforced): GCC 6.3.0-18+deb9u1, binutils 2.28-5, kernel headers linux-libc-dev 4.9.228-1.
- `--prefix=/usr --libdir=/usr/lib64 --disable-werror --enable-kernel=3.2`; no source patches.
- Compiler runtime: Link: generated libgcc_s stub with GCC 4.8.5's interface; tests run with the real Debian 9 GCC 6.3 libgcc_s.so.1 (requires GLIBC_2.14 at most).

| Library | Public numeric GLIBC definitions |
| --- | ---: |
| c | 2686 |
| m | 1181 |
| ld | 8 |
| resolv | 67 |
| pthread | 12 |
| dl | 3 |
| rt | 5 |
| util | 1 |
| **Total** | **3963** |

All eight native DSOs match GNU readelf and the pinned snapshot with zero ABI
differences. The stub audit preserves measured defaults, compatibility versions,
native types, binding/visibility, object sizes, alignment, and shared storage.
Exhaustive isolated-library checks pass **3963 accepted versioned
interfaces, 708 rejected interfaces, 3259 ordinary default bindings,
and 412 compatibility-only names rejected for new unversioned references**.

## Changes from 2.34

The snapshot adds 100 and removes 0 interfaces in this scope:
Added: `__memcmpeq@GLIBC_2.35` (c), `_dl_find_object@GLIBC_2.35` (c), `epoll_pwait2@GLIBC_2.35` (c), `posix_spawn_file_actions_addtcsetpgrp_np@GLIBC_2.35` (c), `__rseq_flags@GLIBC_2.35` (ld), `__rseq_offset@GLIBC_2.35` (ld), `__rseq_size@GLIBC_2.35` (ld), `__rtld_version_placeholder@GLIBC_2.34` (ld), `dfmal@GLIBC_2.35` (m), `dsqrtl@GLIBC_2.35` (m), `f32fmaf128@GLIBC_2.35` (m), `f32fmaf32x@GLIBC_2.35` (m), `f32fmaf64@GLIBC_2.35` (m), `f32fmaf64x@GLIBC_2.35` (m), `f32sqrtf128@GLIBC_2.35` (m), `f32sqrtf32x@GLIBC_2.35` (m), `f32sqrtf64@GLIBC_2.35` (m), `f32sqrtf64x@GLIBC_2.35` (m), `f32xfmaf128@GLIBC_2.35` (m), `f32xfmaf64@GLIBC_2.35` (m), `f32xfmaf64x@GLIBC_2.35` (m), `f32xsqrtf128@GLIBC_2.35` (m), `f32xsqrtf64@GLIBC_2.35` (m), `f32xsqrtf64x@GLIBC_2.35` (m), `f64fmaf128@GLIBC_2.35` (m), `f64fmaf64x@GLIBC_2.35` (m), `f64sqrtf128@GLIBC_2.35` (m), `f64sqrtf64x@GLIBC_2.35` (m), `f64xfmaf128@GLIBC_2.35` (m), `f64xsqrtf128@GLIBC_2.35` (m), `ffma@GLIBC_2.35` (m), `ffmal@GLIBC_2.35` (m), `fmaximum@GLIBC_2.35` (m), `fmaximum_mag@GLIBC_2.35` (m), `fmaximum_mag_num@GLIBC_2.35` (m), `fmaximum_mag_numf@GLIBC_2.35` (m), `fmaximum_mag_numf128@GLIBC_2.35` (m), `fmaximum_mag_numf32@GLIBC_2.35` (m), `fmaximum_mag_numf32x@GLIBC_2.35` (m), `fmaximum_mag_numf64@GLIBC_2.35` (m), and 60 more.

Measured default roles changed for 2 interfaces (`yes` is the default `@@` version, `no` a compatibility `@` version): `hypot@GLIBC_2.2.5` (m) yes to no, `hypotf@GLIBC_2.2.5` (m) yes to no.
5 interfaces changed between GLOBAL and WEAK binding; the stubs preserve
these, and the exhaustive default checks cover the resulting bindings.

2.35 gave `hypot` and `hypotf` new `GLIBC_2.35` defaults and adds 100 interfaces in this scope.

## Applications and Cargo

Real-runtime tests pass allocation, threads/TLS destruction, unwinding, historical
data views, memcpy and fmemopen compatibility behavior, the pthread default-attribute
and sigvec boundary fixtures (each following this provider's evidence), and the five
dedicated math/loading/realtime/utility/resolver suites. Cargo preserves host build
scripts and proc macros, audits target link/runtime paths, and rejects mismatched
release selection. The cargo-libc build, tier 1 scan, tier 2 local runner under a
Linux 3.10 filter, and `check` all pass. The too-new link tests use `arc4random`,
chosen from the matrix (`too-new-symbol.py`), and fail at the linker as required.
The Rust test program's highest directly imported GLIBC tag was 2.34; this is an
import measurement, not a general minimum-runtime guarantee.

## Reproduction

```bash
export ABI_GLIBC_RELEASE=2.35
export RUSTUP_TOOLCHAIN=1.99.0
export ABI_BEHAVIOR_CC=/usr/bin/clang-24
bash examples/glibc-2.35/run.sh /tmp/glibc-2.35-work
bash examples/glibc-two-paths/elf-and-check.sh /tmp/glibc-2.35-work
bash examples/glibc-two-paths/prepare-and-check.sh /tmp/glibc-2.35-work
bash examples/glibc-two-paths/application-and-check.sh /tmp/glibc-2.35-work
bash examples/glibc-two-paths/provider-and-check.sh \
  /tmp/glibc-2.35-work ../cargo-libc /usr/bin/clang-24
```

Use a new absolute work directory. Local validation on 2026-10-11 used Rust 1.99.0,
Clang 18.1.3, GNU binutils 2.42, and podman 5.8.2 on WSL Ubuntu 24.04, with
cargo-libc `bfe04693f6685770b47c9346e32de556a8029f4e`. The measured profile `gnu-source-2.35` is
committed to [history provenance](../data/glibc-history/PROVENANCE.json) and
[the interface reference](reference/GLIBC-SYMBOLS.md).

Both Clang 21/24 jobs for 2.35 passed in the [complete 84-job hosted run](https://github.com/rustcommons/linux-abi-tools/actions/runs/38158161339) at
producer `ae68bf63f5ddde7fc7852e76eab83ad2c1beb2c0` with consumer `bfe04693f6685770b47c9346e32de556a8029f4e` on 2026-10-11. Distribution/kernel
qualification, native dependency adapters, and authenticated SDK packaging remain open.

# glibc 2.26 provider

The experimental x86_64 Linux provider selects GNU glibc 2.26 explicitly.
Matching headers, CRT objects, nonshared archives, loader, and runtime DSOs come
from a pinned source build. This is a generic GNU environment; distribution and
old-kernel qualification remain separate tasks.

Inputs are pinned in [INPUTS.toml](../examples/glibc-2.26/INPUTS.toml):

- Official GNU `glibc-2.26.tar.xz`, SHA-256
  `e54e0a934cd2bc94429be79da5e9385898d2306b9eaf3c92d5a77af96190f6bd`.
  On 2026-10-11 its detached signature verified with `gpgv` against the GNU
  keyring (RSA key `79C43DFBF1CF2187`, Siddhesh Poyarekar). CI checks the SHA-256 only.
- Builder `docker.io/library/debian@sha256:16ee3e11da473f6565ef94a715f1a38b3079e0664913be78869c336a71cb6085` with packages from
  http://archive.debian.org/debian stretch main (signed Release; Valid-Until not enforced): GCC 6.3.0-18+deb9u1, binutils 2.28-5, kernel headers linux-libc-dev 4.9.228-1.
- `--prefix=/usr --libdir=/usr/lib64 --disable-werror --enable-kernel=3.2`; no source patches.
- Compiler runtime: Link: generated libgcc_s stub with GCC 4.8.5's interface; tests run with the real Debian 9 GCC 6.3 libgcc_s.so.1 (requires GLIBC_2.14 at most).

| Library | Public numeric GLIBC definitions |
| --- | ---: |
| c | 2152 |
| m | 595 |
| ld | 8 |
| resolv | 91 |
| pthread | 235 |
| dl | 9 |
| rt | 47 |
| util | 6 |
| **Total** | **3143** |

All eight native DSOs match GNU readelf and the pinned snapshot with zero ABI
differences. The stub audit preserves measured defaults, compatibility versions,
native types, binding/visibility, object sizes, alignment, and shared storage.
Exhaustive isolated-library checks pass **3143 accepted versioned
interfaces, 1528 rejected interfaces, 2878 ordinary default bindings,
and 213 compatibility-only names rejected for new unversioned references**.

## Changes from 2.25

The snapshot adds 149 and removes 0 interfaces in this scope:
Added: `__strtof128_internal@GLIBC_2.26` (c), `__wcstof128_internal@GLIBC_2.26` (c), `preadv2@GLIBC_2.26` (c), `preadv64v2@GLIBC_2.26` (c), `pwritev2@GLIBC_2.26` (c), `pwritev64v2@GLIBC_2.26` (c), `reallocarray@GLIBC_2.26` (c), `strfromf128@GLIBC_2.26` (c), `strtof128@GLIBC_2.26` (c), `strtof128_l@GLIBC_2.26` (c), `wcstof128@GLIBC_2.26` (c), `wcstof128_l@GLIBC_2.26` (c), `__acosf128_finite@GLIBC_2.26` (m), `__acoshf128_finite@GLIBC_2.26` (m), `__asinf128_finite@GLIBC_2.26` (m), `__atan2f128_finite@GLIBC_2.26` (m), `__atanhf128_finite@GLIBC_2.26` (m), `__coshf128_finite@GLIBC_2.26` (m), `__exp10f128_finite@GLIBC_2.26` (m), `__exp2f128_finite@GLIBC_2.26` (m), `__expf128_finite@GLIBC_2.26` (m), `__finitef128@GLIBC_2.26` (m), `__fmodf128_finite@GLIBC_2.26` (m), `__fpclassifyf128@GLIBC_2.26` (m), `__gammaf128_r_finite@GLIBC_2.26` (m), `__hypotf128_finite@GLIBC_2.26` (m), `__iseqsigf128@GLIBC_2.26` (m), `__isinff128@GLIBC_2.26` (m), `__isnanf128@GLIBC_2.26` (m), `__issignalingf128@GLIBC_2.26` (m), `__j0f128_finite@GLIBC_2.26` (m), `__j1f128_finite@GLIBC_2.26` (m), `__jnf128_finite@GLIBC_2.26` (m), `__lgammaf128_r_finite@GLIBC_2.26` (m), `__log10f128_finite@GLIBC_2.26` (m), `__log2f128_finite@GLIBC_2.26` (m), `__logf128_finite@GLIBC_2.26` (m), `__powf128_finite@GLIBC_2.26` (m), `__remainderf128_finite@GLIBC_2.26` (m), `__signbitf128@GLIBC_2.26` (m), and 109 more.

Measured default roles changed for 4 interfaces (`yes` is the default `@@` version, `no` a compatibility `@` version): `__sigaddset@GLIBC_2.2.5` (c) yes to no, `__sigdelset@GLIBC_2.2.5` (c) yes to no, `__sigismember@GLIBC_2.2.5` (c) yes to no, `cfree@GLIBC_2.2.5` (c) yes to no.
Measured symbol types changed for `__rawmemchr@GLIBC_2.2.5` (c) FUNC to IFUNC, `__wmemset_chk@GLIBC_2.4` (c) FUNC to IFUNC, `memchr@GLIBC_2.2.5` (c) FUNC to IFUNC, `memrchr@GLIBC_2.2.5` (c) FUNC to IFUNC, `rawmemchr@GLIBC_2.2.5` (c) FUNC to IFUNC, `rindex@GLIBC_2.2.5` (c) FUNC to IFUNC, `strchrnul@GLIBC_2.2.5` (c) FUNC to IFUNC, `strlen@GLIBC_2.2.5` (c) FUNC to IFUNC, `strnlen@GLIBC_2.2.5` (c) FUNC to IFUNC, `strrchr@GLIBC_2.2.5` (c) FUNC to IFUNC, `wcschr@GLIBC_2.2.5` (c) FUNC to IFUNC, `wcslen@GLIBC_2.2.5` (c) FUNC to IFUNC, `wcsnlen@GLIBC_2.2.5` (c) FUNC to IFUNC, `wcsrchr@GLIBC_2.2.5` (c) FUNC to IFUNC, `wmemchr@GLIBC_2.2.5` (c) FUNC to IFUNC, `wmemset@GLIBC_2.2.5` (c) FUNC to IFUNC.
33 interfaces changed between GLOBAL and WEAK binding; the stubs preserve
these, and the exhaustive default checks cover the resulting bindings.

2.26 needs GCC 4.9 and binutils 2.25, newer than CentOS 7 provides, so this provider introduces the
digest-pinned Debian 9 builder with archived, signature-checked stretch packages (GCC 6.3, binutils
2.28, make 4.1, Python 3.5, Linux 4.9 UAPI headers) and its real GCC 6.3 unwinder, which requires at
most `GLIBC_2.14`. The configuration is unchanged. Sixteen string functions became IFUNC symbols, and
`cfree` and the `__sig*set` helpers became compatibility-only.

## Applications and Cargo

Real-runtime tests pass allocation, threads/TLS destruction, unwinding, historical
data views, memcpy and fmemopen compatibility behavior, the pthread default-attribute
and sigvec boundary fixtures (each following this provider's evidence), and the five
dedicated math/loading/realtime/utility/resolver suites. Cargo preserves host build
scripts and proc macros, audits target link/runtime paths, and rejects mismatched
release selection. The cargo-libc build, tier 1 scan, tier 2 local runner under a
Linux 3.10 filter, and `check` all pass. The too-new link tests use `memfd_create`,
chosen from the matrix (`too-new-symbol.py`), and fail at the linker as required.
The Rust test program's highest directly imported GLIBC tag was 2.18; this is an
import measurement, not a general minimum-runtime guarantee.

## Reproduction

```bash
export ABI_GLIBC_RELEASE=2.26
export RUSTUP_TOOLCHAIN=1.99.0
export ABI_BEHAVIOR_CC=/usr/bin/clang-24
bash examples/glibc-2.26/run.sh /tmp/glibc-2.26-work
bash examples/glibc-two-paths/elf-and-check.sh /tmp/glibc-2.26-work
bash examples/glibc-two-paths/prepare-and-check.sh /tmp/glibc-2.26-work
bash examples/glibc-two-paths/application-and-check.sh /tmp/glibc-2.26-work
bash examples/glibc-two-paths/provider-and-check.sh \
  /tmp/glibc-2.26-work ../cargo-libc /usr/bin/clang-24
```

Use a new absolute work directory. Local validation on 2026-10-11 used Rust 1.99.0,
Clang 18.1.3, GNU binutils 2.42, and podman 5.8.2 on WSL Ubuntu 24.04, with
cargo-libc `a259c167741c6b1d31eaa3b7c664df8faff3b7a7`. The measured profile `gnu-source-2.26` is
committed to [history provenance](../data/glibc-history/PROVENANCE.json) and
[the interface reference](reference/GLIBC-SYMBOLS.md).

Both Clang 21/24 jobs for 2.26 passed in the [hosted provider run](https://github.com/rustcommons/linux-abi-tools/actions/runs/38155189880) at producer
`27caf3dbf9b7b5b018763629d3a3d2ebba377123` with consumer `a259c167741c6b1d31eaa3b7c664df8faff3b7a7`,
and again in the [complete 84-job run](https://github.com/rustcommons/linux-abi-tools/actions/runs/38158161339) at producer `ae68bf63f5ddde7fc7852e76eab83ad2c1beb2c0`
with consumer `bfe04693f6685770b47c9346e32de556a8029f4e` on 2026-10-11. Distribution/kernel
qualification, native dependency adapters, and authenticated SDK packaging remain open.

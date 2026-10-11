# glibc 2.38 provider

The experimental x86_64 Linux provider selects GNU glibc 2.38 explicitly.
Matching headers, CRT objects, nonshared archives, loader, and runtime DSOs come
from a pinned source build. This is a generic GNU environment; distribution and
old-kernel qualification remain separate tasks.

Inputs are pinned in [INPUTS.toml](../examples/glibc-2.38/INPUTS.toml):

- Official GNU `glibc-2.38.tar.xz`, SHA-256
  `fb82998998b2b29965467bc1b69d152e9c307d2cf301c9eafb4555b770ef3fd2`.
  On 2026-10-11 its detached signature verified with `gpgv` against the GNU
  keyring (RSA key `16792B4EA25340F8`, Carlos O'Donell). CI checks the SHA-256 only.
- Builder `docker.io/library/debian@sha256:bc49dc1918ee1a47a93e65b5e4676e8680fb754b133197b92ca52bfe6731d5f0` with packages from
  http://snapshot.debian.org/archive/debian/20260901T000000Z bookworm main (signed Release; Valid-Until not enforced): GCC 12.2.0-14+deb12u1, binutils 2.40-2, kernel headers linux-libc-dev 6.1.176-1.
- `--prefix=/usr --libdir=/usr/lib64 --disable-werror --enable-kernel=3.2`; no source patches.
- Compiler runtime: Real Debian 12 GCC 12.2 libgcc_s.so.1 (requires GLIBC_2.35 at most).

| Library | Public numeric GLIBC definitions |
| --- | ---: |
| c | 2743 |
| m | 1183 |
| ld | 8 |
| resolv | 67 |
| pthread | 12 |
| dl | 3 |
| rt | 5 |
| util | 1 |
| **Total** | **4022** |

All eight native DSOs match GNU readelf and the pinned snapshot with zero ABI
differences. The stub audit preserves measured defaults, compatibility versions,
native types, binding/visibility, object sizes, alignment, and shared storage.
Exhaustive isolated-library checks pass **4022 accepted versioned
interfaces, 649 rejected interfaces, 3315 ordinary default bindings,
and 413 compatibility-only names rejected for new unversioned references**.

## Changes from 2.37

The snapshot adds 42 and removes 0 interfaces in this scope:
Added: `__isoc23_fscanf@GLIBC_2.38` (c), `__isoc23_fwscanf@GLIBC_2.38` (c), `__isoc23_scanf@GLIBC_2.38` (c), `__isoc23_sscanf@GLIBC_2.38` (c), `__isoc23_strtoimax@GLIBC_2.38` (c), `__isoc23_strtol@GLIBC_2.38` (c), `__isoc23_strtol_l@GLIBC_2.38` (c), `__isoc23_strtoll@GLIBC_2.38` (c), `__isoc23_strtoll_l@GLIBC_2.38` (c), `__isoc23_strtoul@GLIBC_2.38` (c), `__isoc23_strtoul_l@GLIBC_2.38` (c), `__isoc23_strtoull@GLIBC_2.38` (c), `__isoc23_strtoull_l@GLIBC_2.38` (c), `__isoc23_strtoumax@GLIBC_2.38` (c), `__isoc23_swscanf@GLIBC_2.38` (c), `__isoc23_vfscanf@GLIBC_2.38` (c), `__isoc23_vfwscanf@GLIBC_2.38` (c), `__isoc23_vscanf@GLIBC_2.38` (c), `__isoc23_vsscanf@GLIBC_2.38` (c), `__isoc23_vswscanf@GLIBC_2.38` (c), `__isoc23_vwscanf@GLIBC_2.38` (c), `__isoc23_wcstoimax@GLIBC_2.38` (c), `__isoc23_wcstol@GLIBC_2.38` (c), `__isoc23_wcstol_l@GLIBC_2.38` (c), `__isoc23_wcstoll@GLIBC_2.38` (c), `__isoc23_wcstoll_l@GLIBC_2.38` (c), `__isoc23_wcstoul@GLIBC_2.38` (c), `__isoc23_wcstoul_l@GLIBC_2.38` (c), `__isoc23_wcstoull@GLIBC_2.38` (c), `__isoc23_wcstoull_l@GLIBC_2.38` (c), `__isoc23_wcstoumax@GLIBC_2.38` (c), `__isoc23_wscanf@GLIBC_2.38` (c), `__strlcat_chk@GLIBC_2.38` (c), `__strlcpy_chk@GLIBC_2.38` (c), `__wcslcat_chk@GLIBC_2.38` (c), `__wcslcpy_chk@GLIBC_2.38` (c), `strlcat@GLIBC_2.38` (c), `strlcpy@GLIBC_2.38` (c), `wcslcat@GLIBC_2.38` (c), `wcslcpy@GLIBC_2.38` (c), and 2 more.

Measured default roles changed for 2 interfaces (`yes` is the default `@@` version, `no` a compatibility `@` version): `fmod@GLIBC_2.2.5` (m) yes to no, `fmodf@GLIBC_2.2.5` (m) yes to no.
2 interfaces changed between GLOBAL and WEAK binding; the stubs preserve
these, and the exhaustive default checks cover the resulting bindings.

2.38 added the `__isoc23_*` scanf/strtol family. C code compiled against these headers with
`_GNU_SOURCE` or a C2X/C23 standard is redirected to them and therefore requires glibc 2.38; Rust code
does not use the C headers and is unaffected. `fmod`/`fmodf` gained new `GLIBC_2.38` defaults.

## Applications and Cargo

Real-runtime tests pass allocation, threads/TLS destruction, unwinding, historical
data views, memcpy and fmemopen compatibility behavior, the pthread default-attribute
and sigvec boundary fixtures (each following this provider's evidence), and the five
dedicated math/loading/realtime/utility/resolver suites. Cargo preserves host build
scripts and proc macros, audits target link/runtime paths, and rejects mismatched
release selection. The cargo-libc build, tier 1 scan, tier 2 local runner under a
Linux 3.10 filter, and `check` all pass. The too-new link tests use `pidfd_getpid`,
chosen from the matrix (`too-new-symbol.py`), and fail at the linker as required.
The Rust test program's highest directly imported GLIBC tag was 2.34; this is an
import measurement, not a general minimum-runtime guarantee.

## Reproduction

```bash
export ABI_GLIBC_RELEASE=2.38
export RUSTUP_TOOLCHAIN=1.99.0
export ABI_BEHAVIOR_CC=/usr/bin/clang-24
bash examples/glibc-2.38/run.sh /tmp/glibc-2.38-work
bash examples/glibc-two-paths/elf-and-check.sh /tmp/glibc-2.38-work
bash examples/glibc-two-paths/prepare-and-check.sh /tmp/glibc-2.38-work
bash examples/glibc-two-paths/application-and-check.sh /tmp/glibc-2.38-work
bash examples/glibc-two-paths/provider-and-check.sh \
  /tmp/glibc-2.38-work ../cargo-libc /usr/bin/clang-24
```

Use a new absolute work directory. Local validation on 2026-10-11 used Rust 1.99.0,
Clang 18.1.3, GNU binutils 2.42, and podman 5.8.2 on WSL Ubuntu 24.04, with
cargo-libc `bfe04693f6685770b47c9346e32de556a8029f4e`. The measured profile `gnu-source-2.38` is
committed to [history provenance](../data/glibc-history/PROVENANCE.json) and
[the interface reference](reference/GLIBC-SYMBOLS.md).

Hosted Clang 21/24 jobs for 2.38 are configured in the provider workflow;
hosted qualification is recorded here once they pass. Distribution/kernel
qualification, native dependency adapters, and authenticated SDK packaging remain open.

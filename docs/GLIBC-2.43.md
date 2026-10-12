# glibc 2.43 provider

The experimental x86_64 Linux provider selects GNU glibc 2.43 explicitly.
Matching headers, CRT objects, nonshared archives, loader, and runtime DSOs come
from a pinned source build. This is a generic GNU environment; distribution and
old-kernel qualification remain separate tasks.

Inputs are pinned in [INPUTS.toml](../examples/glibc-2.43/INPUTS.toml):

- Official GNU `glibc-2.43.tar.xz`, SHA-256
  `d9c86c6b5dbddb43a3e08270c5844fc5177d19442cf5b8df4be7c07cd5fa3831`.
  On 2026-10-11 its detached signature verified with `gpgv` against the GNU
  keyring (RSA key `DC2B16215ED5412A`, Andreas K. Huettel). CI checks the SHA-256 only.
- Builder `docker.io/library/debian@sha256:bc49dc1918ee1a47a93e65b5e4676e8680fb754b133197b92ca52bfe6731d5f0` with packages from
  http://snapshot.debian.org/archive/debian/20260901T000000Z bookworm main (signed Release; Valid-Until not enforced): GCC 12.2.0-14+deb12u1, binutils 2.40-2, kernel headers linux-libc-dev 6.1.176-1.
- `--prefix=/usr --libdir=/usr/lib64 --disable-werror --enable-kernel=3.2`; no source patches.
- Compiler runtime: Link: generated libgcc_s stub with GCC 4.8.5's interface; tests run with the real Debian 12 GCC 12.2 libgcc_s.so.1 (requires GLIBC_2.35 at most).

| Library | Public numeric GLIBC definitions |
| --- | ---: |
| c | 2845 |
| m | 1372 |
| ld | 8 |
| resolv | 67 |
| pthread | 12 |
| dl | 3 |
| rt | 5 |
| util | 1 |
| **Total** | **4313** |

All eight native DSOs match GNU readelf and the pinned snapshot with zero ABI
differences. The stub audit preserves measured defaults, compatibility versions,
native types, binding/visibility, object sizes, alignment, and shared storage.
Exhaustive isolated-library checks pass **4313 accepted versioned
interfaces, 358 rejected interfaces, 3545 ordinary default bindings,
and 416 compatibility-only names rejected for new unversioned references**.

## Changes from 2.42

The snapshot adds 60 and removes 0 interfaces in this scope:
Added: `__memset_explicit_chk@GLIBC_2.43` (c), `free_aligned_sized@GLIBC_2.43` (c), `free_sized@GLIBC_2.43` (c), `memalignment@GLIBC_2.43` (c), `memset_explicit@GLIBC_2.43` (c), `mseal@GLIBC_2.43` (c), `openat2@GLIBC_2.43` (c), `umaxabs@GLIBC_2.43` (c), `acosf@GLIBC_2.43` (m), `acoshf@GLIBC_2.43` (m), `asinf@GLIBC_2.43` (m), `atan2f@GLIBC_2.43` (m), `atanhf@GLIBC_2.43` (m), `coshf@GLIBC_2.43` (m), `fromfp@GLIBC_2.43` (m), `fromfpf@GLIBC_2.43` (m), `fromfpf128@GLIBC_2.43` (m), `fromfpf32@GLIBC_2.43` (m), `fromfpf32x@GLIBC_2.43` (m), `fromfpf64@GLIBC_2.43` (m), `fromfpf64x@GLIBC_2.43` (m), `fromfpl@GLIBC_2.43` (m), `fromfpx@GLIBC_2.43` (m), `fromfpxf@GLIBC_2.43` (m), `fromfpxf128@GLIBC_2.43` (m), `fromfpxf32@GLIBC_2.43` (m), `fromfpxf32x@GLIBC_2.43` (m), `fromfpxf64@GLIBC_2.43` (m), `fromfpxf64x@GLIBC_2.43` (m), `fromfpxl@GLIBC_2.43` (m), `j0f@GLIBC_2.43` (m), `j1f@GLIBC_2.43` (m), `jnf@GLIBC_2.43` (m), `lgammaf@GLIBC_2.43` (m), `lgammaf_r@GLIBC_2.43` (m), `log10f@GLIBC_2.43` (m), `remainder@GLIBC_2.43` (m), `remainderf@GLIBC_2.43` (m), `sinhf@GLIBC_2.43` (m), `sqrtf@GLIBC_2.43` (m), and 20 more.

Measured default roles changed for 53 interfaces (`yes` is the default `@@` version, `no` a compatibility `@` version): `uimaxabs@GLIBC_2.42` (c) yes to no, `acosf@GLIBC_2.2.5` (m) yes to no, `acoshf@GLIBC_2.2.5` (m) yes to no, `asinf@GLIBC_2.2.5` (m) yes to no, `atan2f@GLIBC_2.2.5` (m) yes to no, `atanhf@GLIBC_2.2.5` (m) yes to no, `coshf@GLIBC_2.2.5` (m) yes to no, `fromfp@GLIBC_2.25` (m) yes to no, `fromfpf@GLIBC_2.25` (m) yes to no, `fromfpf128@GLIBC_2.26` (m) yes to no, `fromfpf32@GLIBC_2.27` (m) yes to no, `fromfpf32x@GLIBC_2.27` (m) yes to no, `fromfpf64@GLIBC_2.27` (m) yes to no, `fromfpf64x@GLIBC_2.27` (m) yes to no, `fromfpl@GLIBC_2.25` (m) yes to no, `fromfpx@GLIBC_2.25` (m) yes to no, `fromfpxf@GLIBC_2.25` (m) yes to no, `fromfpxf128@GLIBC_2.26` (m) yes to no, `fromfpxf32@GLIBC_2.27` (m) yes to no, `fromfpxf32x@GLIBC_2.27` (m) yes to no, `fromfpxf64@GLIBC_2.27` (m) yes to no, `fromfpxf64x@GLIBC_2.27` (m) yes to no, `fromfpxl@GLIBC_2.25` (m) yes to no, `j0f@GLIBC_2.2.5` (m) yes to no, `j1f@GLIBC_2.2.5` (m) yes to no, `jnf@GLIBC_2.2.5` (m) yes to no, `lgammaf@GLIBC_2.23` (m) yes to no, `lgammaf_r@GLIBC_2.2.5` (m) yes to no, `log10f@GLIBC_2.2.5` (m) yes to no, `remainder@GLIBC_2.2.5` (m) yes to no, `remainderf@GLIBC_2.2.5` (m) yes to no, `sinhf@GLIBC_2.2.5` (m) yes to no, `sqrtf@GLIBC_2.2.5` (m) yes to no, `tgammaf@GLIBC_2.2.5` (m) yes to no, `ufromfp@GLIBC_2.25` (m) yes to no, `ufromfpf@GLIBC_2.25` (m) yes to no, `ufromfpf128@GLIBC_2.26` (m) yes to no, `ufromfpf32@GLIBC_2.27` (m) yes to no, `ufromfpf32x@GLIBC_2.27` (m) yes to no, `ufromfpf64@GLIBC_2.27` (m) yes to no, and 13 more.
99 interfaces changed between GLOBAL and WEAK binding; the stubs preserve
these, and the exhaustive default checks cover the resulting bindings.

2.43 gave 53 interfaces new defaults, most of them single-precision libm functions (`acosf`,
`asinf`, `atan2f`, `coshf`, and others) and the `fromfp` families, and added `free_sized`,
`free_aligned_sized`, `memset_explicit`, `mseal`, and `openat2`. Its source was signed on 2026-01-23.
The only later release, 2.44, adds no libc function, so the too-new tests use a name absent from
every snapshot.

## Applications and Cargo

Real-runtime tests pass allocation, threads/TLS destruction, unwinding, historical
data views, memcpy and fmemopen compatibility behavior, the pthread default-attribute
and sigvec boundary fixtures (each following this provider's evidence), and the five
dedicated math/loading/realtime/utility/resolver suites. Cargo preserves host build
scripts and proc macros, audits target link/runtime paths, and rejects mismatched
release selection. The cargo-libc build, tier 1 scan, tier 2 local runner under a
Linux 3.10 filter, and `check` all pass. The too-new link tests use `glibc_abi_tools_absent_probe`,
chosen from the matrix (`too-new-symbol.py`), and fail at the linker as required.
The Rust test program's highest directly imported GLIBC tag was 2.34; this is an
import measurement, not a general minimum-runtime guarantee.

## Reproduction

```bash
export ABI_GLIBC_RELEASE=2.43
export RUSTUP_TOOLCHAIN=1.99.0
export ABI_BEHAVIOR_CC=/usr/bin/clang-24
bash examples/glibc-2.43/run.sh /tmp/glibc-2.43-work
bash examples/glibc-two-paths/elf-and-check.sh /tmp/glibc-2.43-work
bash examples/glibc-two-paths/prepare-and-check.sh /tmp/glibc-2.43-work
bash examples/glibc-two-paths/application-and-check.sh /tmp/glibc-2.43-work
bash examples/glibc-two-paths/provider-and-check.sh \
  /tmp/glibc-2.43-work ../cargo-libc /usr/bin/clang-24
```

Use a new absolute work directory. Local validation on 2026-10-11 used Rust 1.99.0,
Clang 18.1.3, GNU binutils 2.42, and podman 5.8.2 on WSL Ubuntu 24.04, with
cargo-libc `bfe04693f6685770b47c9346e32de556a8029f4e`. The measured profile `gnu-source-2.43` is
committed to [history provenance](../data/glibc-history/PROVENANCE.json) and
[the interface reference](reference/GLIBC-SYMBOLS.md).

Both Clang 21/24 jobs for 2.43 passed in the [complete 84-job hosted run](https://github.com/rustcommons/linux-abi-tools/actions/runs/38158161339) at
producer `ae68bf63f5ddde7fc7852e76eab83ad2c1beb2c0` with consumer `bfe04693f6685770b47c9346e32de556a8029f4e` on 2026-10-11. Distribution/kernel
qualification, native dependency adapters, and authenticated SDK packaging remain open.

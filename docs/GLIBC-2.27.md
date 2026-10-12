# glibc 2.27 provider

The experimental x86_64 Linux provider selects GNU glibc 2.27 explicitly.
Matching headers, CRT objects, nonshared archives, loader, and runtime DSOs come
from a pinned source build. This is a generic GNU environment; distribution and
old-kernel qualification remain separate tasks.

Inputs are pinned in [INPUTS.toml](../examples/glibc-2.27/INPUTS.toml):

- Official GNU `glibc-2.27.tar.xz`, SHA-256
  `5172de54318ec0b7f2735e5a91d908afe1c9ca291fec16b5374d9faadfc1fc72`.
  On 2026-10-11 its detached signature verified with `gpgv` against the GNU
  keyring (RSA key `16792B4EA25340F8`, Carlos O'Donell). CI checks the SHA-256 only.
- Builder `docker.io/library/debian@sha256:16ee3e11da473f6565ef94a715f1a38b3079e0664913be78869c336a71cb6085` with packages from
  http://archive.debian.org/debian stretch main (signed Release; Valid-Until not enforced): GCC 6.3.0-18+deb9u1, binutils 2.28-5, kernel headers linux-libc-dev 4.9.228-1.
- `--prefix=/usr --libdir=/usr/lib64 --disable-werror --enable-kernel=3.2`; no source patches.
- Compiler runtime: Link: generated libgcc_s stub with GCC 4.8.5's interface; tests run with the real Debian 9 GCC 6.3 libgcc_s.so.1 (requires GLIBC_2.14 at most).

| Library | Public numeric GLIBC definitions |
| --- | ---: |
| c | 2182 |
| m | 1016 |
| ld | 8 |
| resolv | 91 |
| pthread | 235 |
| dl | 9 |
| rt | 47 |
| util | 6 |
| **Total** | **3594** |

All eight native DSOs match GNU readelf and the pinned snapshot with zero ABI
differences. The stub audit preserves measured defaults, compatibility versions,
native types, binding/visibility, object sizes, alignment, and shared storage.
Exhaustive isolated-library checks pass **3594 accepted versioned
interfaces, 1077 rejected interfaces, 3312 ordinary default bindings,
and 223 compatibility-only names rejected for new unversioned references**.

## Changes from 2.26

The snapshot adds 451 and removes 0 interfaces in this scope:
Added: `copy_file_range@GLIBC_2.27` (c), `glob@GLIBC_2.27` (c), `glob64@GLIBC_2.27` (c), `memfd_create@GLIBC_2.27` (c), `mlock2@GLIBC_2.27` (c), `pkey_alloc@GLIBC_2.27` (c), `pkey_free@GLIBC_2.27` (c), `pkey_get@GLIBC_2.27` (c), `pkey_mprotect@GLIBC_2.27` (c), `pkey_set@GLIBC_2.27` (c), `strfromf32@GLIBC_2.27` (c), `strfromf32x@GLIBC_2.27` (c), `strfromf64@GLIBC_2.27` (c), `strfromf64x@GLIBC_2.27` (c), `strtof32@GLIBC_2.27` (c), `strtof32_l@GLIBC_2.27` (c), `strtof32x@GLIBC_2.27` (c), `strtof32x_l@GLIBC_2.27` (c), `strtof64@GLIBC_2.27` (c), `strtof64_l@GLIBC_2.27` (c), `strtof64x@GLIBC_2.27` (c), `strtof64x_l@GLIBC_2.27` (c), `wcstof32@GLIBC_2.27` (c), `wcstof32_l@GLIBC_2.27` (c), `wcstof32x@GLIBC_2.27` (c), `wcstof32x_l@GLIBC_2.27` (c), `wcstof64@GLIBC_2.27` (c), `wcstof64_l@GLIBC_2.27` (c), `wcstof64x@GLIBC_2.27` (c), `wcstof64x_l@GLIBC_2.27` (c), `acosf32@GLIBC_2.27` (m), `acosf32x@GLIBC_2.27` (m), `acosf64@GLIBC_2.27` (m), `acosf64x@GLIBC_2.27` (m), `acoshf32@GLIBC_2.27` (m), `acoshf32x@GLIBC_2.27` (m), `acoshf64@GLIBC_2.27` (m), `acoshf64x@GLIBC_2.27` (m), `asinf32@GLIBC_2.27` (m), `asinf32x@GLIBC_2.27` (m), and 411 more.

Measured default roles changed for 17 interfaces (`yes` is the default `@@` version, `no` a compatibility `@` version): `__nss_group_lookup@GLIBC_2.2.5` (c) yes to no, `__nss_hosts_lookup@GLIBC_2.2.5` (c) yes to no, `__nss_passwd_lookup@GLIBC_2.2.5` (c) yes to no, `glob@GLIBC_2.2.5` (c) yes to no, `glob64@GLIBC_2.2.5` (c) yes to no, `_LIB_VERSION@GLIBC_2.2.5` (m) yes to no, `exp2f@GLIBC_2.2.5` (m) yes to no, `expf@GLIBC_2.2.5` (m) yes to no, `log2f@GLIBC_2.2.5` (m) yes to no, `logf@GLIBC_2.2.5` (m) yes to no, `matherr@GLIBC_2.2.5` (m) yes to no, `pow10@GLIBC_2.2.5` (m) yes to no, `pow10f@GLIBC_2.2.5` (m) yes to no, `pow10l@GLIBC_2.2.5` (m) yes to no, `powf@GLIBC_2.2.5` (m) yes to no, `pthread_self@GLIBC_2.2.5` (pthread) yes to no, `__p_secstodate@GLIBC_2.2.5` (resolv) yes to no.
Measured symbol types changed for `__exp2f_finite@GLIBC_2.15` (m) FUNC to IFUNC, `__expf_finite@GLIBC_2.15` (m) FUNC to IFUNC, `__log2f_finite@GLIBC_2.15` (m) FUNC to IFUNC, `__logf_finite@GLIBC_2.15` (m) FUNC to IFUNC, `__powf_finite@GLIBC_2.15` (m) FUNC to IFUNC, `cosf@GLIBC_2.2.5` (m) FUNC to IFUNC, `sincosf@GLIBC_2.2.5` (m) FUNC to IFUNC, `sinf@GLIBC_2.2.5` (m) FUNC to IFUNC, `trunc@GLIBC_2.2.5` (m) FUNC to IFUNC, `truncf@GLIBC_2.2.5` (m) FUNC to IFUNC, `longjmp@GLIBC_2.2.5` (pthread) IFUNC to FUNC, `siglongjmp@GLIBC_2.2.5` (pthread) IFUNC to FUNC, `system@GLIBC_2.2.5` (pthread) IFUNC to FUNC.
70 interfaces changed between GLOBAL and WEAK binding; the stubs preserve
these, and the exhaustive default checks cover the resulting bindings.

2.27 gave `expf`, `exp2f`, `logf`, `log2f`, `powf` and `glob`/`glob64` new `GLIBC_2.27` defaults; the old
versions remain for existing binaries. A program built against this provider that calls them (for
example Rust's `f32::exp`) therefore requires glibc 2.27, which is the correct result. `memfd_create`
first appears in 2.27, so the too-new tests switch to the matrix-selected `fcntl64` (2.28).

## Applications and Cargo

Real-runtime tests pass allocation, threads/TLS destruction, unwinding, historical
data views, memcpy and fmemopen compatibility behavior, the pthread default-attribute
and sigvec boundary fixtures (each following this provider's evidence), and the five
dedicated math/loading/realtime/utility/resolver suites. Cargo preserves host build
scripts and proc macros, audits target link/runtime paths, and rejects mismatched
release selection. The cargo-libc build, tier 1 scan, tier 2 local runner under a
Linux 3.10 filter, and `check` all pass. The too-new link tests use `fcntl64`,
chosen from the matrix (`too-new-symbol.py`), and fail at the linker as required.
The Rust test program's highest directly imported GLIBC tag was 2.18; this is an
import measurement, not a general minimum-runtime guarantee.

## Reproduction

```bash
export ABI_GLIBC_RELEASE=2.27
export RUSTUP_TOOLCHAIN=1.99.0
export ABI_BEHAVIOR_CC=/usr/bin/clang-24
bash examples/glibc-2.27/run.sh /tmp/glibc-2.27-work
bash examples/glibc-two-paths/elf-and-check.sh /tmp/glibc-2.27-work
bash examples/glibc-two-paths/prepare-and-check.sh /tmp/glibc-2.27-work
bash examples/glibc-two-paths/application-and-check.sh /tmp/glibc-2.27-work
bash examples/glibc-two-paths/provider-and-check.sh \
  /tmp/glibc-2.27-work ../cargo-libc /usr/bin/clang-24
```

Use a new absolute work directory. Local validation on 2026-10-11 used Rust 1.99.0,
Clang 18.1.3, GNU binutils 2.42, and podman 5.8.2 on WSL Ubuntu 24.04, with
cargo-libc `a259c167741c6b1d31eaa3b7c664df8faff3b7a7`. The measured profile `gnu-source-2.27` is
committed to [history provenance](../data/glibc-history/PROVENANCE.json) and
[the interface reference](reference/GLIBC-SYMBOLS.md).

Both Clang 21/24 jobs for 2.27 passed in the [hosted provider run](https://github.com/rustcommons/linux-abi-tools/actions/runs/38155189880) at producer
`27caf3dbf9b7b5b018763629d3a3d2ebba377123` with consumer `a259c167741c6b1d31eaa3b7c664df8faff3b7a7`,
and again in the [complete 84-job run](https://github.com/rustcommons/linux-abi-tools/actions/runs/38158161339) at producer `ae68bf63f5ddde7fc7852e76eab83ad2c1beb2c0`
with consumer `bfe04693f6685770b47c9346e32de556a8029f4e` on 2026-10-11. Distribution/kernel
qualification, native dependency adapters, and authenticated SDK packaging remain open.

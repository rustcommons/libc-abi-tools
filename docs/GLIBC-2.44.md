# glibc 2.44 provider

The experimental x86_64 Linux provider selects GNU glibc 2.44 explicitly.
Matching headers, CRT objects, nonshared archives, loader, and runtime DSOs come
from a pinned source build. This is a generic GNU environment; distribution and
old-kernel qualification remain separate tasks.

Inputs are pinned in [INPUTS.toml](../examples/glibc-2.44/INPUTS.toml):

- Official GNU `glibc-2.44.tar.xz`, SHA-256
  `37f600f2bef3c5e8300147059568b2a2e40a7ad6ccc65ce942556d49429cc667`.
  On 2026-10-11 its detached signature verified with `gpgv` against the GNU
  keyring (RSA key `DC2B16215ED5412A`, Andreas K. Huettel). CI checks the SHA-256 only.
- Builder `docker.io/library/debian@sha256:bc49dc1918ee1a47a93e65b5e4676e8680fb754b133197b92ca52bfe6731d5f0` with packages from
  http://snapshot.debian.org/archive/debian/20260901T000000Z bookworm main (signed Release; Valid-Until not enforced): GCC 12.2.0-14+deb12u1, binutils 2.40-2, kernel headers linux-libc-dev 6.1.176-1.
- `--prefix=/usr --libdir=/usr/lib64 --disable-werror --enable-kernel=3.2`; no source patches.
- Compiler runtime: Link: generated libgcc_s stub with GCC 4.8.5's interface; tests run with the real Debian 12 GCC 12.2 libgcc_s.so.1 (requires GLIBC_2.35 at most).

| Library | Public numeric GLIBC definitions |
| --- | ---: |
| c | 2845 |
| m | 1374 |
| ld | 8 |
| resolv | 67 |
| pthread | 12 |
| dl | 3 |
| rt | 5 |
| util | 1 |
| **Total** | **4315** |

All eight native DSOs match GNU readelf and the pinned snapshot with zero ABI
differences. The stub audit preserves measured defaults, compatibility versions,
native types, binding/visibility, object sizes, alignment, and shared storage.
Exhaustive isolated-library checks pass **4315 accepted versioned
interfaces, 356 rejected interfaces, 3545 ordinary default bindings,
and 416 compatibility-only names rejected for new unversioned references**.

## Changes from 2.43

The snapshot adds 2 and removes 0 interfaces in this scope:
Added: `cosh@GLIBC_2.44` (m), `sinh@GLIBC_2.44` (m).

Measured default roles changed for 2 interfaces (`yes` is the default `@@` version, `no` a compatibility `@` version): `cosh@GLIBC_2.2.5` (m) yes to no, `sinh@GLIBC_2.2.5` (m) yes to no.
Measured symbol types changed for `__cosh_finite@GLIBC_2.15` (m) FUNC to IFUNC, `coshf32x@GLIBC_2.27` (m) FUNC to IFUNC, `coshf64@GLIBC_2.27` (m) FUNC to IFUNC, `sinhf32x@GLIBC_2.27` (m) FUNC to IFUNC, `sinhf64@GLIBC_2.27` (m) FUNC to IFUNC, `tanh@GLIBC_2.2.5` (m) FUNC to IFUNC.
9 interfaces changed between GLOBAL and WEAK binding; the stubs preserve
these, and the exhaustive default checks cover the resulting bindings.

2.44, signed on 2026-07-24, is the newest release in the pinned ABI lists. It gave `cosh` and `sinh`
new `GLIBC_2.44` defaults. No later release exists in the matrix, so the too-new tests use a name
absent from every snapshot; they still prove that an undefined interface fails at the target linker.

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
export ABI_GLIBC_RELEASE=2.44
export RUSTUP_TOOLCHAIN=1.99.0
export ABI_BEHAVIOR_CC=/usr/bin/clang-24
bash examples/glibc-2.44/run.sh /tmp/glibc-2.44-work
bash examples/glibc-two-paths/elf-and-check.sh /tmp/glibc-2.44-work
bash examples/glibc-two-paths/prepare-and-check.sh /tmp/glibc-2.44-work
bash examples/glibc-two-paths/application-and-check.sh /tmp/glibc-2.44-work
bash examples/glibc-two-paths/provider-and-check.sh \
  /tmp/glibc-2.44-work ../cargo-libc /usr/bin/clang-24
```

Use a new absolute work directory. Local validation on 2026-10-11 used Rust 1.99.0,
Clang 18.1.3, GNU binutils 2.42, and podman 5.8.2 on WSL Ubuntu 24.04, with
cargo-libc `bfe04693f6685770b47c9346e32de556a8029f4e`. The measured profile `gnu-source-2.44` is
committed to [history provenance](../data/glibc-history/PROVENANCE.json) and
[the interface reference](reference/GLIBC-SYMBOLS.md).

Both Clang 21/24 jobs for 2.44 passed in the [complete 84-job hosted run](https://github.com/rustcommons/linux-abi-tools/actions/runs/38158161339) at
producer `ae68bf63f5ddde7fc7852e76eab83ad2c1beb2c0` with consumer `bfe04693f6685770b47c9346e32de556a8029f4e` on 2026-10-11. Distribution/kernel
qualification, native dependency adapters, and authenticated SDK packaging remain open.

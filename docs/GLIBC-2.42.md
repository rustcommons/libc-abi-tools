# glibc 2.42 provider

The experimental x86_64 Linux provider selects GNU glibc 2.42 explicitly.
Matching headers, CRT objects, nonshared archives, loader, and runtime DSOs come
from a pinned source build. This is a generic GNU environment; distribution and
old-kernel qualification remain separate tasks.

Inputs are pinned in [INPUTS.toml](../examples/glibc-2.42/INPUTS.toml):

- Official GNU `glibc-2.42.tar.xz`, SHA-256
  `d1775e32e4628e64ef930f435b67bb63af7599acb6be2b335b9f19f16509f17f`.
  On 2026-10-11 its detached signature verified with `gpgv` against the GNU
  keyring (RSA key `DC2B16215ED5412A`, Andreas K. Huettel). CI checks the SHA-256 only.
- Builder `docker.io/library/debian@sha256:bc49dc1918ee1a47a93e65b5e4676e8680fb754b133197b92ca52bfe6731d5f0` with packages from
  http://snapshot.debian.org/archive/debian/20260901T000000Z bookworm main (signed Release; Valid-Until not enforced): GCC 12.2.0-14+deb12u1, binutils 2.40-2, kernel headers linux-libc-dev 6.1.176-1.
- `--prefix=/usr --libdir=/usr/lib64 --disable-werror --enable-kernel=3.2`; no source patches.
- Compiler runtime: Real Debian 12 GCC 12.2 libgcc_s.so.1 (requires GLIBC_2.35 at most).

| Library | Public numeric GLIBC definitions |
| --- | ---: |
| c | 2837 |
| m | 1320 |
| ld | 8 |
| resolv | 67 |
| pthread | 12 |
| dl | 3 |
| rt | 5 |
| util | 1 |
| **Total** | **4253** |

All eight native DSOs match GNU readelf and the pinned snapshot with zero ABI
differences. The stub audit preserves measured defaults, compatibility versions,
native types, binding/visibility, object sizes, alignment, and shared storage.
Exhaustive isolated-library checks pass **4253 accepted versioned
interfaces, 418 rejected interfaces, 3538 ordinary default bindings,
and 415 compatibility-only names rejected for new unversioned references**.

## Changes from 2.41

The snapshot adds 57 and removes 0 interfaces in this scope:
Added: `__inet_ntop_chk@GLIBC_2.42` (c), `__inet_pton_chk@GLIBC_2.42` (c), `cfgetibaud@GLIBC_2.42` (c), `cfgetispeed@GLIBC_2.42` (c), `cfgetobaud@GLIBC_2.42` (c), `cfgetospeed@GLIBC_2.42` (c), `cfsetbaud@GLIBC_2.42` (c), `cfsetibaud@GLIBC_2.42` (c), `cfsetispeed@GLIBC_2.42` (c), `cfsetobaud@GLIBC_2.42` (c), `cfsetospeed@GLIBC_2.42` (c), `cfsetspeed@GLIBC_2.42` (c), `pthread_gettid_np@GLIBC_2.42` (c), `uabs@GLIBC_2.42` (c), `uimaxabs@GLIBC_2.42` (c), `ulabs@GLIBC_2.42` (c), `ullabs@GLIBC_2.42` (c), `compoundn@GLIBC_2.42` (m), `compoundnf@GLIBC_2.42` (m), `compoundnf128@GLIBC_2.42` (m), `compoundnf32@GLIBC_2.42` (m), `compoundnf32x@GLIBC_2.42` (m), `compoundnf64@GLIBC_2.42` (m), `compoundnf64x@GLIBC_2.42` (m), `compoundnl@GLIBC_2.42` (m), `pown@GLIBC_2.42` (m), `pownf@GLIBC_2.42` (m), `pownf128@GLIBC_2.42` (m), `pownf32@GLIBC_2.42` (m), `pownf32x@GLIBC_2.42` (m), `pownf64@GLIBC_2.42` (m), `pownf64x@GLIBC_2.42` (m), `pownl@GLIBC_2.42` (m), `powr@GLIBC_2.42` (m), `powrf@GLIBC_2.42` (m), `powrf128@GLIBC_2.42` (m), `powrf32@GLIBC_2.42` (m), `powrf32x@GLIBC_2.42` (m), `powrf64@GLIBC_2.42` (m), `powrf64x@GLIBC_2.42` (m), and 17 more.

Measured default roles changed for 5 interfaces (`yes` is the default `@@` version, `no` a compatibility `@` version): `cfgetispeed@GLIBC_2.2.5` (c) yes to no, `cfgetospeed@GLIBC_2.2.5` (c) yes to no, `cfsetispeed@GLIBC_2.2.5` (c) yes to no, `cfsetospeed@GLIBC_2.2.5` (c) yes to no, `cfsetspeed@GLIBC_2.2.5` (c) yes to no.
Measured symbol types changed for `modf@GLIBC_2.2.5` (c) FUNC to IFUNC, `modff@GLIBC_2.2.5` (c) FUNC to IFUNC, `__atanh_finite@GLIBC_2.15` (m) FUNC to IFUNC, `__sinh_finite@GLIBC_2.15` (m) FUNC to IFUNC, `modf@GLIBC_2.2.5` (m) FUNC to IFUNC, `modff@GLIBC_2.2.5` (m) FUNC to IFUNC, `modff32@GLIBC_2.27` (m) FUNC to IFUNC, `modff32x@GLIBC_2.27` (m) FUNC to IFUNC, `modff64@GLIBC_2.27` (m) FUNC to IFUNC.
2 interfaces changed between GLOBAL and WEAK binding; the stubs preserve
these, and the exhaustive default checks cover the resulting bindings.

2.42 is the first release whose stated minimum (GCC 12.1, binutils 2.39) requires the Debian 12 builder.
It added the `cfsetbaud` family and gave `cfgetispeed`, `cfgetospeed`, `cfsetispeed`, `cfsetospeed`
and `cfsetspeed` new `GLIBC_2.42` defaults.

## Applications and Cargo

Real-runtime tests pass allocation, threads/TLS destruction, unwinding, historical
data views, memcpy and fmemopen compatibility behavior, the pthread default-attribute
and sigvec boundary fixtures (each following this provider's evidence), and the five
dedicated math/loading/realtime/utility/resolver suites. Cargo preserves host build
scripts and proc macros, audits target link/runtime paths, and rejects mismatched
release selection. The cargo-libc build, tier 1 scan, tier 2 local runner under a
Linux 3.10 filter, and `check` all pass. The too-new link tests use `__memset_explicit_chk`,
chosen from the matrix (`too-new-symbol.py`), and fail at the linker as required.
The Rust test program's highest directly imported GLIBC tag was 2.34; this is an
import measurement, not a general minimum-runtime guarantee.

## Reproduction

```bash
export ABI_GLIBC_RELEASE=2.42
export RUSTUP_TOOLCHAIN=1.99.0
export ABI_BEHAVIOR_CC=/usr/bin/clang-24
bash examples/glibc-2.42/run.sh /tmp/glibc-2.42-work
bash examples/glibc-two-paths/elf-and-check.sh /tmp/glibc-2.42-work
bash examples/glibc-two-paths/prepare-and-check.sh /tmp/glibc-2.42-work
bash examples/glibc-two-paths/application-and-check.sh /tmp/glibc-2.42-work
bash examples/glibc-two-paths/provider-and-check.sh \
  /tmp/glibc-2.42-work ../cargo-libc /usr/bin/clang-24
```

Use a new absolute work directory. Local validation on 2026-10-11 used Rust 1.99.0,
Clang 18.1.3, GNU binutils 2.42, and podman 5.8.2 on WSL Ubuntu 24.04, with
cargo-libc `bfe04693f6685770b47c9346e32de556a8029f4e`. The measured profile `gnu-source-2.42` is
committed to [history provenance](../data/glibc-history/PROVENANCE.json) and
[the interface reference](reference/GLIBC-SYMBOLS.md).

Hosted Clang 21/24 jobs for 2.42 are configured in the provider workflow;
hosted qualification is recorded here once they pass. Distribution/kernel
qualification, native dependency adapters, and authenticated SDK packaging remain open.

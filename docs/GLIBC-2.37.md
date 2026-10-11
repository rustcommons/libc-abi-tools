# glibc 2.37 provider

The experimental x86_64 Linux provider selects GNU glibc 2.37 explicitly.
Matching headers, CRT objects, nonshared archives, loader, and runtime DSOs come
from a pinned source build. This is a generic GNU environment; distribution and
old-kernel qualification remain separate tasks.

Inputs are pinned in [INPUTS.toml](../examples/glibc-2.37/INPUTS.toml):

- Official GNU `glibc-2.37.tar.xz`, SHA-256
  `2257eff111a1815d74f46856daaf40b019c1e553156c69d48ba0cbfc1bb91a43`.
  On 2026-10-11 its detached signature verified with `gpgv` against the GNU
  keyring (RSA key `16792B4EA25340F8`, Carlos O'Donell). CI checks the SHA-256 only.
- Builder `docker.io/library/debian@sha256:bc49dc1918ee1a47a93e65b5e4676e8680fb754b133197b92ca52bfe6731d5f0` with packages from
  http://snapshot.debian.org/archive/debian/20260901T000000Z bookworm main (signed Release; Valid-Until not enforced): GCC 12.2.0-14+deb12u1, binutils 2.40-2, kernel headers linux-libc-dev 6.1.176-1.
- `--prefix=/usr --libdir=/usr/lib64 --disable-werror --enable-kernel=3.2`; no source patches.
- Compiler runtime: Real Debian 12 GCC 12.2 libgcc_s.so.1 (requires GLIBC_2.35 at most).

| Library | Public numeric GLIBC definitions |
| --- | ---: |
| c | 2703 |
| m | 1181 |
| ld | 8 |
| resolv | 67 |
| pthread | 12 |
| dl | 3 |
| rt | 5 |
| util | 1 |
| **Total** | **3980** |

All eight native DSOs match GNU readelf and the pinned snapshot with zero ABI
differences. The stub audit preserves measured defaults, compatibility versions,
native types, binding/visibility, object sizes, alignment, and shared storage.
Exhaustive isolated-library checks pass **3980 accepted versioned
interfaces, 691 rejected interfaces, 3275 ordinary default bindings,
and 413 compatibility-only names rejected for new unversioned references**.

## Changes from 2.36

The snapshot records no interface added or removed in this scope since 2.36.

Measured default roles changed for 1 interfaces (`yes` is the default `@@` version, `no` a compatibility `@` version): `__ivaliduser@GLIBC_2.2.5` (c) yes to no.
Measured symbol types changed for `wcpcpy@GLIBC_2.2.5` (c) FUNC to IFUNC, `wcpncpy@GLIBC_2.2.5` (c) FUNC to IFUNC, `wcscat@GLIBC_2.2.5` (c) FUNC to IFUNC, `wcsncat@GLIBC_2.2.5` (c) FUNC to IFUNC, `wcsncpy@GLIBC_2.2.5` (c) FUNC to IFUNC.
10 interfaces changed between GLOBAL and WEAK binding; the stubs preserve
these, and the exhaustive default checks cover the resulting bindings.

2.37 introduces the third builder: Debian 9's binutils 2.28 cannot assemble the AVX-512 string
functions (`vpcmpneqb`) even though glibc's stated minimum is binutils 2.25, so 2.37 and later use a
digest-pinned Debian 12 image with packages from a fixed snapshot.debian.org date (GCC 12.2, binutils
2.40, Linux 6.1 UAPI headers) and its GCC 12.2 unwinder, which requires at most `GLIBC_2.35`. The
snapshot adds no interfaces in this scope; `__ivaliduser` became compatibility-only.

## Applications and Cargo

Real-runtime tests pass allocation, threads/TLS destruction, unwinding, historical
data views, memcpy and fmemopen compatibility behavior, the pthread default-attribute
and sigvec boundary fixtures (each following this provider's evidence), and the five
dedicated math/loading/realtime/utility/resolver suites. Cargo preserves host build
scripts and proc macros, audits target link/runtime paths, and rejects mismatched
release selection. The cargo-libc build, tier 1 scan, tier 2 local runner under a
Linux 3.10 filter, and `check` all pass. The too-new link tests use `__isoc23_fscanf`,
chosen from the matrix (`too-new-symbol.py`), and fail at the linker as required.
The Rust test program's highest directly imported GLIBC tag was 2.34; this is an
import measurement, not a general minimum-runtime guarantee.

## Reproduction

```bash
export ABI_GLIBC_RELEASE=2.37
export RUSTUP_TOOLCHAIN=1.99.0
export ABI_BEHAVIOR_CC=/usr/bin/clang-24
bash examples/glibc-2.37/run.sh /tmp/glibc-2.37-work
bash examples/glibc-two-paths/elf-and-check.sh /tmp/glibc-2.37-work
bash examples/glibc-two-paths/prepare-and-check.sh /tmp/glibc-2.37-work
bash examples/glibc-two-paths/application-and-check.sh /tmp/glibc-2.37-work
bash examples/glibc-two-paths/provider-and-check.sh \
  /tmp/glibc-2.37-work ../cargo-libc /usr/bin/clang-24
```

Use a new absolute work directory. Local validation on 2026-10-11 used Rust 1.99.0,
Clang 18.1.3, GNU binutils 2.42, and podman 5.8.2 on WSL Ubuntu 24.04, with
cargo-libc `bfe04693f6685770b47c9346e32de556a8029f4e`. The measured profile `gnu-source-2.37` is
committed to [history provenance](../data/glibc-history/PROVENANCE.json) and
[the interface reference](reference/GLIBC-SYMBOLS.md).

Hosted Clang 21/24 jobs for 2.37 are configured in the provider workflow;
hosted qualification is recorded here once they pass. Distribution/kernel
qualification, native dependency adapters, and authenticated SDK packaging remain open.

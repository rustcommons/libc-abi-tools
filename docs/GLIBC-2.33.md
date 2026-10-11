# glibc 2.33 provider

The experimental x86_64 Linux provider selects GNU glibc 2.33 explicitly.
Matching headers, CRT objects, nonshared archives, loader, and runtime DSOs come
from a pinned source build. This is a generic GNU environment; distribution and
old-kernel qualification remain separate tasks.

Inputs are pinned in [INPUTS.toml](../examples/glibc-2.33/INPUTS.toml):

- Official GNU `glibc-2.33.tar.xz`, SHA-256
  `2e2556000e105dbd57f0b6b2a32ff2cf173bde4f0d85dffccfd8b7e51a0677ff`.
  On 2026-10-11 its detached signature verified with `gpgv` against the GNU
  keyring (RSA key `16792B4EA25340F8`, Carlos O'Donell). CI checks the SHA-256 only.
- Builder `docker.io/library/debian@sha256:16ee3e11da473f6565ef94a715f1a38b3079e0664913be78869c336a71cb6085` with packages from
  http://archive.debian.org/debian stretch main (signed Release; Valid-Until not enforced): GCC 6.3.0-18+deb9u1, binutils 2.28-5, kernel headers linux-libc-dev 4.9.228-1.
- `--prefix=/usr --libdir=/usr/lib64 --disable-werror --enable-kernel=3.2`; no source patches.
- Compiler runtime: Real Debian 9 GCC 6.3 libgcc_s.so.1 (requires GLIBC_2.14 at most).

| Library | Public numeric GLIBC definitions |
| --- | ---: |
| c | 2230 |
| m | 1090 |
| ld | 4 |
| resolv | 91 |
| pthread | 228 |
| dl | 9 |
| rt | 42 |
| util | 6 |
| **Total** | **3700** |

All eight native DSOs match GNU readelf and the pinned snapshot with zero ABI
differences. The stub audit preserves measured defaults, compatibility versions,
native types, binding/visibility, object sizes, alignment, and shared storage.
Exhaustive isolated-library checks pass **3700 accepted versioned
interfaces, 971 rejected interfaces, 3247 ordinary default bindings,
and 365 compatibility-only names rejected for new unversioned references**.

## Changes from 2.32

The snapshot adds 12 and removes 0 interfaces in this scope:
Added: `__x86_get_cpuid_feature_leaf@GLIBC_2.33` (c), `fstat@GLIBC_2.33` (c), `fstat64@GLIBC_2.33` (c), `fstatat@GLIBC_2.33` (c), `fstatat64@GLIBC_2.33` (c), `lstat@GLIBC_2.33` (c), `lstat64@GLIBC_2.33` (c), `mallinfo2@GLIBC_2.33` (c), `mknod@GLIBC_2.33` (c), `mknodat@GLIBC_2.33` (c), `stat@GLIBC_2.33` (c), `stat64@GLIBC_2.33` (c).

Measured default roles changed for 1 interfaces (`yes` is the default `@@` version, `no` a compatibility `@` version): `vtimes@GLIBC_2.2.5` (c) yes to no.
17 interfaces changed between GLOBAL and WEAK binding; the stubs preserve
these, and the exhaustive default checks cover the resulting bindings.

2.33 made `vtimes` compatibility-only; otherwise it adds 12 interfaces. It is the last release before the
library merger and completes stage 4 of the provider plan.

## Applications and Cargo

Real-runtime tests pass allocation, threads/TLS destruction, unwinding, historical
data views, memcpy and fmemopen compatibility behavior, the pthread default-attribute
and sigvec boundary fixtures (each following this provider's evidence), and the five
dedicated math/loading/realtime/utility/resolver suites. Cargo preserves host build
scripts and proc macros, audits target link/runtime paths, and rejects mismatched
release selection. The cargo-libc build, tier 1 scan, tier 2 local runner under a
Linux 3.10 filter, and `check` all pass. The too-new link tests use `_Fork`,
chosen from the matrix (`too-new-symbol.py`), and fail at the linker as required.
The Rust test program's highest directly imported GLIBC tag was 2.33; this is an
import measurement, not a general minimum-runtime guarantee.

## Reproduction

```bash
export ABI_GLIBC_RELEASE=2.33
export RUSTUP_TOOLCHAIN=1.99.0
export ABI_BEHAVIOR_CC=/usr/bin/clang-24
bash examples/glibc-2.33/run.sh /tmp/glibc-2.33-work
bash examples/glibc-two-paths/elf-and-check.sh /tmp/glibc-2.33-work
bash examples/glibc-two-paths/prepare-and-check.sh /tmp/glibc-2.33-work
bash examples/glibc-two-paths/application-and-check.sh /tmp/glibc-2.33-work
bash examples/glibc-two-paths/provider-and-check.sh \
  /tmp/glibc-2.33-work ../cargo-libc /usr/bin/clang-24
```

Use a new absolute work directory. Local validation on 2026-10-11 used Rust 1.99.0,
Clang 18.1.3, GNU binutils 2.42, and podman 5.8.2 on WSL Ubuntu 24.04, with
cargo-libc `bfe04693f6685770b47c9346e32de556a8029f4e`. The measured profile `gnu-source-2.33` is
committed to [history provenance](../data/glibc-history/PROVENANCE.json) and
[the interface reference](reference/GLIBC-SYMBOLS.md).

Hosted Clang 21/24 jobs for 2.33 are configured in the provider workflow;
hosted qualification is recorded here once they pass. Distribution/kernel
qualification, native dependency adapters, and authenticated SDK packaging remain open.

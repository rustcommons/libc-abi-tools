# glibc 2.36 provider

The experimental x86_64 Linux provider selects GNU glibc 2.36 explicitly.
Matching headers, CRT objects, nonshared archives, loader, and runtime DSOs come
from a pinned source build. This is a generic GNU environment; distribution and
old-kernel qualification remain separate tasks.

Inputs are pinned in [INPUTS.toml](../examples/glibc-2.36/INPUTS.toml):

- Official GNU `glibc-2.36.tar.xz`, SHA-256
  `1c959fea240906226062cb4b1e7ebce71a9f0e3c0836c09e7e3423d434fcfe75`.
  On 2026-10-11 its detached signature verified with `gpgv` against the GNU
  keyring (RSA key `16792B4EA25340F8`, Carlos O'Donell). CI checks the SHA-256 only.
- Builder `docker.io/library/debian@sha256:16ee3e11da473f6565ef94a715f1a38b3079e0664913be78869c336a71cb6085` with packages from
  http://archive.debian.org/debian stretch main (signed Release; Valid-Until not enforced): GCC 6.3.0-18+deb9u1, binutils 2.28-5, kernel headers linux-libc-dev 4.9.228-1.
- `--prefix=/usr --libdir=/usr/lib64 --disable-werror --enable-kernel=3.2`; no source patches.
- Compiler runtime: Real Debian 9 GCC 6.3 libgcc_s.so.1 (requires GLIBC_2.14 at most).

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
interfaces, 691 rejected interfaces, 3276 ordinary default bindings,
and 412 compatibility-only names rejected for new unversioned references**.

## Changes from 2.35

The snapshot adds 17 and removes 0 interfaces in this scope:
Added: `arc4random@GLIBC_2.36` (c), `arc4random_buf@GLIBC_2.36` (c), `arc4random_uniform@GLIBC_2.36` (c), `c8rtomb@GLIBC_2.36` (c), `fsconfig@GLIBC_2.36` (c), `fsmount@GLIBC_2.36` (c), `fsopen@GLIBC_2.36` (c), `fspick@GLIBC_2.36` (c), `mbrtoc8@GLIBC_2.36` (c), `mount_setattr@GLIBC_2.36` (c), `move_mount@GLIBC_2.36` (c), `open_tree@GLIBC_2.36` (c), `pidfd_getfd@GLIBC_2.36` (c), `pidfd_open@GLIBC_2.36` (c), `pidfd_send_signal@GLIBC_2.36` (c), `process_madvise@GLIBC_2.36` (c), `process_mrelease@GLIBC_2.36` (c).

No measured default role changed.
Measured symbol types changed for `sincos@GLIBC_2.2.5` (m) FUNC to IFUNC, `sincosf32x@GLIBC_2.27` (m) FUNC to IFUNC, `sincosf64@GLIBC_2.27` (m) FUNC to IFUNC.
0 interfaces changed between GLOBAL and WEAK binding; the stubs preserve
these, and the exhaustive default checks cover the resulting bindings.

2.36 is the last release built with the Debian 9 builder; from 2.37 the AVX-512 string functions need a
newer assembler than binutils 2.28. `sincos` became an IFUNC symbol.

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
export ABI_GLIBC_RELEASE=2.36
export RUSTUP_TOOLCHAIN=1.99.0
export ABI_BEHAVIOR_CC=/usr/bin/clang-24
bash examples/glibc-2.36/run.sh /tmp/glibc-2.36-work
bash examples/glibc-two-paths/elf-and-check.sh /tmp/glibc-2.36-work
bash examples/glibc-two-paths/prepare-and-check.sh /tmp/glibc-2.36-work
bash examples/glibc-two-paths/application-and-check.sh /tmp/glibc-2.36-work
bash examples/glibc-two-paths/provider-and-check.sh \
  /tmp/glibc-2.36-work ../cargo-libc /usr/bin/clang-24
```

Use a new absolute work directory. Local validation on 2026-10-11 used Rust 1.99.0,
Clang 18.1.3, GNU binutils 2.42, and podman 5.8.2 on WSL Ubuntu 24.04, with
cargo-libc `bfe04693f6685770b47c9346e32de556a8029f4e`. The measured profile `gnu-source-2.36` is
committed to [history provenance](../data/glibc-history/PROVENANCE.json) and
[the interface reference](reference/GLIBC-SYMBOLS.md).

Hosted Clang 21/24 jobs for 2.36 are configured in the provider workflow;
hosted qualification is recorded here once they pass. Distribution/kernel
qualification, native dependency adapters, and authenticated SDK packaging remain open.

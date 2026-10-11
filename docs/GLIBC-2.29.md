# glibc 2.29 provider

The experimental x86_64 Linux provider selects GNU glibc 2.29 explicitly.
Matching headers, CRT objects, nonshared archives, loader, and runtime DSOs come
from a pinned source build. This is a generic GNU environment; distribution and
old-kernel qualification remain separate tasks.

Inputs are pinned in [INPUTS.toml](../examples/glibc-2.29/INPUTS.toml):

- Official GNU `glibc-2.29.tar.xz`, SHA-256
  `f3eeb8d57e25ca9fc13c2af3dae97754f9f643bc69229546828e3a240e2af04b`.
  On 2026-10-11 its detached signature verified with `gpgv` against the GNU
  keyring (RSA key `79C43DFBF1CF2187`, Siddhesh Poyarekar). CI checks the SHA-256 only.
- Builder `docker.io/library/debian@sha256:16ee3e11da473f6565ef94a715f1a38b3079e0664913be78869c336a71cb6085` with packages from
  http://archive.debian.org/debian stretch main (signed Release; Valid-Until not enforced): GCC 6.3.0-18+deb9u1, binutils 2.28-5, kernel headers linux-libc-dev 4.9.228-1.
- `--prefix=/usr --libdir=/usr/lib64 --disable-werror --enable-kernel=3.2`; no source patches.
- Compiler runtime: Real Debian 9 GCC 6.3 libgcc_s.so.1 (requires GLIBC_2.14 at most).

| Library | Public numeric GLIBC definitions |
| --- | ---: |
| c | 2192 |
| m | 1073 |
| ld | 8 |
| resolv | 91 |
| pthread | 256 |
| dl | 9 |
| rt | 47 |
| util | 6 |
| **Total** | **3682** |

All eight native DSOs match GNU readelf and the pinned snapshot with zero ABI
differences. The stub audit preserves measured defaults, compatibility versions,
native types, binding/visibility, object sizes, alignment, and shared storage.
Exhaustive isolated-library checks pass **3682 accepted versioned
interfaces, 989 rejected interfaces, 3384 ordinary default bindings,
and 234 compatibility-only names rejected for new unversioned references**.

## Changes from 2.28

The snapshot adds 8 and removes 0 interfaces in this scope:
Added: `getcpu@GLIBC_2.29` (c), `posix_spawn_file_actions_addchdir_np@GLIBC_2.29` (c), `posix_spawn_file_actions_addfchdir_np@GLIBC_2.29` (c), `exp@GLIBC_2.29` (m), `exp2@GLIBC_2.29` (m), `log@GLIBC_2.29` (m), `log2@GLIBC_2.29` (m), `pow@GLIBC_2.29` (m).

Measured default roles changed for 6 interfaces (`yes` is the default `@@` version, `no` a compatibility `@` version): `_IO_vfscanf@GLIBC_2.2.5` (c) yes to no, `exp@GLIBC_2.2.5` (m) yes to no, `exp2@GLIBC_2.2.5` (m) yes to no, `log@GLIBC_2.2.5` (m) yes to no, `log2@GLIBC_2.2.5` (m) yes to no, `pow@GLIBC_2.2.5` (m) yes to no.
17 interfaces changed between GLOBAL and WEAK binding; the stubs preserve
these, and the exhaustive default checks cover the resulting bindings.

2.29 gave `exp`, `exp2`, `log`, `log2`, and `pow` new `GLIBC_2.29` defaults. A program built against this
provider that calls them (for example Rust's `f64::exp`, `f64::ln`, or `f64::powf`) therefore requires
glibc 2.29; build against an older provider to keep the older requirement. The exhaustive default
checks verify the binding.

## Applications and Cargo

Real-runtime tests pass allocation, threads/TLS destruction, unwinding, historical
data views, memcpy and fmemopen compatibility behavior, the pthread default-attribute
and sigvec boundary fixtures (each following this provider's evidence), and the five
dedicated math/loading/realtime/utility/resolver suites. Cargo preserves host build
scripts and proc macros, audits target link/runtime paths, and rejects mismatched
release selection. The cargo-libc build, tier 1 scan, tier 2 local runner under a
Linux 3.10 filter, and `check` all pass. The too-new link tests use `getdents64`,
chosen from the matrix (`too-new-symbol.py`), and fail at the linker as required.
The Rust test program's highest directly imported GLIBC tag was 2.28; this is an
import measurement, not a general minimum-runtime guarantee.

## Reproduction

```bash
export ABI_GLIBC_RELEASE=2.29
export RUSTUP_TOOLCHAIN=1.99.0
export ABI_BEHAVIOR_CC=/usr/bin/clang-24
bash examples/glibc-2.29/run.sh /tmp/glibc-2.29-work
bash examples/glibc-two-paths/elf-and-check.sh /tmp/glibc-2.29-work
bash examples/glibc-two-paths/prepare-and-check.sh /tmp/glibc-2.29-work
bash examples/glibc-two-paths/application-and-check.sh /tmp/glibc-2.29-work
bash examples/glibc-two-paths/provider-and-check.sh \
  /tmp/glibc-2.29-work ../cargo-libc /usr/bin/clang-24
```

Use a new absolute work directory. Local validation on 2026-10-11 used Rust 1.99.0,
Clang 18.1.3, GNU binutils 2.42, and podman 5.8.2 on WSL Ubuntu 24.04, with
cargo-libc `a259c167741c6b1d31eaa3b7c664df8faff3b7a7`. The measured profile `gnu-source-2.29` is
committed to [history provenance](../data/glibc-history/PROVENANCE.json) and
[the interface reference](reference/GLIBC-SYMBOLS.md).

Hosted Clang 21/24 jobs for 2.29 are configured in the provider workflow;
hosted qualification is recorded here once they pass. Distribution/kernel
qualification, native dependency adapters, and authenticated SDK packaging remain open.

# glibc 2.32 provider

The experimental x86_64 Linux provider selects GNU glibc 2.32 explicitly.
Matching headers, CRT objects, nonshared archives, loader, and runtime DSOs come
from a pinned source build. This is a generic GNU environment; distribution and
old-kernel qualification remain separate tasks.

Inputs are pinned in [INPUTS.toml](../examples/glibc-2.32/INPUTS.toml):

- Official GNU `glibc-2.32.tar.xz`, SHA-256
  `1627ea54f5a1a8467032563393e0901077626dc66f37f10ee6363bb722222836`.
  On 2026-10-11 its detached signature verified with `gpgv` against the GNU
  keyring (RSA key `16792B4EA25340F8`, Carlos O'Donell). CI checks the SHA-256 only.
- Builder `docker.io/library/debian@sha256:16ee3e11da473f6565ef94a715f1a38b3079e0664913be78869c336a71cb6085` with packages from
  http://archive.debian.org/debian stretch main (signed Release; Valid-Until not enforced): GCC 6.3.0-18+deb9u1, binutils 2.28-5, kernel headers linux-libc-dev 4.9.228-1.
- `--prefix=/usr --libdir=/usr/lib64 --disable-werror --enable-kernel=3.2`; no source patches.
- Compiler runtime: Real Debian 9 GCC 6.3 libgcc_s.so.1 (requires GLIBC_2.14 at most).

| Library | Public numeric GLIBC definitions |
| --- | ---: |
| c | 2218 |
| m | 1090 |
| ld | 4 |
| resolv | 91 |
| pthread | 228 |
| dl | 9 |
| rt | 42 |
| util | 6 |
| **Total** | **3688** |

All eight native DSOs match GNU readelf and the pinned snapshot with zero ABI
differences. The stub audit preserves measured defaults, compatibility versions,
native types, binding/visibility, object sizes, alignment, and shared storage.
Exhaustive isolated-library checks pass **3688 accepted versioned
interfaces, 983 rejected interfaces, 3236 ordinary default bindings,
and 364 compatibility-only names rejected for new unversioned references**.

## Changes from 2.31

The snapshot adds 18 and removes 22 interfaces in this scope:
Added: `__libc_single_threaded@GLIBC_2.32` (c), `pthread_attr_getsigmask_np@GLIBC_2.32` (c), `pthread_attr_setaffinity_np@GLIBC_2.3.3` (c), `pthread_attr_setaffinity_np@GLIBC_2.3.4` (c), `pthread_attr_setaffinity_np@GLIBC_2.32` (c), `pthread_attr_setsigmask_np@GLIBC_2.32` (c), `pthread_getaffinity_np@GLIBC_2.3.3` (c), `pthread_getaffinity_np@GLIBC_2.3.4` (c), `pthread_getaffinity_np@GLIBC_2.32` (c), `pthread_getattr_np@GLIBC_2.2.5` (c), `pthread_getattr_np@GLIBC_2.32` (c), `pthread_sigmask@GLIBC_2.2.5` (c), `pthread_sigmask@GLIBC_2.32` (c), `sigabbrev_np@GLIBC_2.32` (c), `sigdescr_np@GLIBC_2.32` (c), `strerrordesc_np@GLIBC_2.32` (c), `strerrorname_np@GLIBC_2.32` (c), `exp10f@GLIBC_2.32` (m).
Removed: `calloc@GLIBC_2.2.5` (ld), `free@GLIBC_2.2.5` (ld), `malloc@GLIBC_2.2.5` (ld), `realloc@GLIBC_2.2.5` (ld), `pthread_attr_getschedpolicy@GLIBC_2.2.5` (pthread), `pthread_attr_getscope@GLIBC_2.2.5` (pthread), `pthread_attr_setaffinity_np@GLIBC_2.3.3` (pthread), `pthread_attr_setaffinity_np@GLIBC_2.3.4` (pthread), `pthread_attr_setschedpolicy@GLIBC_2.2.5` (pthread), `pthread_attr_setscope@GLIBC_2.2.5` (pthread), `pthread_cond_destroy@GLIBC_2.2.5` (pthread), `pthread_cond_destroy@GLIBC_2.3.2` (pthread), `pthread_cond_init@GLIBC_2.2.5` (pthread), `pthread_cond_init@GLIBC_2.3.2` (pthread), `pthread_condattr_destroy@GLIBC_2.2.5` (pthread), `pthread_condattr_init@GLIBC_2.2.5` (pthread), `pthread_getaffinity_np@GLIBC_2.3.3` (pthread), `pthread_getaffinity_np@GLIBC_2.3.4` (pthread), `pthread_getattr_np@GLIBC_2.2.5` (pthread), `pthread_getschedparam@GLIBC_2.2.5` (pthread), `pthread_setschedparam@GLIBC_2.2.5` (pthread), `pthread_sigmask@GLIBC_2.2.5` (pthread).

Measured default roles changed for 16 interfaces (`yes` is the default `@@` version, `no` a compatibility `@` version): `__key_decryptsession_pk_LOCAL@GLIBC_2.2.5` (c) yes to no, `__key_encryptsession_pk_LOCAL@GLIBC_2.2.5` (c) yes to no, `__key_gendes_LOCAL@GLIBC_2.2.5` (c) yes to no, `__sysctl@GLIBC_2.2.5` (c) yes to no, `_sys_errlist@GLIBC_2.12` (c) yes to no, `_sys_nerr@GLIBC_2.12` (c) yes to no, `_sys_siglist@GLIBC_2.3.3` (c) yes to no, `clnt_sperrno@GLIBC_2.2.5` (c) yes to no, `getrpcport@GLIBC_2.2.5` (c) yes to no, `sstk@GLIBC_2.2.5` (c) yes to no, `sys_errlist@GLIBC_2.12` (c) yes to no, `sys_nerr@GLIBC_2.12` (c) yes to no, `sys_sigabbrev@GLIBC_2.3.3` (c) yes to no, `sys_siglist@GLIBC_2.3.3` (c) yes to no, `sysctl@GLIBC_2.2.5` (c) yes to no, `exp10f@GLIBC_2.2.5` (m) yes to no.
9 interfaces changed between GLOBAL and WEAK binding; the stubs preserve
these, and the exhaustive default checks cover the resulting bindings.

2.32 made `sys_errlist`, `sys_nerr`, `sys_siglist`, `sys_sigabbrev`, and `sysctl` compatibility-only and
removed the loader's exported `malloc`/`calloc`/`realloc`/`free`. The data-views fixture keeps testing
the historical `sys_errlist` copy relocations through explicit versions.

## Applications and Cargo

Real-runtime tests pass allocation, threads/TLS destruction, unwinding, historical
data views, memcpy and fmemopen compatibility behavior, the pthread default-attribute
and sigvec boundary fixtures (each following this provider's evidence), and the five
dedicated math/loading/realtime/utility/resolver suites. Cargo preserves host build
scripts and proc macros, audits target link/runtime paths, and rejects mismatched
release selection. The cargo-libc build, tier 1 scan, tier 2 local runner under a
Linux 3.10 filter, and `check` all pass. The too-new link tests use `__x86_get_cpuid_feature_leaf`,
chosen from the matrix (`too-new-symbol.py`), and fail at the linker as required.
The Rust test program's highest directly imported GLIBC tag was 2.32; this is an
import measurement, not a general minimum-runtime guarantee.

## Reproduction

```bash
export ABI_GLIBC_RELEASE=2.32
export RUSTUP_TOOLCHAIN=1.99.0
export ABI_BEHAVIOR_CC=/usr/bin/clang-24
bash examples/glibc-2.32/run.sh /tmp/glibc-2.32-work
bash examples/glibc-two-paths/elf-and-check.sh /tmp/glibc-2.32-work
bash examples/glibc-two-paths/prepare-and-check.sh /tmp/glibc-2.32-work
bash examples/glibc-two-paths/application-and-check.sh /tmp/glibc-2.32-work
bash examples/glibc-two-paths/provider-and-check.sh \
  /tmp/glibc-2.32-work ../cargo-libc /usr/bin/clang-24
```

Use a new absolute work directory. Local validation on 2026-10-11 used Rust 1.99.0,
Clang 18.1.3, GNU binutils 2.42, and podman 5.8.2 on WSL Ubuntu 24.04, with
cargo-libc `bfe04693f6685770b47c9346e32de556a8029f4e`. The measured profile `gnu-source-2.32` is
committed to [history provenance](../data/glibc-history/PROVENANCE.json) and
[the interface reference](reference/GLIBC-SYMBOLS.md).

Both Clang 21/24 jobs for 2.32 passed in the [complete 84-job hosted run](https://github.com/rustcommons/linux-abi-tools/actions/runs/38158161339) at
producer `ae68bf63f5ddde7fc7852e76eab83ad2c1beb2c0` with consumer `bfe04693f6685770b47c9346e32de556a8029f4e` on 2026-10-11. Distribution/kernel
qualification, native dependency adapters, and authenticated SDK packaging remain open.

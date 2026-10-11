# glibc 2.30 provider

The experimental x86_64 Linux provider selects GNU glibc 2.30 explicitly.
Matching headers, CRT objects, nonshared archives, loader, and runtime DSOs come
from a pinned source build. This is a generic GNU environment; distribution and
old-kernel qualification remain separate tasks.

Inputs are pinned in [INPUTS.toml](../examples/glibc-2.30/INPUTS.toml):

- Official GNU `glibc-2.30.tar.xz`, SHA-256
  `e2c4114e569afbe7edbc29131a43be833850ab9a459d81beb2588016d2bbb8af`.
  On 2026-10-11 its detached signature verified with `gpgv` against the GNU
  keyring (RSA key `16792B4EA25340F8`, Carlos O'Donell). CI checks the SHA-256 only.
- Builder `docker.io/library/debian@sha256:16ee3e11da473f6565ef94a715f1a38b3079e0664913be78869c336a71cb6085` with packages from
  http://archive.debian.org/debian stretch main (signed Release; Valid-Until not enforced): GCC 6.3.0-18+deb9u1, binutils 2.28-5, kernel headers linux-libc-dev 4.9.228-1.
- `--prefix=/usr --libdir=/usr/lib64 --disable-werror --enable-kernel=3.2`; no source patches.
- Compiler runtime: Real Debian 9 GCC 6.3 libgcc_s.so.1 (requires GLIBC_2.14 at most).

| Library | Public numeric GLIBC definitions |
| --- | ---: |
| c | 2196 |
| m | 1073 |
| ld | 8 |
| resolv | 91 |
| pthread | 259 |
| dl | 9 |
| rt | 47 |
| util | 6 |
| **Total** | **3689** |

All eight native DSOs match GNU readelf and the pinned snapshot with zero ABI
differences. The stub audit preserves measured defaults, compatibility versions,
native types, binding/visibility, object sizes, alignment, and shared storage.
Exhaustive isolated-library checks pass **3689 accepted versioned
interfaces, 982 rejected interfaces, 3379 ordinary default bindings,
and 246 compatibility-only names rejected for new unversioned references**.

## Changes from 2.29

The snapshot adds 9 and removes 2 interfaces in this scope:
Added: `getdents64@GLIBC_2.30` (c), `gettid@GLIBC_2.30` (c), `tgkill@GLIBC_2.30` (c), `twalk_r@GLIBC_2.30` (c), `pthread_cond_clockwait@GLIBC_2.30` (pthread), `pthread_mutex_clocklock@GLIBC_2.30` (pthread), `pthread_rwlock_clockrdlock@GLIBC_2.30` (pthread), `pthread_rwlock_clockwrlock@GLIBC_2.30` (pthread), `sem_clockwait@GLIBC_2.30` (pthread).
Removed: `__vfork@GLIBC_2.2.5` (pthread), `vfork@GLIBC_2.2.5` (pthread).

Measured default roles changed for 14 interfaces (`yes` is the default `@@` version, `no` a compatibility `@` version): `__nss_database_lookup@GLIBC_2.2.5` (c) yes to no, `__nss_next@GLIBC_2.2.5` (c) yes to no, `fattach@GLIBC_2.2.5` (c) yes to no, `fdetach@GLIBC_2.2.5` (c) yes to no, `getmsg@GLIBC_2.2.5` (c) yes to no, `getpmsg@GLIBC_2.2.5` (c) yes to no, `isastream@GLIBC_2.2.5` (c) yes to no, `putmsg@GLIBC_2.2.5` (c) yes to no, `putpmsg@GLIBC_2.2.5` (c) yes to no, `clock_getcpuclockid@GLIBC_2.2.5` (rt) yes to no, `clock_getres@GLIBC_2.2.5` (rt) yes to no, `clock_gettime@GLIBC_2.2.5` (rt) yes to no, `clock_nanosleep@GLIBC_2.2.5` (rt) yes to no, `clock_settime@GLIBC_2.2.5` (rt) yes to no.
3 interfaces changed between GLOBAL and WEAK binding; the stubs preserve
these, and the exhaustive default checks cover the resulting bindings.

2.30 removed libpthread's `vfork`/`__vfork` exports altogether (they had been compatibility-only since
2.20), made the STREAMS stubs (`fattach`, `getmsg`, `putmsg`, and similar) compatibility-only, and left
librt's `clock_*` functions as compatibility symbols, since libc provides the defaults.

## Applications and Cargo

Real-runtime tests pass allocation, threads/TLS destruction, unwinding, historical
data views, memcpy and fmemopen compatibility behavior, the pthread default-attribute
and sigvec boundary fixtures (each following this provider's evidence), and the five
dedicated math/loading/realtime/utility/resolver suites. Cargo preserves host build
scripts and proc macros, audits target link/runtime paths, and rejects mismatched
release selection. The cargo-libc build, tier 1 scan, tier 2 local runner under a
Linux 3.10 filter, and `check` all pass. The too-new link tests use `pthread_attr_getsigmask_np`,
chosen from the matrix (`too-new-symbol.py`), and fail at the linker as required.
The Rust test program's highest directly imported GLIBC tag was 2.30; this is an
import measurement, not a general minimum-runtime guarantee.

## Reproduction

```bash
export ABI_GLIBC_RELEASE=2.30
export RUSTUP_TOOLCHAIN=1.99.0
export ABI_BEHAVIOR_CC=/usr/bin/clang-24
bash examples/glibc-2.30/run.sh /tmp/glibc-2.30-work
bash examples/glibc-two-paths/elf-and-check.sh /tmp/glibc-2.30-work
bash examples/glibc-two-paths/prepare-and-check.sh /tmp/glibc-2.30-work
bash examples/glibc-two-paths/application-and-check.sh /tmp/glibc-2.30-work
bash examples/glibc-two-paths/provider-and-check.sh \
  /tmp/glibc-2.30-work ../cargo-libc /usr/bin/clang-24
```

Use a new absolute work directory. Local validation on 2026-10-11 used Rust 1.99.0,
Clang 18.1.3, GNU binutils 2.42, and podman 5.8.2 on WSL Ubuntu 24.04, with
cargo-libc `bfe04693f6685770b47c9346e32de556a8029f4e`. The measured profile `gnu-source-2.30` is
committed to [history provenance](../data/glibc-history/PROVENANCE.json) and
[the interface reference](reference/GLIBC-SYMBOLS.md).

Both Clang 21/24 jobs for 2.30 passed in the [complete 84-job hosted run](https://github.com/rustcommons/linux-abi-tools/actions/runs/38158161339) at
producer `ae68bf63f5ddde7fc7852e76eab83ad2c1beb2c0` with consumer `bfe04693f6685770b47c9346e32de556a8029f4e` on 2026-10-11. Distribution/kernel
qualification, native dependency adapters, and authenticated SDK packaging remain open.

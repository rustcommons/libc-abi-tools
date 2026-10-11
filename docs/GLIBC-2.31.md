# glibc 2.31 provider

The experimental x86_64 Linux provider selects GNU glibc 2.31 explicitly.
Matching headers, CRT objects, nonshared archives, loader, and runtime DSOs come
from a pinned source build. This is a generic GNU environment; distribution and
old-kernel qualification remain separate tasks.

Inputs are pinned in [INPUTS.toml](../examples/glibc-2.31/INPUTS.toml):

- Official GNU `glibc-2.31.tar.xz`, SHA-256
  `9246fe44f68feeec8c666bb87973d590ce0137cca145df014c72ec95be9ffd17`.
  On 2026-10-11 its detached signature verified with `gpgv` against the GNU
  keyring (RSA key `79C43DFBF1CF2187`, Siddhesh Poyarekar). CI checks the SHA-256 only.
- Builder `docker.io/library/debian@sha256:16ee3e11da473f6565ef94a715f1a38b3079e0664913be78869c336a71cb6085` with packages from
  http://archive.debian.org/debian stretch main (signed Release; Valid-Until not enforced): GCC 6.3.0-18+deb9u1, binutils 2.28-5, kernel headers linux-libc-dev 4.9.228-1.
- `--prefix=/usr --libdir=/usr/lib64 --disable-werror --enable-kernel=3.2`; no source patches.
- Compiler runtime: Real Debian 9 GCC 6.3 libgcc_s.so.1 (requires GLIBC_2.14 at most).

| Library | Public numeric GLIBC definitions |
| --- | ---: |
| c | 2201 |
| m | 1089 |
| ld | 8 |
| resolv | 91 |
| pthread | 246 |
| dl | 9 |
| rt | 42 |
| util | 6 |
| **Total** | **3692** |

All eight native DSOs match GNU readelf and the pinned snapshot with zero ABI
differences. The stub audit preserves measured defaults, compatibility versions,
native types, binding/visibility, object sizes, alignment, and shared storage.
Exhaustive isolated-library checks pass **3692 accepted versioned
interfaces, 979 rejected interfaces, 3258 ordinary default bindings,
and 349 compatibility-only names rejected for new unversioned references**.

## Changes from 2.30

The snapshot adds 23 and removes 20 interfaces in this scope:
Added: `clock_getcpuclockid@GLIBC_2.2.5` (c), `clock_getres@GLIBC_2.2.5` (c), `clock_gettime@GLIBC_2.2.5` (c), `clock_nanosleep@GLIBC_2.2.5` (c), `clock_settime@GLIBC_2.2.5` (c), `totalorder@GLIBC_2.31` (m), `totalorderf@GLIBC_2.31` (m), `totalorderf128@GLIBC_2.31` (m), `totalorderf32@GLIBC_2.31` (m), `totalorderf32x@GLIBC_2.31` (m), `totalorderf64@GLIBC_2.31` (m), `totalorderf64x@GLIBC_2.31` (m), `totalorderl@GLIBC_2.31` (m), `totalordermag@GLIBC_2.31` (m), `totalordermagf@GLIBC_2.31` (m), `totalordermagf128@GLIBC_2.31` (m), `totalordermagf32@GLIBC_2.31` (m), `totalordermagf32x@GLIBC_2.31` (m), `totalordermagf64@GLIBC_2.31` (m), `totalordermagf64x@GLIBC_2.31` (m), `totalordermagl@GLIBC_2.31` (m), `__libpthread_version_placeholder@GLIBC_2.2.6` (pthread), `pthread_clockjoin_np@GLIBC_2.31` (pthread).
Removed: `__nanosleep@GLIBC_2.2.6` (pthread), `__wait@GLIBC_2.2.5` (pthread), `nanosleep@GLIBC_2.2.5` (pthread), `pthread_attr_destroy@GLIBC_2.2.5` (pthread), `pthread_attr_getdetachstate@GLIBC_2.2.5` (pthread), `pthread_attr_getinheritsched@GLIBC_2.2.5` (pthread), `pthread_attr_getschedparam@GLIBC_2.2.5` (pthread), `pthread_attr_init@GLIBC_2.2.5` (pthread), `pthread_attr_setdetachstate@GLIBC_2.2.5` (pthread), `pthread_attr_setinheritsched@GLIBC_2.2.5` (pthread), `pthread_attr_setschedparam@GLIBC_2.2.5` (pthread), `pthread_equal@GLIBC_2.2.5` (pthread), `pthread_self@GLIBC_2.2.5` (pthread), `wait@GLIBC_2.2.5` (pthread), `waitpid@GLIBC_2.2.5` (pthread), `clock_getcpuclockid@GLIBC_2.2.5` (rt), `clock_getres@GLIBC_2.2.5` (rt), `clock_gettime@GLIBC_2.2.5` (rt), `clock_nanosleep@GLIBC_2.2.5` (rt), `clock_settime@GLIBC_2.2.5` (rt).

Measured default roles changed for 124 interfaces (`yes` is the default `@@` version, `no` a compatibility `@` version): `stime@GLIBC_2.2.5` (c) yes to no, `__acos_finite@GLIBC_2.15` (m) yes to no, `__acosf128_finite@GLIBC_2.26` (m) yes to no, `__acosf_finite@GLIBC_2.15` (m) yes to no, `__acosh_finite@GLIBC_2.15` (m) yes to no, `__acoshf128_finite@GLIBC_2.26` (m) yes to no, `__acoshf_finite@GLIBC_2.15` (m) yes to no, `__acoshl_finite@GLIBC_2.15` (m) yes to no, `__acosl_finite@GLIBC_2.15` (m) yes to no, `__asin_finite@GLIBC_2.15` (m) yes to no, `__asinf128_finite@GLIBC_2.26` (m) yes to no, `__asinf_finite@GLIBC_2.15` (m) yes to no, `__asinl_finite@GLIBC_2.15` (m) yes to no, `__atan2_finite@GLIBC_2.15` (m) yes to no, `__atan2f128_finite@GLIBC_2.26` (m) yes to no, `__atan2f_finite@GLIBC_2.15` (m) yes to no, `__atan2l_finite@GLIBC_2.15` (m) yes to no, `__atanh_finite@GLIBC_2.15` (m) yes to no, `__atanhf128_finite@GLIBC_2.26` (m) yes to no, `__atanhf_finite@GLIBC_2.15` (m) yes to no, `__atanhl_finite@GLIBC_2.15` (m) yes to no, `__cosh_finite@GLIBC_2.15` (m) yes to no, `__coshf128_finite@GLIBC_2.26` (m) yes to no, `__coshf_finite@GLIBC_2.15` (m) yes to no, `__coshl_finite@GLIBC_2.15` (m) yes to no, `__exp10_finite@GLIBC_2.15` (m) yes to no, `__exp10f128_finite@GLIBC_2.26` (m) yes to no, `__exp10f_finite@GLIBC_2.15` (m) yes to no, `__exp10l_finite@GLIBC_2.15` (m) yes to no, `__exp2_finite@GLIBC_2.15` (m) yes to no, `__exp2f128_finite@GLIBC_2.26` (m) yes to no, `__exp2f_finite@GLIBC_2.15` (m) yes to no, `__exp2l_finite@GLIBC_2.15` (m) yes to no, `__exp_finite@GLIBC_2.15` (m) yes to no, `__expf128_finite@GLIBC_2.26` (m) yes to no, `__expf_finite@GLIBC_2.15` (m) yes to no, `__expl_finite@GLIBC_2.15` (m) yes to no, `__fmod_finite@GLIBC_2.15` (m) yes to no, `__fmodf128_finite@GLIBC_2.26` (m) yes to no, `__fmodf_finite@GLIBC_2.15` (m) yes to no, and 84 more.
30 interfaces changed between GLOBAL and WEAK binding; the stubs preserve
these, and the exhaustive default checks cover the resulting bindings.

2.31 removed duplicate libpthread and librt exports (for example `pthread_attr_init`, `pthread_self`,
`nanosleep`, `clock_getres`) that libc provides, and made the `__*_finite` libm entry points and `stime`
compatibility-only (124 measured default changes in all).

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
export ABI_GLIBC_RELEASE=2.31
export RUSTUP_TOOLCHAIN=1.99.0
export ABI_BEHAVIOR_CC=/usr/bin/clang-24
bash examples/glibc-2.31/run.sh /tmp/glibc-2.31-work
bash examples/glibc-two-paths/elf-and-check.sh /tmp/glibc-2.31-work
bash examples/glibc-two-paths/prepare-and-check.sh /tmp/glibc-2.31-work
bash examples/glibc-two-paths/application-and-check.sh /tmp/glibc-2.31-work
bash examples/glibc-two-paths/provider-and-check.sh \
  /tmp/glibc-2.31-work ../cargo-libc /usr/bin/clang-24
```

Use a new absolute work directory. Local validation on 2026-10-11 used Rust 1.99.0,
Clang 18.1.3, GNU binutils 2.42, and podman 5.8.2 on WSL Ubuntu 24.04, with
cargo-libc `bfe04693f6685770b47c9346e32de556a8029f4e`. The measured profile `gnu-source-2.31` is
committed to [history provenance](../data/glibc-history/PROVENANCE.json) and
[the interface reference](reference/GLIBC-SYMBOLS.md).

Both Clang 21/24 jobs for 2.31 passed in the [complete 84-job hosted run](https://github.com/rustcommons/linux-abi-tools/actions/runs/38158161339) at
producer `ae68bf63f5ddde7fc7852e76eab83ad2c1beb2c0` with consumer `bfe04693f6685770b47c9346e32de556a8029f4e` on 2026-10-11. Distribution/kernel
qualification, native dependency adapters, and authenticated SDK packaging remain open.

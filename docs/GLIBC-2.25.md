# glibc 2.25 provider

The experimental x86_64 Linux provider selects GNU glibc 2.25 explicitly.
Matching headers, CRT objects, nonshared archives, loader, and runtime DSOs come
from a pinned source build. This is a generic GNU environment; distribution and
old-kernel qualification remain separate tasks.

Inputs are pinned in [INPUTS.toml](../examples/glibc-2.25/INPUTS.toml):

- Official GNU `glibc-2.25.tar.xz`, SHA-256
  `067bd9bb3390e79aa45911537d13c3721f1d9d3769931a30c2681bfee66f23a0`.
  On 2026-10-11 its detached signature verified with `gpgv` against the GNU
  keyring (RSA key `79C43DFBF1CF2187`, Siddhesh Poyarekar). CI checks the SHA-256 only.
- Builder `docker.io/library/centos@sha256:dead07b4d8ed7e29e98de0f4504d87e8880d4347859d839686a31da35a3b532f` with packages from
  https://vault.centos.org/7.9.2009/os/x86_64/: GCC 4.8.5-44.el7, kernel headers 3.10.0-1160.el7.
- `--prefix=/usr --libdir=/usr/lib64 --disable-werror --enable-kernel=3.2`; no source patches.
- Compiler runtime: Real CentOS GCC 4.8.5 libgcc_s.so.1, GLIBC requirements independently inspected.

| Library | Public numeric GLIBC definitions |
| --- | ---: |
| c | 2140 |
| m | 458 |
| ld | 8 |
| resolv | 91 |
| pthread | 235 |
| dl | 9 |
| rt | 47 |
| util | 6 |
| **Total** | **2994** |

All eight native DSOs match GNU readelf and the pinned snapshot with zero ABI
differences. The stub audit preserves measured defaults, compatibility versions,
native types, binding/visibility, object sizes, alignment, and shared storage.
Exhaustive isolated-library checks pass **2994 accepted versioned
interfaces, 1677 rejected interfaces, 2733 ordinary default bindings,
and 209 compatibility-only names rejected for new unversioned references**.

## Changes from 2.24

The snapshot adds 57 and removes 1 interfaces in this scope:
Added: `__explicit_bzero_chk@GLIBC_2.25` (c), `explicit_bzero@GLIBC_2.25` (c), `getentropy@GLIBC_2.25` (c), `getrandom@GLIBC_2.25` (c), `strfromd@GLIBC_2.25` (c), `strfromf@GLIBC_2.25` (c), `strfroml@GLIBC_2.25` (c), `__iscanonicall@GLIBC_2.25` (m), `__iseqsig@GLIBC_2.25` (m), `__iseqsigf@GLIBC_2.25` (m), `__iseqsigl@GLIBC_2.25` (m), `canonicalize@GLIBC_2.25` (m), `canonicalizef@GLIBC_2.25` (m), `canonicalizel@GLIBC_2.25` (m), `fegetmode@GLIBC_2.25` (m), `fesetexcept@GLIBC_2.25` (m), `fesetmode@GLIBC_2.25` (m), `fetestexceptflag@GLIBC_2.25` (m), `fmaxmag@GLIBC_2.25` (m), `fmaxmagf@GLIBC_2.25` (m), `fmaxmagl@GLIBC_2.25` (m), `fminmag@GLIBC_2.25` (m), `fminmagf@GLIBC_2.25` (m), `fminmagl@GLIBC_2.25` (m), `fromfp@GLIBC_2.25` (m), `fromfpf@GLIBC_2.25` (m), `fromfpl@GLIBC_2.25` (m), `fromfpx@GLIBC_2.25` (m), `fromfpxf@GLIBC_2.25` (m), `fromfpxl@GLIBC_2.25` (m), `getpayload@GLIBC_2.25` (m), `getpayloadf@GLIBC_2.25` (m), `getpayloadl@GLIBC_2.25` (m), `llogb@GLIBC_2.25` (m), `llogbf@GLIBC_2.25` (m), `llogbl@GLIBC_2.25` (m), `roundeven@GLIBC_2.25` (m), `roundevenf@GLIBC_2.25` (m), `roundevenl@GLIBC_2.25` (m), `setpayload@GLIBC_2.25` (m), and 17 more.
Removed: `__libc_memalign@GLIBC_2.2.5` (ld).

Measured default roles changed for 16 interfaces (`yes` is the default `@@` version, `no` a compatibility `@` version): `__strsep_1c@GLIBC_2.2.5` (c) yes to no, `__strsep_2c@GLIBC_2.2.5` (c) yes to no, `__strsep_3c@GLIBC_2.2.5` (c) yes to no, `__strtok_r_1c@GLIBC_2.2.5` (c) yes to no, `malloc_get_state@GLIBC_2.2.5` (c) yes to no, `malloc_set_state@GLIBC_2.2.5` (c) yes to no, `_gethtbyaddr@GLIBC_2.2.5` (resolv) yes to no, `_gethtbyname@GLIBC_2.2.5` (resolv) yes to no, `_gethtbyname2@GLIBC_2.2.5` (resolv) yes to no, `_gethtent@GLIBC_2.2.5` (resolv) yes to no, `_sethtent@GLIBC_2.2.5` (resolv) yes to no, `res_gethostbyaddr@GLIBC_2.2.5` (resolv) yes to no, `res_gethostbyname@GLIBC_2.2.5` (resolv) yes to no, `res_gethostbyname2@GLIBC_2.2.5` (resolv) yes to no, `res_send_setqhook@GLIBC_2.2.5` (resolv) yes to no, `res_send_setrhook@GLIBC_2.2.5` (resolv) yes to no.
16 interfaces changed between GLOBAL and WEAK binding; the stubs preserve
these, and the exhaustive default checks cover the resulting bindings.

2.25 removed `__libc_memalign` from the loader's exported ABI (the snapshot records the removal, and
the exhaustive suite rejects it) and made `malloc_get_state`/`malloc_set_state` and several obsolete
resolver entry points compatibility-only. This is the last release built with the CentOS 7 builder.

## Applications and Cargo

Real-runtime tests pass allocation, threads/TLS destruction, unwinding, historical
data views, memcpy and fmemopen compatibility behavior, the pthread default-attribute
and sigvec boundary fixtures (each following this provider's evidence), and the five
dedicated math/loading/realtime/utility/resolver suites. Cargo preserves host build
scripts and proc macros, audits target link/runtime paths, and rejects mismatched
release selection. The cargo-libc build, tier 1 scan, tier 2 local runner under a
Linux 3.10 filter, and `check` all pass. The too-new link tests use `memfd_create`,
chosen from the matrix (`too-new-symbol.py`), and fail at the linker as required.
The Rust test program's highest directly imported GLIBC tag was 2.18; this is an
import measurement, not a general minimum-runtime guarantee.

## Reproduction

```bash
export ABI_GLIBC_RELEASE=2.25
export RUSTUP_TOOLCHAIN=1.99.0
export ABI_BEHAVIOR_CC=/usr/bin/clang-24
bash examples/glibc-2.25/run.sh /tmp/glibc-2.25-work
bash examples/glibc-two-paths/elf-and-check.sh /tmp/glibc-2.25-work
bash examples/glibc-two-paths/prepare-and-check.sh /tmp/glibc-2.25-work
bash examples/glibc-two-paths/application-and-check.sh /tmp/glibc-2.25-work
bash examples/glibc-two-paths/provider-and-check.sh \
  /tmp/glibc-2.25-work ../cargo-libc /usr/bin/clang-24
```

Use a new absolute work directory. Local validation on 2026-10-11 used Rust 1.99.0,
Clang 18.1.3, GNU binutils 2.42, and podman 5.8.2 on WSL Ubuntu 24.04, with
cargo-libc `a259c167741c6b1d31eaa3b7c664df8faff3b7a7`. The measured profile `gnu-source-2.25` is
committed to [history provenance](../data/glibc-history/PROVENANCE.json) and
[the interface reference](reference/GLIBC-SYMBOLS.md).

Both Clang 21/24 jobs for 2.25 passed in the [hosted provider run](https://github.com/rustcommons/linux-abi-tools/actions/runs/38155189880) at producer
`27caf3dbf9b7b5b018763629d3a3d2ebba377123` with consumer `a259c167741c6b1d31eaa3b7c664df8faff3b7a7`,
and again in the [complete 84-job run](https://github.com/rustcommons/linux-abi-tools/actions/runs/38158161339) at producer `ae68bf63f5ddde7fc7852e76eab83ad2c1beb2c0`
with consumer `bfe04693f6685770b47c9346e32de556a8029f4e` on 2026-10-11. Distribution/kernel
qualification, native dependency adapters, and authenticated SDK packaging remain open.

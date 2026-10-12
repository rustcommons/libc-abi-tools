# glibc providers

Every glibc release in the pinned ABI lists, 2.17 through 2.44, has an experimental
x86_64 provider: generated link-only stubs for the eight libraries plus real headers,
CRT objects, nonshared archives, loader, and runtime DSOs from a pinned, signature-checked
GNU source build (2.17 also has a CentOS 7 package route). Each is a generic GNU build,
not a distribution-qualified sysroot; old-distribution and kernel qualification remain
separate. Select one with the v1 descriptor ([PROVIDER-CONTRACT.md](PROVIDER-CONTRACT.md)).
Generic GNU builds can differ from a distribution's glibc; the measured differences, their
consequences, and licensing considerations are in [DISTRIBUTION-DIFFERENCES.md](DISTRIBUTION-DIFFERENCES.md).
Today a provider is prepared with the repository's scripts; the planned ways to obtain one
(compile from shipped source by default, CI-built binaries, or a local container build) are
designed in [PROVIDER-DELIVERY.md](PROVIDER-DELIVERY.md).

The counts are the exhaustive isolated-library link checks from the hosted run:
versioned interfaces that must link, interfaces from other releases that must not,
unversioned references that must bind to the measured default, and names with only
compatibility versions that new unversioned references must not reach. The too-new
symbol is the matrix-selected interface that must fail to link in the C and Cargo
tests (2.43 and 2.44 have no later release, so they use a name absent from every snapshot).

| Release | Builder | Accepted | Rejected | Defaults | Compatibility-only | Too-new symbol | Details |
| --- | --- | ---: | ---: | ---: | ---: | --- | --- |
| 2.17 (prebuilt) | CentOS 7 packages | 2915 | 1756 | 2847 | 21 | `memfd_create` | [glibc-two-paths](../examples/glibc-two-paths/README.md) |
| 2.17 (source) | CentOS 7, GCC 4.8.5 | 2915 | 1756 | 2705 | 163 | `memfd_create` | [glibc-two-paths](../examples/glibc-two-paths/README.md) |
| 2.18 | CentOS 7, GCC 4.8.5 | 2921 | 1750 | 2711 | 163 | `memfd_create` | [GLIBC-2.18.md](GLIBC-2.18.md) |
| 2.19 | CentOS 7, GCC 4.8.5 | 2921 | 1750 | 2711 | 163 | `memfd_create` | [GLIBC-2.19.md](GLIBC-2.19.md) |
| 2.20 | CentOS 7, GCC 4.8.5 | 2921 | 1750 | 2709 | 165 | `memfd_create` | [GLIBC-2.20.md](GLIBC-2.20.md) |
| 2.21 | CentOS 7, GCC 4.8.5 | 2921 | 1750 | 2708 | 166 | `memfd_create` | [GLIBC-2.21.md](GLIBC-2.21.md) |
| 2.22 | CentOS 7, GCC 4.8.5 | 2922 | 1749 | 2703 | 171 | `memfd_create` | [GLIBC-2.22.md](GLIBC-2.22.md) |
| 2.23 | CentOS 7, GCC 4.8.5 | 2931 | 1740 | 2699 | 181 | `memfd_create` | [GLIBC-2.23.md](GLIBC-2.23.md) |
| 2.24 | CentOS 7, GCC 4.8.5 | 2938 | 1733 | 2693 | 193 | `memfd_create` | [GLIBC-2.24.md](GLIBC-2.24.md) |
| 2.25 | CentOS 7, GCC 4.8.5 | 2994 | 1677 | 2733 | 209 | `memfd_create` | [GLIBC-2.25.md](GLIBC-2.25.md) |
| 2.26 | Debian 9, GCC 6.3 | 3143 | 1528 | 2878 | 213 | `memfd_create` | [GLIBC-2.26.md](GLIBC-2.26.md) |
| 2.27 | Debian 9, GCC 6.3 | 3594 | 1077 | 3312 | 223 | `fcntl64` | [GLIBC-2.27.md](GLIBC-2.27.md) |
| 2.28 | Debian 9, GCC 6.3 | 3674 | 997 | 3382 | 233 | `getcpu` | [GLIBC-2.28.md](GLIBC-2.28.md) |
| 2.29 | Debian 9, GCC 6.3 | 3682 | 989 | 3384 | 234 | `getdents64` | [GLIBC-2.29.md](GLIBC-2.29.md) |
| 2.30 | Debian 9, GCC 6.3 | 3689 | 982 | 3379 | 246 | `pthread_attr_getsigmask_np` | [GLIBC-2.30.md](GLIBC-2.30.md) |
| 2.31 | Debian 9, GCC 6.3 | 3692 | 979 | 3258 | 349 | `pthread_attr_getsigmask_np` | [GLIBC-2.31.md](GLIBC-2.31.md) |
| 2.32 | Debian 9, GCC 6.3 | 3688 | 983 | 3236 | 364 | `__x86_get_cpuid_feature_leaf` | [GLIBC-2.32.md](GLIBC-2.32.md) |
| 2.33 | Debian 9, GCC 6.3 | 3700 | 971 | 3247 | 365 | `_Fork` | [GLIBC-2.33.md](GLIBC-2.33.md) |
| 2.34 | Debian 9, GCC 6.3 | 3863 | 808 | 3162 | 411 | `__memcmpeq` | [GLIBC-2.34.md](GLIBC-2.34.md) |
| 2.35 | Debian 9, GCC 6.3 | 3963 | 708 | 3259 | 412 | `arc4random` | [GLIBC-2.35.md](GLIBC-2.35.md) |
| 2.36 | Debian 9, GCC 6.3 | 3980 | 691 | 3276 | 412 | `__isoc23_fscanf` | [GLIBC-2.36.md](GLIBC-2.36.md) |
| 2.37 | Debian 12, GCC 12.2 | 3980 | 691 | 3275 | 413 | `__isoc23_fscanf` | [GLIBC-2.37.md](GLIBC-2.37.md) |
| 2.38 | Debian 12, GCC 12.2 | 4022 | 649 | 3315 | 413 | `pidfd_getpid` | [GLIBC-2.38.md](GLIBC-2.38.md) |
| 2.39 | Debian 12, GCC 12.2 | 4098 | 573 | 3390 | 413 | `sched_getattr` | [GLIBC-2.39.md](GLIBC-2.39.md) |
| 2.40 | Debian 12, GCC 12.2 | 4138 | 533 | 3428 | 415 | `sched_getattr` | [GLIBC-2.40.md](GLIBC-2.40.md) |
| 2.41 | Debian 12, GCC 12.2 | 4196 | 475 | 3486 | 415 | `__inet_ntop_chk` | [GLIBC-2.41.md](GLIBC-2.41.md) |
| 2.42 | Debian 12, GCC 12.2 | 4253 | 418 | 3538 | 415 | `__memset_explicit_chk` | [GLIBC-2.42.md](GLIBC-2.42.md) |
| 2.43 | Debian 12, GCC 12.2 | 4313 | 358 | 3545 | 416 | `glibc_abi_tools_absent_probe` | [GLIBC-2.43.md](GLIBC-2.43.md) |
| 2.44 | Debian 12, GCC 12.2 | 4315 | 356 | 3545 | 416 | `glibc_abi_tools_absent_probe` | [GLIBC-2.44.md](GLIBC-2.44.md) |
| **Total** | | **103282** | **32177** | **90514** | **8373** | | |

## What changes for users

- A program can only need the GLIBC versions its provider defines, and newer providers
  move common functions to newer default versions. Examples measured here: `expf`/`powf`
  (2.27), `exp`/`log`/`pow` (2.29), and from 2.34 `__libc_start_main` in every `crt1.o`,
  so every executable built with a 2.34 or later provider requires glibc 2.34 or newer.
  Choose the oldest provider that your target systems have.
- Source builds leave Sun RPC compatibility-only (upstream's default without
  `--enable-obsolete-rpc`), unlike the CentOS 7 packages of 2.17.
- From 2.28, `libpthread_nonshared.a` no longer exists; from 2.34, libpthread, libdl, librt
  and libutil are small placeholders and their interfaces live in libc. The eight-library
  provider contract describes both layouts unchanged.

## Builders

| Builder | Releases | Why |
| --- | --- | --- |
| CentOS 7 (GCC 4.8.5, binutils 2.27), vault packages | 2.17–2.25 | Era-matched; 2.26 needs GCC 4.9 |
| Debian 9 (GCC 6.3, binutils 2.28), archive.debian.org | 2.26–2.36 | Meets GCC 6.2 and binutils 2.25; binutils 2.28 cannot assemble the AVX-512 code in 2.37 |
| Debian 12 (GCC 12.2, binutils 2.40), snapshot.debian.org 2026-09-01 | 2.37–2.44 | 2.42 needs GCC 12.1 and binutils 2.39 |

Images are pinned by digest and apt verifies the signed Release files. Each provider links
against a generated `libgcc_s` stub with GCC 4.8.5's interface, so programs need no newer
compiler runtime than CentOS 7's; the tests run them with each builder's real `libgcc_s.so.1`.

Hosted run: [38158161339](https://github.com/rustcommons/linux-abi-tools/actions/runs/38158161339) at producer `ae68bf63f5ddde7fc7852e76eab83ad2c1beb2c0` with
consumer cargo-libc `bfe04693f6685770b47c9346e32de556a8029f4e`, Rust 1.99.0 and Clang 21/24.

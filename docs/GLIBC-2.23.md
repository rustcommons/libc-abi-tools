# glibc 2.23 provider

The experimental x86_64 Linux provider selects GNU glibc 2.23 explicitly.
Matching headers, CRT objects, nonshared archives, loader, and runtime DSOs come
from a pinned source build. This is a generic GNU environment; distribution and
old-kernel qualification remain separate tasks.

Inputs are pinned in [INPUTS.toml](../examples/glibc-2.23/INPUTS.toml):

- Official GNU `glibc-2.23.tar.xz`, SHA-256
  `94efeb00e4603c8546209cefb3e1a50a5315c86fa9b078b6fad758e187ce13e9`.
  On 2026-10-11 its detached signature verified with `gpgv` against the GNU
  keyring (RSA key `25EF0A436C2A4AFF`, Carlos O'Donell). CI checks the SHA-256 only.
- Builder `docker.io/library/centos@sha256:dead07b4d8ed7e29e98de0f4504d87e8880d4347859d839686a31da35a3b532f` with packages from
  https://vault.centos.org/7.9.2009/os/x86_64/: GCC 4.8.5-44.el7, kernel headers 3.10.0-1160.el7.
- `--prefix=/usr --libdir=/usr/lib64 --disable-werror --enable-kernel=3.2`; no source patches.
- Compiler runtime: Real CentOS GCC 4.8.5 libgcc_s.so.1, GLIBC requirements independently inspected.

| Library | Public numeric GLIBC definitions |
| --- | ---: |
| c | 2132 |
| m | 402 |
| ld | 9 |
| resolv | 91 |
| pthread | 235 |
| dl | 9 |
| rt | 47 |
| util | 6 |
| **Total** | **2931** |

All eight native DSOs match GNU readelf and the pinned snapshot with zero ABI
differences. The stub audit preserves measured defaults, compatibility versions,
native types, binding/visibility, object sizes, alignment, and shared storage.
Exhaustive isolated-library checks pass **2931 accepted versioned
interfaces, 1740 rejected interfaces, 2699 ordinary default bindings,
and 181 compatibility-only names rejected for new unversioned references**.

## Changes from 2.22

The snapshot adds 9 and removes 0 interfaces in this scope:
Added: `fts64_children@GLIBC_2.23` (c), `fts64_close@GLIBC_2.23` (c), `fts64_open@GLIBC_2.23` (c), `fts64_read@GLIBC_2.23` (c), `fts64_set@GLIBC_2.23` (c), `__signgam@GLIBC_2.23` (m), `lgamma@GLIBC_2.23` (m), `lgammaf@GLIBC_2.23` (m), `lgammal@GLIBC_2.23` (m).

Measured default roles changed for 13 interfaces (`yes` is the default `@@` version, `no` a compatibility `@` version): `advance@GLIBC_2.2.5` (c) yes to no, `bdflush@GLIBC_2.2.5` (c) yes to no, `create_module@GLIBC_2.2.5` (c) yes to no, `get_kernel_syms@GLIBC_2.2.5` (c) yes to no, `loc1@GLIBC_2.2.5` (c) yes to no, `loc2@GLIBC_2.2.5` (c) yes to no, `locs@GLIBC_2.2.5` (c) yes to no, `query_module@GLIBC_2.2.5` (c) yes to no, `step@GLIBC_2.2.5` (c) yes to no, `uselib@GLIBC_2.2.5` (c) yes to no, `lgamma@GLIBC_2.2.5` (m) yes to no, `lgammaf@GLIBC_2.2.5` (m) yes to no, `lgammal@GLIBC_2.2.5` (m) yes to no.
Measured symbol types changed for `__strcasestr@GLIBC_2.2.5` (c) IFUNC to FUNC, `strcasestr@GLIBC_2.2.5` (c) IFUNC to FUNC.
7 interfaces changed between GLOBAL and WEAK binding; the stubs preserve
these, and the exhaustive default checks cover the resulting bindings.

2.23 gave `lgamma`, `lgammaf`, `lgammal`, and libm's `__signgam` new `GLIBC_2.23` defaults; the
`GLIBC_2.2.5` versions remain for old binaries, so new references bind to `GLIBC_2.23`. Several obsolete
interfaces (`bdflush`, `create_module`, `get_kernel_syms`, `query_module`, `uselib`, and the `regexp.h`
`advance`/`step`/`loc1`/`loc2`/`locs`) became compatibility-only; the exhaustive compatibility-only check
rejects new unversioned references to them. This and the next two providers use the unchanged
CentOS 7 builder.

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
export ABI_GLIBC_RELEASE=2.23
export RUSTUP_TOOLCHAIN=1.99.0
export ABI_BEHAVIOR_CC=/usr/bin/clang-24
bash examples/glibc-2.23/run.sh /tmp/glibc-2.23-work
bash examples/glibc-two-paths/elf-and-check.sh /tmp/glibc-2.23-work
bash examples/glibc-two-paths/prepare-and-check.sh /tmp/glibc-2.23-work
bash examples/glibc-two-paths/application-and-check.sh /tmp/glibc-2.23-work
bash examples/glibc-two-paths/provider-and-check.sh \
  /tmp/glibc-2.23-work ../cargo-libc /usr/bin/clang-24
```

Use a new absolute work directory. Local validation on 2026-10-11 used Rust 1.99.0,
Clang 18.1.3, GNU binutils 2.42, and podman 5.8.2 on WSL Ubuntu 24.04, with
cargo-libc `a259c167741c6b1d31eaa3b7c664df8faff3b7a7`. The measured profile `gnu-source-2.23` is
committed to [history provenance](../data/glibc-history/PROVENANCE.json) and
[the interface reference](reference/GLIBC-SYMBOLS.md).

Both Clang 21/24 jobs for 2.23 passed in the [hosted provider run](https://github.com/rustcommons/linux-abi-tools/actions/runs/38155189880) at producer
`27caf3dbf9b7b5b018763629d3a3d2ebba377123` with consumer `a259c167741c6b1d31eaa3b7c664df8faff3b7a7`,
and again in the [complete 84-job run](https://github.com/rustcommons/linux-abi-tools/actions/runs/38158161339) at producer `ae68bf63f5ddde7fc7852e76eab83ad2c1beb2c0`
with consumer `bfe04693f6685770b47c9346e32de556a8029f4e` on 2026-10-11. Distribution/kernel
qualification, native dependency adapters, and authenticated SDK packaging remain open.

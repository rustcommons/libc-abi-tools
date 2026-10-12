# glibc 2.20 provider

The experimental x86_64 Linux provider selects GNU glibc 2.20 explicitly.
Matching headers, CRT objects, nonshared archives, loader, and runtime DSOs come
from a pinned source build. This is a generic GNU environment; distribution and
old-kernel qualification remain separate tasks.

Inputs are pinned in [INPUTS.toml](../examples/glibc-2.20/INPUTS.toml):

- Official GNU `glibc-2.20.tar.xz`, SHA-256
  `f84b6d42aecc288d593c397b0a3d02260a33ee686bce0c634eb9b32798f36ba5`.
  On 2026-10-11 its detached signature verified with `gpgv` against the GNU
  keyring (`https://ftp.gnu.org/gnu/gnu-keyring.gpg`, RSA key
  `25EF0A436C2A4AFF`, Carlos O'Donell). CI checks the SHA-256 only.
- The same digest-pinned CentOS 7 builder and archived signature-checked
  packages as 2.18 and 2.19, including GCC 4.8.5 and Linux 3.10 UAPI headers.
- `--prefix=/usr --libdir=/usr/lib64 --disable-werror --enable-kernel=3.2`.
- No source patches; the 2.18 builder and configuration built 2.20 unchanged.
- A generated `libgcc_s` stub with GCC 4.8.5's interface to link against, the real
  CentOS GCC 4.8.5 unwinder to run the tests, and explicit host GCC CRT begin/end objects.

The work tree's `prebuilt` directory supplies only the bootstrap unwinder file;
it is not a 2.20 distribution runtime. The selected target route is `source`.
Generated stubs supply link-time interfaces, never runtime implementations.

| Library | Public numeric GLIBC definitions |
| --- | ---: |
| c | 2126 |
| m | 398 |
| ld | 9 |
| resolv | 91 |
| pthread | 235 |
| dl | 9 |
| rt | 47 |
| util | 6 |
| **Total** | **2921** |

All eight native DSOs match GNU readelf and the pinned snapshot with zero ABI
differences. The full stub audit preserves measured defaults, compatibility
versions, native types, binding/visibility, object sizes, alignment, and shared
storage. Exhaustive isolated-library checks pass **2921 accepted versioned
interfaces, 1750 rejected interfaces, and 2709 ordinary default bindings**.

## Measured changes from 2.19

The pinned snapshots record no public interface added, removed, or changed in
size between 2.19 and 2.20 on x86_64
([adjacent differences](reference/GLIBC-DIFFS.md)), so there is no GLIBC_2.20
symbol to use as a new boundary fixture, and none is invented. The existing
matrix-derived boundaries apply: both `pthread_*attr_default_np@GLIBC_2.18`
interfaces link and run, while `fmemopen@GLIBC_2.22` and
`memfd_create@GLIBC_2.27` are rejected.

The measured native properties of existing interfaces did change, compared with
the `gnu-source-2.19` profile:

| Library | Interfaces | 2.19 | 2.20 |
| --- | --- | --- | --- |
| pthread | `vfork`, `__vfork` @GLIBC_2.2.5 | Default (`@@`) FUNC | Compatibility-only (`@`) IFUNC |
| c | `memset`, `__memset_chk` | FUNC | IFUNC |
| pthread | `pthread_rwlock_rdlock`, `_unlock`, `_wrlock` @GLIBC_2.2.5 | GLOBAL | WEAK |

The libpthread `vfork` change explains the two fewer ordinary default bindings
(2709 rather than 2711). The 2.20 source (`nptl/pt-vfork.c`, ChangeLog
2014-08-04) states that libpthread keeps `vfork` and `__vfork` only for the
historical shared-library ABI, as IFUNC symbols that redirect to libc's. A new
unversioned reference therefore binds to libc's `vfork@@GLIBC_2.2.5`. The
generated libpthread stub preserves both names as non-default IFUNC
definitions, which the export audit checks against the measured profile. Other differences are function sizes and shared
alias ranges, recorded in the profile but not ABI changes. Behavior changes
inside existing symbol versions are outside this ABI evidence.

Real-runtime tests pass allocation, threads/TLS destruction, unwinding, historical
data views, memcpy and fmemopen compatibility behavior, plus the five dedicated
math/loading/realtime/utility/resolver suites. Cargo preserves host build scripts
and proc macros, audits target link/runtime paths, rejects mismatched release
selection, and fails at the linker on the too-new `memfd_create` import. The
cargo-libc build, tier 1 scan, tier 2 local runner under a Linux 3.10 filter,
and `check` all pass with the 2.20 provider.

The six-field v1 descriptor is unchanged and now accepts exactly 2.17, 2.18,
2.19, 2.20, or 2.22. Both repositories implement identical parsers; older
consumers reject 2.20 explicitly. Existing provider selections remain exact,
with no fallback.

## Reproduction

On an x86_64 Linux build host with the documented tools:

```bash
export ABI_GLIBC_RELEASE=2.20
export RUSTUP_TOOLCHAIN=1.99.0
export ABI_BEHAVIOR_CC=/usr/bin/clang-24
bash examples/glibc-2.20/run.sh /tmp/glibc-2.20-work
bash examples/glibc-two-paths/elf-and-check.sh /tmp/glibc-2.20-work
bash examples/glibc-two-paths/prepare-and-check.sh /tmp/glibc-2.20-work
bash examples/glibc-two-paths/application-and-check.sh /tmp/glibc-2.20-work
bash examples/glibc-two-paths/provider-and-check.sh \
  /tmp/glibc-2.20-work ../cargo-libc /usr/bin/clang-24
```

Use a new absolute work directory. These scripts install into that tree, not the
host system. Local validation on 2026-10-11 used Rust 1.99.0, Clang 18.1.3,
GNU binutils 2.42, and podman 5.8.2 on WSL Ubuntu 24.04, with cargo-libc
`8444108c1b83e72ef484b95f756308de34725844`. The Cargo example's highest
directly imported numeric GLIBC tag was 2.18; this is an import measurement,
not a general minimum-runtime guarantee.

The measured profile `gnu-source-2.20`, input/metadata hashes, and default roles
are committed to [history provenance](../data/glibc-history/PROVENANCE.json) and
[the interface reference](reference/GLIBC-SYMBOLS.md), imported with
`scripts/import-glibc-elf-profile.py`.

Both new 2.20 jobs in the [hosted provider matrix](https://github.com/rustcommons/linux-abi-tools/actions/runs/38144128128)
passed under Rust 1.99.0 and Clang 21/24 on 2026-10-11 at producer commit
`5ed1e12856f98566dfb2c61d0c64ce7922306b27`, with pinned consumer commit
`8444108c1b83e72ef484b95f756308de34725844`. All fifteen jobs passed (five preparations
and ten applications), including every existing provider release. Actual
distribution/kernel qualification, native dependency adapters, and authenticated
SDK packaging remain open.

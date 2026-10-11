# glibc 2.19 provider

The experimental x86_64 Linux provider selects GNU glibc 2.19 explicitly.
Matching headers, CRT objects, nonshared archives, loader, and runtime DSOs come
from a pinned source build. This is a generic GNU environment; distribution and
old-kernel qualification remain separate tasks. In particular, it does not
qualify Ubuntu 14.04 or Debian 8, whose glibc 2.19 packages carry their own patches.

Inputs are pinned in [INPUTS.toml](../examples/glibc-2.19/INPUTS.toml):

- Official GNU `glibc-2.19.tar.xz`, SHA-256
  `2d3997f588401ea095a0b27227b1d50cdfdd416236f6567b564549d3b46ea2a2`.
  On 2026-10-11 its detached signature verified with `gpgv` against the GNU
  keyring (`https://ftp.gnu.org/gnu/gnu-keyring.gpg`, RSA key
  `25EF0A436C2A4AFF`, Carlos O'Donell). CI checks the SHA-256 only.
- The same digest-pinned CentOS 7 builder and archived signature-checked
  packages as 2.18, including GCC 4.8.5 and Linux 3.10 UAPI headers.
- `--prefix=/usr --libdir=/usr/lib64 --disable-werror --enable-kernel=3.2`.
- No source patches; the 2.18 builder and configuration built 2.19 unchanged.
- Compatible real CentOS GCC unwinder and explicit host GCC CRT begin/end objects.

The work tree's `prebuilt` directory supplies only the bootstrap unwinder file;
it is not a 2.19 distribution runtime. The selected target route is `source`.
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
interfaces, 1750 rejected interfaces, and 2711 ordinary default bindings**.

## No new versioned interfaces

The pinned snapshots record no public interface added, removed, or changed in
these eight libraries between 2.18 and 2.19 on x86_64
([adjacent differences](reference/GLIBC-DIFFS.md)); the measured profile
agrees, with the same 2921 definitions as 2.18. There is therefore no
GLIBC_2.19 symbol to use as a new boundary fixture, and none is invented.
The existing matrix-derived boundaries apply instead: both
`pthread_*attr_default_np@GLIBC_2.18` interfaces link and run under 2.19, while
`fmemopen@GLIBC_2.22` and `memfd_create@GLIBC_2.27` are rejected. The same
2711 ordinary default bindings as 2.18 are measured. Behavior changes inside
existing symbol versions are outside this ABI evidence.

Real-runtime tests pass allocation, threads/TLS destruction, unwinding, historical
data views, memcpy and fmemopen compatibility behavior, plus the five dedicated
math/loading/realtime/utility/resolver suites. Cargo preserves host build scripts
and proc macros, audits target link/runtime paths, rejects mismatched release
selection, and fails at the linker on the too-new `memfd_create` import. The
cargo-libc build, tier 1 scan, tier 2 local runner under a Linux 3.10 filter,
and `check` all pass with the 2.19 provider.

The six-field v1 descriptor is unchanged and now accepts exactly 2.17, 2.18,
2.19, or 2.22. Both repositories implement identical parsers; older consumers
reject 2.19 explicitly. Existing provider selections remain exact, with no fallback.

## Reproduction

On an x86_64 Linux build host with the documented tools:

```bash
export ABI_GLIBC_RELEASE=2.19
export RUSTUP_TOOLCHAIN=1.99.0
export ABI_BEHAVIOR_CC=/usr/bin/clang-24
bash examples/glibc-2.19/run.sh /tmp/glibc-2.19-work
bash examples/glibc-two-paths/elf-and-check.sh /tmp/glibc-2.19-work
bash examples/glibc-two-paths/prepare-and-check.sh /tmp/glibc-2.19-work
bash examples/glibc-two-paths/application-and-check.sh /tmp/glibc-2.19-work
bash examples/glibc-two-paths/provider-and-check.sh \
  /tmp/glibc-2.19-work ../cargo-libc /usr/bin/clang-24
```

Use a new absolute work directory. These scripts install into that tree, not the
host system. Local validation on 2026-10-11 used Rust 1.99.0, Clang 18.1.3,
GNU binutils 2.42, and podman 5.8.2 on WSL Ubuntu 24.04, with cargo-libc
`2a6d0e57f915c774d71b1261b6707014d2bde47e`. The Cargo example's highest
directly imported numeric GLIBC tag was 2.18; this is an import measurement,
not a general minimum-runtime guarantee.

The measured profile `gnu-source-2.19`, input/metadata hashes, and default roles
are committed to [history provenance](../data/glibc-history/PROVENANCE.json) and
[the interface reference](reference/GLIBC-SYMBOLS.md), imported with
`scripts/import-glibc-elf-profile.py`.

Hosted Clang 21/24 jobs for 2.19 are configured in the provider workflow;
hosted qualification is recorded here once they pass. Actual distribution/kernel
qualification, native dependency adapters, and authenticated SDK packaging remain open.

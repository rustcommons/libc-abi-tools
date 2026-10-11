# glibc 2.21 provider

The experimental x86_64 Linux provider selects GNU glibc 2.21 explicitly.
Matching headers, CRT objects, nonshared archives, loader, and runtime DSOs come
from a pinned source build. This is a generic GNU environment; distribution and
old-kernel qualification remain separate tasks. With 2.21, every release from
2.17 through 2.22 has a provider.

Inputs are pinned in [INPUTS.toml](../examples/glibc-2.21/INPUTS.toml):

- Official GNU `glibc-2.21.tar.xz`, SHA-256
  `aeeb362437965a5d3f40b151094ca79def04a115bd363fdd4a9a0c69482923b8`.
  On 2026-10-11 its detached signature verified with `gpgv` against the GNU
  keyring (`https://ftp.gnu.org/gnu/gnu-keyring.gpg`, RSA key
  `25EF0A436C2A4AFF`, Carlos O'Donell). CI checks the SHA-256 only.
- The same digest-pinned CentOS 7 builder and archived signature-checked
  packages as 2.18–2.20, including GCC 4.8.5 and Linux 3.10 UAPI headers.
- `--prefix=/usr --libdir=/usr/lib64 --disable-werror --enable-kernel=3.2`.
- No source patches; the 2.18 builder and configuration built 2.21 unchanged.
- Compatible real CentOS GCC unwinder and explicit host GCC CRT begin/end objects.

The work tree's `prebuilt` directory supplies only the bootstrap unwinder file;
it is not a 2.21 distribution runtime. The selected target route is `source`.
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
interfaces, 1750 rejected interfaces, and 2708 ordinary default bindings**.

## Measured changes from 2.20

The pinned snapshots record no public interface added, removed, or changed in
size between 2.20 and 2.21 on x86_64
([adjacent differences](reference/GLIBC-DIFFS.md)). The existing matrix-derived
boundaries apply: both `pthread_*attr_default_np@GLIBC_2.18` interfaces link and
run, while `fmemopen@GLIBC_2.22` and `memfd_create@GLIBC_2.27` are rejected.

One measured default changed. The 2.21 NEWS says: "The obsolete sigvec function
has been removed... ABI compatibility for old binaries using sigvec remains
intact." libc still exports `sigvec@GLIBC_2.2.5`, but only as a
compatibility (`@`) definition, so there is one fewer ordinary default binding
(2708 rather than 2709). Existing binaries that import it still load; new code
cannot link to it.

The new `sigvec-default-2.21.c` fixture tests this boundary from each
provider's own measured default flag. Where `sigvec` is a default (2.17–2.20),
an unversioned reference links, binds to `sigvec@GLIBC_2.2.5`, and installs and
delivers a signal handler under the real runtime. Where it is
compatibility-only (2.21 and later), the link fails with
`undefined reference to 'sigvec'`. It reaches the symbol through an asm label,
because 2.21 headers no longer declare it and older headers declare it with
their own type.

The other changes are 38 further GLOBAL/WEAK binding swaps in libc, libm, and
libpthread, plus section-index and function-size changes. The stubs preserve
them, and none changes which version an unversioned reference binds to.
Behavior changes inside existing symbol versions are outside this ABI evidence.

Real-runtime tests pass allocation, threads/TLS destruction, unwinding, historical
data views, memcpy and fmemopen compatibility behavior, plus the five dedicated
math/loading/realtime/utility/resolver suites. Cargo preserves host build scripts
and proc macros, audits target link/runtime paths, rejects mismatched release
selection, and fails at the linker on the too-new `memfd_create` import. The
cargo-libc build, tier 1 scan, tier 2 local runner under a Linux 3.10 filter,
and `check` all pass with the 2.21 provider.

The six-field v1 descriptor is unchanged and now accepts exactly one of 2.17
through 2.22. Both repositories implement identical parsers; older consumers
reject 2.21 explicitly. Existing provider selections remain exact, with no fallback.

## Reproduction

On an x86_64 Linux build host with the documented tools:

```bash
export ABI_GLIBC_RELEASE=2.21
export RUSTUP_TOOLCHAIN=1.99.0
export ABI_BEHAVIOR_CC=/usr/bin/clang-24
bash examples/glibc-2.21/run.sh /tmp/glibc-2.21-work
bash examples/glibc-two-paths/elf-and-check.sh /tmp/glibc-2.21-work
bash examples/glibc-two-paths/prepare-and-check.sh /tmp/glibc-2.21-work
bash examples/glibc-two-paths/application-and-check.sh /tmp/glibc-2.21-work
bash examples/glibc-two-paths/provider-and-check.sh \
  /tmp/glibc-2.21-work ../cargo-libc /usr/bin/clang-24
```

Use a new absolute work directory. These scripts install into that tree, not the
host system. Local validation on 2026-10-11 used Rust 1.99.0, Clang 18.1.3,
GNU binutils 2.42, and podman 5.8.2 on WSL Ubuntu 24.04, with cargo-libc
`16a464ea88aa3b54d652583303fe6ed6c4d26e58`. The sigvec fixture was also run
locally against the 2.19 and 2.20 providers. The Cargo example's highest
directly imported numeric GLIBC tag was 2.18; this is an import measurement,
not a general minimum-runtime guarantee.

The measured profile `gnu-source-2.21`, input/metadata hashes, and default roles
are committed to [history provenance](../data/glibc-history/PROVENANCE.json) and
[the interface reference](reference/GLIBC-SYMBOLS.md), imported with
`scripts/import-glibc-elf-profile.py`.

Both new 2.21 jobs in the [hosted provider matrix](https://github.com/rustcommons/linux-abi-tools/actions/runs/38147732823)
passed under Rust 1.99.0 and Clang 21/24 on 2026-10-11 at producer commit
`85b69c8ad27be4ae486e6723ca0698a06ba96c92`, with pinned consumer commit
`16a464ea88aa3b54d652583303fe6ed6c4d26e58`. All eighteen jobs passed (six preparations
and twelve applications). The retained diagnostics show the sigvec fixture
linking and running on 2.17 (both routes), 2.18, 2.19 and 2.20, and rejected on
2.21 and 2.22. Actual distribution/kernel qualification, native dependency
adapters, and authenticated SDK packaging remain open.

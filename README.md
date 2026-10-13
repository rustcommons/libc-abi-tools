![libc-abi-tools: an orange gear and release-to-ABI diagram](docs/assets/banner.svg)

# linux-abi-tools

ABI data and tools for targeting older Linux systems from Rust: which libc
interfaces and which kernel syscalls a selected target provides. Formerly
`libc-abi-tools`; renamed on 2026-10-10 when `kernel-abi-tools` joined it.

| Crate | Location | Purpose |
| --- | --- | --- |
| `libc-abi-tools` | repository root | glibc ABI metadata, stub generation, providers (described below) |
| [`kernel-abi-tools`](crates/kernel-abi-tools/README.md) | `crates/kernel-abi-tools` | Kernel syscall/`madvise` data and seccomp profiles that simulate older kernels |
| [`linux-targets`](crates/linux-targets/README.md) | `crates/linux-targets` | Distribution presets (glibc and kernel floors, image) and target selection rules |
| [`linux-abi-scan`](crates/linux-abi-scan/README.md) | `crates/linux-abi-scan` | Static inspection of built executables against the glibc and kernel floors (tier 1) |
| [`linux-abi-tools`](crates/linux-abi-tools/README.md) | `crates/linux-abi-tools` | One dependency that re-exports the four crates above |

[Workflows](docs/WORKFLOWS.md) explains which tool answers which question:
building against an older glibc, inspecting a binary, and running it under an
older kernel with a local seccomp filter (no root) or in a container. It also
shows how each maps onto cargo-libc. The crates are published to crates.io
only; see [Releasing](docs/RELEASING.md).

Each crate is std-only. `cargo` in the root acts on `libc-abi-tools` unless
given `--workspace` or `-p`.

## libc-abi-tools

[Current glibc coverage](docs/GLIBC-COVERAGE.md) lists each library, test stage,
qualified input profile, and remaining runtime/architecture work.
See [CI and release artifacts](docs/CI-AND-RELEASE-ARTIFACTS.md) for Rust/Clang roles,
the shared preparation stage, and planned version-suffixed target selection.

[Browse versioned glibc interfaces](docs/reference/GLIBC-SYMBOLS.md),
[release differences](docs/reference/GLIBC-DIFFS.md), and the
[history/conformance plan](docs/GLIBC-HISTORY.md). These references distinguish
new defaults from retained compatibility interfaces and link to input provenance.

The experimental glibc 2.17–2.44 [provider contract](docs/PROVIDER-CONTRACT.md) connects
all eight generated stub libraries to the companion `cargo-libc` coordinator.
CI targets Rust 1.99.0 and Clang 21/24; the roadmap tracks each vendored version's
remaining real input and runtime setup.
The [2.22 experiment](docs/GLIBC-2.22.md) includes a source-built GNU runtime and
an explicit 2.22 symbol test; SLES qualification remains separate.
The [2.18 provider](docs/GLIBC-2.18.md) fills the first gap with pinned source
inputs, exhaustive linking, and pthread default-attribute boundary tests.
The [2.19 provider](docs/GLIBC-2.19.md) uses the same builder; it adds no new
versioned interfaces on x86_64, so it reuses the existing boundary tests.
The [2.20 provider](docs/GLIBC-2.20.md) also builds unchanged; it adds no
interfaces but records measured default and IFUNC changes, such as libpthread's
`vfork` becoming compatibility-only.
The [2.21 provider](docs/GLIBC-2.21.md) completes 2.17–2.22; its new fixture
checks that `sigvec`, compatibility-only from 2.21, no longer links for new code.
Every release in the pinned ABI lists now has a provider: see the [provider index](docs/PROVIDERS.md)
for each release's builder, exhaustive results, and what changes for users (for example, every
executable built with a 2.34 or later provider requires glibc 2.34).

**Target libc versions beyond your host.**

[![CI](https://github.com/rustcommons/linux-abi-tools/actions/workflows/ci.yml/badge.svg)](https://github.com/rustcommons/linux-abi-tools/actions/workflows/ci.yml)
![Rust 1.90+](https://img.shields.io/badge/Rust-1.90%2B-orange)
[![Code: MIT](https://img.shields.io/badge/code-MIT-blue)](LICENSE)
[![glibc data: LGPL-2.1-or-later](https://img.shields.io/badge/glibc_data-LGPL--2.1--or--later-blue)](https://github.com/rustcommons/glibc-abilists)
![Status: metadata prototype](https://img.shields.io/badge/status-metadata_prototype-yellow)

Rust tools for collecting and inspecting libc ABI metadata, starting with
**x86_64 Linux glibc 2.17+**. The ABI lists come from the **GNU C Library**
itself, collected from glibc's signed release tags by
[glibc-abilists](https://github.com/rustcommons/glibc-abilists). The project was
inspired by, and parts of its code adapted from,
[Zig's libc ABI tools](https://codeberg.org/ziglang/libc-abi-tools); credit to
the **Zig contributors** for the idea and design. See [NOTICE.md](NOTICE.md) for
complete attribution and the separate code and data licenses.

The companion [cargo-libc](https://github.com/rustcommons/cargo-libc) is the Cargo
subcommand built on these crates: it prepares the 2.17–2.44 providers itself
(see its [user guide](https://github.com/rustcommons/cargo-libc/blob/main/docs/USER-GUIDE.md)).

## What works today

- Parse historical grouped ABI lists and modern per-line versioned lists.
- Load exact release snapshots for eight libraries: `m`, `c`, `ld`, `resolv`,
  `pthread`, `dl`, `rt`, and `util`.
- Export deterministic TSV or query one symbol across those libraries.
- Preserve historical symbol versions and object sizes; recognize explicit TLS
  entries when present. Absolute version-node markers and `GCC_*` entries are skipped.
- Collect the selected ABI files from a clean glibc source checkout at an exact
  `glibc-MAJOR.MINOR` tag, with source commit and license files.
- Collect defined dynamic-symbol evidence directly from ELF64 x86_64 shared
  libraries: SONAME, versions/defaults, binding, visibility, IFUNC/TLS kinds,
  64-bit sizes, and observed shared ranges. See [ELF metadata](docs/ELF-METADATA.md).

The ABI lists for glibc **2.17 through 2.44** are compiled in from the
[glibc-abilists](https://github.com/rustcommons/glibc-abilists) crate (default
`bundled-glibc-data` feature; LGPL-2.1-or-later). Each comes unmodified from
glibc's release tag, with the tag's signature check and the file's blob id and
SHA-256 recorded there. A release label is not a claim of tested runtime
compatibility. Every release is covered by parser tests. To leave the LGPL
data out, build with `default-features = false` and pass `--data DIR`.
The default `provider-sources` feature adds
[glibc-provider-sources](https://github.com/rustcommons/glibc-provider) (glibc's
sources and headers, LGPL-2.1-or-later, and Linux UAPI headers) for `source` mode.
Build and tests require no Zig; the only dependencies are those two crates.

## Build and try it

Requires Rust/Cargo 1.90+; collection also requires Git. Run from this repository:

```sh
cargo test --locked
cargo run --locked -- glibc snapshot --release 2.22 --symbol memcpy
```

Output:

```text
release target                   library symbol version       kind size
2.22    x86_64-unknown-linux-gnu   c       memcpy GLIBC_2.2.5   F    -
2.22    x86_64-unknown-linux-gnu   c       memcpy GLIBC_2.14    F    -
```

The real output uses tabs. Omit `--symbol` to export the whole release.
No matching symbol produces a header-only result. Unknown/missing releases fail;
the tool never substitutes a newer release.

```sh
cargo install --path . --locked
libc-abi glibc snapshot --release 2.17 > glibc-2.17.tsv
libc-abi glibc export-data --output /path/to/glibc-data       # files + SOURCE.tsv + COPYING.LIB
libc-abi glibc snapshot --data /path/to/glibc-data --release 2.17
libc-abi glibc collect --source /path/to/glibc-checkout --output /path/to/new-data
libc-abi glibc elf --input /path/to/target/libc.so.6 > libc-elf.tsv
libc-abi libgcc stub --output /path/to/new-dir   # link-only libgcc_s.so.1 sources, GCC 4.8.5's interface
libc-abi glibc libraries --release 2.28 --input /path/to/lib64 > libraries.tsv
libc-abi provider prepare --glibc 2.28          # a link provider, built locally; prints its descriptor
libc-abi provider list
```

`libc-abi provider` prepares glibc link providers (stubs, startup objects,
nonshared archives, headers) for cargo-libc's `--provider` in a cache. The default
`source` mode compiles the recorded glibc sources of
[glibc-provider-sources](https://github.com/rustcommons/glibc-provider) with the
host's GCC, offline, and verifies every object against the qualified container
build; `container` mode runs that build locally. **[Using glibc providers](docs/USING-PROVIDERS.md)**
explains when to choose each mode, the cache, and offline and air-gapped use; the
design is in [PROVIDER-DELIVERY.md](docs/PROVIDER-DELIVERY.md#libc-abi-provider).

The installed executable carries the ABI lists; `--data` reads a directory with
the same layout instead (required when built without `bundled-glibc-data`).
Collection refuses an existing output release and validates the
inputs before writing. An I/O failure during writing can leave a partial new
directory; inspect it before retrying. Source tags identify a checkout, not its
authenticity: use a trusted source repository.

## Why exact release snapshots?

Symbols can move between libraries. The newer file may retain an old symbol
version while moving its library membership; inferring all history from the
latest file would put it in the wrong library for older targets. Tests cover
`pthread_sigmask` moving to libc in 2.32 and startup changes in 2.34.

glibc 2.17 is a **target release floor**, not a symbol-version filter.
`GLIBC_2.2.5` still belongs in a 2.17 or 2.22 snapshot.

## What comes next

Compact cross-release consolidation, Zig `abilists` binary interoperability,
a packaged release SDK and additional provider versions are future work.
ABI lists alone do not identify every default-version alias, binding/visibility
detail, or TLS property needed for a complete generator. Headers, startup code,
nonshared objects, compiler runtimes, and oldest-target runtime tests are separate
requirements. No glibc implementation is included.

See [ROADMAP.md](docs/ROADMAP.md), [ARCHITECTURE.md](docs/ARCHITECTURE.md),
and [VALIDATION.md](docs/VALIDATION.md).

For a hands-on comparison of obtaining real glibc inputs versus building them
from source, and a verified two-function generated libc experiment, see the
[two-path C example](examples/glibc-two-paths/README.md).
The [expanded emitter experiment](docs/STUB-GENERATION.md) additionally tests
both memcpy versions, object copy-relocation, IFUNC, TLS, and observed alias storage.
The [full-library follow-up](docs/FULL-LIBRARIES.md) generates all pinned public
glibc 2.17 exports in the eight selected libraries and passes C/Rust runtime checks.

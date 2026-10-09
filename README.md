![libc-abi-tools: an orange gear and release-to-ABI diagram](docs/assets/banner.svg)

# libc-abi-tools

**Target libc versions beyond your host.**

[![CI](https://github.com/rustcommons/libc-abi-tools/actions/workflows/ci.yml/badge.svg)](https://github.com/rustcommons/libc-abi-tools/actions/workflows/ci.yml)
![Rust 1.90+](https://img.shields.io/badge/Rust-1.90%2B-orange)
[![Code: MIT](https://img.shields.io/badge/code-MIT-blue)](LICENSE)
[![Data: upstream license](https://img.shields.io/badge/data-upstream_license-blue)](data/glibc/COPYING.abilists)
![Status: metadata prototype](https://img.shields.io/badge/status-metadata_prototype-yellow)

Rust tools for collecting and inspecting libc ABI metadata, starting with
**x86_64 Linux glibc 2.17+**. This is a port of the glibc portion of
[Zig's libc ABI tools](https://codeberg.org/ziglang/libc-abi-tools).
Credit to the **Zig contributors** and **GNU C Library contributors** for the
tooling and source data on which this project builds. See [NOTICE.md](NOTICE.md)
for complete attribution, the source pin, and separate code/data licenses.

The companion [cargo-libc](https://github.com/rustcommons/cargo-libc) coordinates
Cargo builds with an existing sysroot. These repositories are not integrated yet.

## What works today

- Parse historical grouped ABI lists and modern per-line versioned lists.
- Load exact release snapshots for eight libraries: `m`, `c`, `ld`, `resolv`,
  `pthread`, `dl`, `rt`, and `util`.
- Export deterministic TSV or query one symbol across those libraries.
- Preserve historical symbol versions and object sizes; recognize explicit TLS
  entries when present. Absolute version-node markers and `GCC_*` entries are skipped.
- Collect the selected ABI files from a clean glibc source checkout at an exact
  `glibc-MAJOR.MINOR` tag, with source commit and license files.

The repository includes 224 unmodified ABI lists from the pinned upstream's
**2.17 through 2.44 snapshot directories**. These are upstream labels, not a claim
about released glibc versions or tested runtime compatibility. Every snapshot is
covered by parser tests; [checksums and provenance](data/glibc/UPSTREAM.toml) are
committed with the data. Build and tests require no Zig or third-party Rust crates.

## Build and try it

Requires Rust/Cargo 1.90+; collection also requires Git. Run from this repository:

```sh
cargo test --locked --offline
cargo run --locked --offline -- glibc snapshot --data data/glibc --release 2.22 --symbol memcpy
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
cargo install --path . --locked --offline
libc-abi glibc snapshot --data data/glibc --release 2.17 > glibc-2.17.tsv
libc-abi glibc collect --source /path/to/glibc-checkout --output /path/to/new-data
```

Installing the executable does not install the data directory; pass `--data`
explicitly. Collection refuses an existing output release and validates the
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
stub shared libraries, and integration with `cargo-libc` are future work.
ABI lists alone do not identify every default-version alias, binding/visibility
detail, or TLS property needed for a complete generator. Headers, startup code,
nonshared objects, compiler runtimes, and oldest-target runtime tests are separate
requirements. No glibc implementation is included.

See [ROADMAP.md](docs/ROADMAP.md), [ARCHITECTURE.md](docs/ARCHITECTURE.md),
and [VALIDATION.md](docs/VALIDATION.md).

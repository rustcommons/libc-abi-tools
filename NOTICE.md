# Attribution and provenance

This project is a Rust port of the glibc portion of
[Zig's libc ABI tools](https://codeberg.org/ziglang/libc-abi-tools), created and
maintained by the **Zig contributors**. Their ABI collection workflow, library
selection, source data, format research, and explanation of library migrations
are the foundation of this work. This is an independent project with no implied
endorsement by Zig.

## Pinned upstream

- Repository: https://codeberg.org/ziglang/libc-abi-tools
- Commit: `9217cf899bf12bbb24a8a3de4cc2de682de0379d`
- Commit title: `update to Zig 0.17.0`
- Retrieved: 2026-10-09
- Browse the [pinned tree](https://codeberg.org/ziglang/libc-abi-tools/src/commit/9217cf899bf12bbb24a8a3de4cc2de682de0379d).

| Material | Origin | Use here |
| --- | --- | --- |
| Collection workflow | `glibc/collect.zig` | Rust collector restricted to eight x86_64 libraries; exact-tag/clean-tree checks added |
| Parsing and library selection | `glibc/consolidate.zig` | Modern F/D/A parsing and the eight library names; grouped pre-2.23 format added |
| Numeric version handling | `glibc/Version.zig` and the local cargo-glibc prototype | Strict Rust parsing; target-release validation separate from symbol versions |
| ABI migration explanation | `glibc/README.md` | Design and regression tests; compact consolidation not implemented yet |
| ABI lists | `glibc/2.17` through `glibc/2.44`, x86_64/64 only | 224 unmodified source files, preserving original paths |

## Licenses

The Rust code is MIT licensed; see [LICENSE](LICENSE). Reused/adapted Zig tooling
retains `Copyright (c) Zig contributors` and the complete upstream MIT notice in
[LICENSES/Zig-MIT.txt](LICENSES/Zig-MIT.txt).

The ABI lists originate in **GNU C Library (glibc)** and were collected by the
Zig team. They have a separate upstream-supplied license document:
[data/glibc/COPYING.abilists](data/glibc/COPYING.abilists), containing GPL version 2.
That document is copied unchanged. The project MIT license does not relicense
these data files. Retain their provenance and license when redistributing them
or derived datasets. No glibc implementation, headers, CRT objects, or runtime
libraries are included.

[UPSTREAM.toml](data/glibc/UPSTREAM.toml) records the pin and selected scope;
[SHA256SUMS](data/glibc/SHA256SUMS) covers every copied ABI list. Snapshot directory
labels are reproduced from upstream and do not establish release status or runtime
qualification. Exported TSV is derived from these separately licensed data.

The companion `cargo-libc` begins with the MIT-licensed `cargo-glibc` prototype,
whose original copyright notice is preserved in that repository.

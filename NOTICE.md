# Attribution and provenance

## Inspiration and design: Zig

The idea for this project, and much of its design, come from
[Zig's libc ABI tools](https://codeberg.org/ziglang/libc-abi-tools), created and
maintained by the **Zig contributors**: keeping glibc's ABI lists as versioned
data, the collection workflow, the library selection, the abilist formats, and
the explanation of how symbols migrate between libraries. This is an independent
Rust project with no implied endorsement by Zig.

Parts of the Rust code were adapted from Zig's tooling at commit
`9217cf899bf12bbb24a8a3de4cc2de682de0379d` (2026-10-09). Those parts retain
`Copyright (c) Zig contributors` and the upstream MIT notice in
[LICENSES/Zig-MIT.txt](LICENSES/Zig-MIT.txt):

| Material | Zig origin | Use here |
| --- | --- | --- |
| Collection workflow | `glibc/collect.zig` | Rust collector restricted to eight x86_64 libraries; exact-tag/clean-tree checks added |
| Parsing and library selection | `glibc/consolidate.zig` | Modern F/D/A parsing and the eight library names; grouped pre-2.23 format added |
| Numeric version handling | `glibc/Version.zig` and the local cargo-glibc prototype | Strict Rust parsing; target-release validation separate from symbol versions |
| ABI migration explanation | `glibc/README.md` | Design and regression tests; compact consolidation not implemented yet |

No data is taken from Zig.

## glibc ABI data: the GNU C Library

The ABI lists come from the **GNU C Library (glibc)** itself, collected from
glibc's repository (https://sourceware.org/git/glibc.git) at its signed release
tags `glibc-2.17` through `glibc-2.44` by the
[glibc-abilists](https://github.com/rustcommons/glibc-abilists) crate. That
crate records each tag's object, commit, and signature check, and each file's
path, git blob id, and SHA-256, and copies the files unmodified. They are
Copyright (C) Free Software Foundation, Inc. and the glibc contributors, under
the **GNU Lesser General Public License, version 2.1 or later** (glibc's
`COPYING.LIB`).

- `libc-abi-tools` depends on `glibc-abilists` through the default
  `bundled-glibc-data` feature, so a default build compiles the LGPL data in.
  Build with `default-features = false` to leave it out and read a data
  directory (`--data DIR`) instead.
- `libc-abi glibc export-data` writes the lists with `SOURCE.tsv` (tag and
  commit per release) and `COPYING.LIB` (the LGPL text). Keep both with any copy.
- No glibc implementation, headers, CRT objects, or runtime libraries are
  included.

Until 2026-10-10 this repository carried a copy of the same 224 x86_64 files
taken from Zig's repository. Before it was removed, every one of them was
verified to be byte-identical to the file in glibc's own release tag.

The cross-release matrices in `data/glibc-history` and the generated Markdown
tables in `docs/reference` are derived from these ABI lists and carry their
license, LGPL-2.1-or-later. Native ELF observations are labelled with their
real input profiles and hashes; historical tag/archive catalogs are evidence
inventories, not validated runtime support.

## Code license

The Rust code is MIT licensed; see [LICENSE](LICENSE). The MIT license does
not relicense the glibc data or anything derived from it.

The companion `cargo-libc` begins with the MIT-licensed `cargo-glibc` prototype,
whose original copyright notice is preserved in that repository.

## Workspace crates

The sections above cover the root `libc-abi-tools` crate. The other crates
carry their own attribution:

- [`crates/kernel-abi-tools/NOTICE`](crates/kernel-abi-tools/NOTICE): Linux
  kernel syscall and `madvise` data provenance (derived facts; no kernel
  source).
- `crates/linux-targets`: the distribution presets name distribution releases
  and their public container images, which were used to read glibc versions.
  No distribution software or image is included. Distribution names are
  trademarks of their owners.

# Attribution and provenance

## Inspiration and design: Zig

The idea for this project, and much of its design, come from
[Zig's libc ABI tools](https://codeberg.org/ziglang/libc-abi-tools), created and
maintained by the **Zig contributors**: keeping glibc's ABI lists as versioned
data, the collection workflow, the library selection, the abilist formats, and
the explanation of how symbols migrate between libraries. This is an independent
Rust project with no implied endorsement by Zig.

Parts of the Rust code were adapted from Zig's tooling at the commit recorded
below. Those parts retain `Copyright (c) Zig contributors` and the MIT license
under which that commit was published, copied in
[LICENSES/Zig-MIT.txt](LICENSES/Zig-MIT.txt):

| Material | Zig origin (git blob at the commit) | Use here |
| --- | --- | --- |
| Collection workflow | `glibc/collect.zig` (`92171fc82a571d00d9823150afcd8e209b73706c`) | Rust collector restricted to eight x86_64 libraries; exact-tag/clean-tree checks added |
| Parsing and library selection | `glibc/consolidate.zig` (`f9e9549098fb416a9d2394399bd355ee39847f51`) | Modern F/D/A parsing and the eight library names; grouped pre-2.23 format added |
| Numeric version handling | `glibc/Version.zig` (`3681cd038fa0236f1e3ff754fff0f681affb3831`) and the local cargo-glibc prototype | Strict Rust parsing; target-release validation separate from symbol versions |
| ABI migration explanation | `glibc/README.md` (`21cb228f2639e96cbf83d36705b9debedb16a01d`) | Design and regression tests; compact consolidation not implemented yet |

No data is taken from Zig.

### Zig attribution record

The attribution and license above rest on this exact upstream state. It records
the license under which that commit was published, so the basis for our use is
preserved even if Zig's repository or license changes later.

| Item | Value |
| --- | --- |
| Repository | https://codeberg.org/ziglang/libc-abi-tools |
| Commit | `9217cf899bf12bbb24a8a3de4cc2de682de0379d` ([browse](https://codeberg.org/ziglang/libc-abi-tools/src/commit/9217cf899bf12bbb24a8a3de4cc2de682de0379d)) |
| Commit title | `update to Zig 0.17.0`; the repository has no tags |
| Zig release targeted | **0.17.0**, Zig's latest release: tag `0.17.0` (tag object `afee4225358129f909e01dfaed265dca0977befe`), commit `7647adab80dd088f4de3610fd245915a912eb6ad`, released 2026-10-01 (tagged 2026-10-01T14:58:17-07:00; date confirmed by https://ziglang.org/download/index.json). Its `LICENSE` is the same git blob as below. |
| Author, date | Alex Rønne Petersen, 2026-10-02T05:17:51+02:00 (committed 05:18:04+02:00) |
| Tree | `05d2e09a1257d3630c7f7a886df8193c1a229f5c` |
| `LICENSE` | git blob `9ce01373c89082fd81e8ea0793aab0978140526b`, SHA-256 `5c537d6853e005298a285d508cff9ac7192cea23576c840d485b2b586a7ff177`, "The MIT License (Expat)", `Copyright (c) Zig contributors` |
| Our copy | [LICENSES/Zig-MIT.txt](LICENSES/Zig-MIT.txt) is byte-identical (same git blob id) |
| First retrieved | 2026-10-09 |
| Re-verified | 2026-10-10: the commit was the tip of Codeberg's default branch, and the same commit object is also served by https://github.com/ziglang/libc-abi-tools (whose default branch, `6878b563bd0620c626dc92c7b641a903b648c592` from 2025-10-18, is an ancestor of it) |

For the record, Zig's tree at that commit also shipped `glibc/COPYING.abilists`
(git blob `d159169d1050894d3ea3b98e1c965c4058208fe1`, the GNU GPL version 2) next
to its copies of glibc's ABI lists. This project used those copies until
2026-10-10 and now takes the lists from glibc itself (below).

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

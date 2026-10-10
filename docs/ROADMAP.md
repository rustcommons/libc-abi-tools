# Roadmap

Plan established 2026-10-09. Start with x86_64 Linux and glibc target releases 2.17+.

## Resume here: first-release goal and next decision

The goal is to provide the compatibility files needed to **build against a
selected libc version that is not installed on the build host**. Initially this
means dynamically linked x86_64 Linux executables targeting glibc 2.17 or later.
`libc-abi-tools` supplies ABI metadata and the future compatibility artifacts;
`cargo-libc` selects those artifacts and coordinates Cargo builds. A complete
release for this goal must go beyond collecting and inspecting metadata.

**Current checkpoint:** milestone 1 is implemented and tested. Exact-release ABI
snapshots, source collection, attribution, and a pinned upstream are in place.
Compact consolidation, authenticated SDK packaging, and additional provider versions
are not implemented. Native ELF evidence now drives all 2915 pinned public numeric
GLIBC 2.17 definitions across eight generated libraries, using both real input
sources. C data-version/copy-relocation tests and Rust allocation/thread/TLS
destruction/unwinding tests pass with both old runtimes on WSL. General alias
semantics, complete SDK packaging, and runtime qualification on an old glibc 2.17
distribution or SLES 12 SP5/glibc 2.22 remain open. See
[VALIDATION.md](VALIDATION.md) and [FULL-LIBRARIES.md](FULL-LIBRARIES.md).

The experimental local 2.17 provider now connects these artifacts to Cargo;
both real input routes pass with Rust 1.99 while keeping host build units separate.

**Release sourcing decision remains open:** choose where compatible real startup objects,
`libc_nonshared.a`, headers, and compiler runtime inputs will come from. Generated
ABI stub libraries cannot supply these. Compare a pinned SDK/sysroot source,
reproducible builds from pinned upstream sources, or a documented combination.
Record which inputs we generate, obtain, or require from the user, together with
their versions, provenance, redistribution terms, and update process. This
choice remains open; do not assume the host's installed files are compatible.

Then agree on the first release's exact supported glibc range, build-host
requirements, native-dependency scope, and packaging. A 2.17 target floor does
not qualify every later snapshot. Other architectures and libc families remain
outside the initial release.

Work in this order:

1. Resolve the input-source decision and confirm the release acceptance criteria.
2. Complete milestone 2: historical consolidation, missing symbol metadata, and
   the versioned artifact contract with cargo-libc.
3. Complete milestone 3: generate and verify link-only libraries, assemble the
   real inputs, and integrate the provider with cargo-libc.
4. Run the target runtime matrix and package only the configurations that pass.

## First-release acceptance checklist

These gates define the intended end-to-end release; exact versions and test
environments must be agreed before claiming support.

- [ ] A documented command selects a supported glibc version on a newer Linux
  host and produces a usable link environment without Zig.
- [ ] Every generated/downloaded input has a pinned source, integrity checks,
  preserved license notices, and a documented reproduction procedure.
- [ ] ABI comparisons validate library membership, symbol versions, default
  aliases, binding/visibility, and object/TLS properties against reference data.
- [ ] The environment includes or explicitly resolves compatible CRT, nonshared,
  header, and compiler-runtime inputs without silently using incompatible host files.
- [x] cargo-libc builds with the experimental 2.17 provider while keeping host build scripts and
  procedural macros on their host configuration.
- [ ] Representative programs run on glibc 2.17 and SLES 12 SP5/glibc 2.22;
  coverage includes threads/TLS, unwinding, C FFI, and the agreed native dependencies.
- [ ] Negative tests reject unavailable newer APIs and incompatible inputs;
  generated stub libraries are used only for linking, never deployed as runtime libc.
- [ ] Hosted CI, the declared MSRV, installation instructions, and release
  packaging pass; supported configurations and remaining limits are documented.

## 1. Repository foundation and metadata port

- [x] Commit CLAUDE.md/AGENTS.md and imported Git conventions first.
- [x] Pin Zig libc-abi-tools and preserve source/data attribution and licenses.
- [x] Import only the eight selected x86_64 library lists for snapshots 2.17 onward.
- [x] Parse both historical grouped and modern flat ABI lists.
- [x] Preserve library, symbol version, function/object kind, and size.
- [x] Export deterministic exact-release snapshots and symbol queries.
- [x] Collect selected files from clean, exactly tagged glibc source checkouts.
- [x] Test all 28 pinned snapshots, the 2.17 floor, and library migrations.
- [x] Add README artwork and CI configuration.

## 2. Cross-release consolidation

- [ ] Port upstream's historical migration corrections and inclusion model.
- [ ] Specify a versioned data contract for cargo-libc.
- [ ] Implement bounded Zig abilists binary reading/writing and compare semantic
  output against the pinned Zig implementation; do not assume order is canonical.
- [ ] Establish authoritative binding, visibility, default-version, alias, and TLS metadata.
- [x] Collect native ELF64 x86_64 definition metadata and compare all eight
  libraries in both glibc 2.17 input trees with readelf and pinned ABI lists.
- [ ] Automate upstream refresh with checksums, provenance, and reviewed diffs.

## 3. Generate and validate link environments

- [x] Demonstrate a two-function generated libc with both real input sources,
  ELF checks, old-libc runtime checks on WSL, and a rejected newer-symbol test.
- [x] Expand selected generation to multi-version functions, IFUNC, sized
  objects/TLS, and observed shared storage, with old-runtime C/fixture checks.
- [x] Generate and independently verify the entire pinned public numeric glibc
  2.17 ABI in all eight libraries with both real input sources; pass direct Rust
  allocation/thread/TLS destruction/unwind and C historical-data-view experiments.
- [x] Emit x86_64 assembly/version scripts and assemble all eight pinned glibc
  2.17 public ABI stub DSOs in the experimental workflow.
- [x] Verify versions/defaults, SONAMEs, kinds, object sizes, binding/visibility,
  data views, and C/Rust resolution for the two qualified 2.17 input trees.
- [ ] Source/build compatible CRT, libc_nonshared.a, headers, and compiler runtimes.
- [x] Integrate glibc 2.17 with cargo-libc behind explicit `--provider` selection;
  preserve host build scripts/procedural macros and audit link/runtime inputs.
- [ ] Test real binaries on glibc 2.17 and SLES 12 SP5/glibc 2.22, including
  threads/TLS, unwinding, native dependencies, and rejected newer symbols.
- [ ] Establish redistribution requirements for generated artifacts and their inputs.

## 4. Expand deliberately

- [ ] Add architectures with their own mappings, sizes, and runtime validation.
- [ ] Add other libc families when their compatibility model and use case are specified.
- [ ] Consider direct ELF emission after the assembler/linker route is validated.

## Repository presentation and release work

| Field | Prepared value |
| --- | --- |
| Tagline | Target libc versions beyond your host. |
| Description | Rust tools for collecting and inspecting libc ABI metadata, starting with glibc. |
| Topics | rust, libc, glibc, abi, elf, linux, toolchain, cross-compilation, zig |
| README image | docs/assets/banner.svg; original editable gear and ABI diagram |
| Badges | CI, Rust 1.90+, MIT code, separate upstream data license, prototype status |

- [ ] Apply description/topics to GitHub settings; describe generation as planned.
- [ ] Observe hosted CI and the MSRV job after the first push.
- [ ] Decide packaging and crate publication policy; verify crate-name availability.
- [ ] Publish only after the advertised milestone passes its validation gates.

Repository settings and releases remain separate from the authorized development push.

## GitHub Actions and per-version setup

WSL is a development environment. Hosted CI is the reproducible gate. The main
workflow pins Rust 1.99.0 on Windows/Linux, retains the Rust 1.90 minimum check,
and gives all 28 vendored snapshot labels checksum/deterministic-export jobs.
Snapshot coverage does not establish released or runtime-qualified environments.

The 2.17 workflow builds both pinned real input trees in a disposable CentOS
builder, checks native ELF metadata and complete generated DSOs, and exercises
Cargo integration separately with Clang 21 and 23. Modern host tools still use
compatible target CRT/nonshared files and the old real unwinder. Building glibc
2.17 itself retains the qualified historical GCC builder; changing that compiler
is a separate source-build task. GNU binutils remain the qualified assembler/linker;
LLD needs separate qualification. Clang 23 is required without silent fallback;
Use the explicit `llvm-toolchain-noble-23` suite; the moving development suite
already publishes Clang 24. Record actual package
revisions because those APT inputs can move.

- [x] Configure per-snapshot CI and 2.17 integration with Rust 1.99/Clang 21/23.
- [x] Land the companion provider integration and pin its locally tested commit
  `904d58180bbf75fb4ac1b7d6ae1f04ef7bd0c862` in CI.
- [ ] Observe hosted jobs after pushing; local tests do not establish hosted success.
- [ ] Add pinned headers/CRT/nonshared/compiler runtime, full-ABI generation,
  Cargo positive/negative tests, and real runtime jobs for each remaining version.
- [ ] Include native C dependencies, TLS/unwind, historical data views, and actual
  target distro/kernel qualification in each release's acceptance evidence.
- [ ] Package authenticated relocatable SDKs and publish only qualified versions.

| Snapshot label | Snapshot CI configured | Remaining real input / provider / runtime setup |
| --- | --- | --- |
| 2.17 | Yes | Both input routes and Cargo tested locally; hosted Clang 21/23 and old distro/kernel qualification pending |
| 2.18 | Yes | Pin inputs; generate provider; add runtime job |
| 2.19 | Yes | Pin inputs; generate provider; add runtime job |
| 2.20 | Yes | Pin inputs; generate provider; add runtime job |
| 2.21 | Yes | Pin inputs; generate provider; add runtime job |
| 2.22 | Yes | Pin inputs; generate provider; add runtime job; qualify SLES 12 SP5 |
| 2.23 | Yes | Pin inputs; generate provider; add runtime job |
| 2.24 | Yes | Pin inputs; generate provider; add runtime job |
| 2.25 | Yes | Pin inputs; generate provider; add runtime job |
| 2.26 | Yes | Pin inputs; generate provider; add runtime job |
| 2.27 | Yes | Pin inputs; generate provider; add runtime job |
| 2.28 | Yes | Pin inputs; generate provider; add runtime job |
| 2.29 | Yes | Pin inputs; generate provider; add runtime job |
| 2.30 | Yes | Pin inputs; generate provider; add runtime job |
| 2.31 | Yes | Pin inputs; generate provider; add runtime job |
| 2.32 | Yes | Pin inputs; generate provider; add runtime job |
| 2.33 | Yes | Pin inputs; generate provider; add runtime job |
| 2.34 | Yes | Pin inputs; implement merged-library layout; add runtime job |
| 2.35 | Yes | Pin inputs; implement merged-library layout; add runtime job |
| 2.36 | Yes | Pin inputs; implement merged-library layout; add runtime job |
| 2.37 | Yes | Pin inputs; implement merged-library layout; add runtime job |
| 2.38 | Yes | Pin inputs; implement merged-library layout; add runtime job |
| 2.39 | Yes | Pin inputs; implement merged-library layout; add runtime job |
| 2.40 | Yes | Pin inputs; implement merged-library layout; add runtime job |
| 2.41 | Yes | Pin inputs; implement merged-library layout; add runtime job |
| 2.42 | Yes | Pin inputs; implement merged-library layout; add runtime job |
| 2.43 | Yes | Verify release/source availability, then add inputs/provider/runtime |
| 2.44 | Yes | Verify release/source availability, then add inputs/provider/runtime |

The last two rows track vendored labels without asserting upstream availability.
Runtime jobs must fail on incompatible inputs, newer symbols, host shared-library
leakage, or execution of generated stubs.

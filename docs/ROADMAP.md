# Roadmap

Plan established 2026-10-09. Start with x86_64 Linux and glibc target releases 2.17+.

See [GLIBC-COVERAGE.md](GLIBC-COVERAGE.md) for current per-library coverage,
test-stage guarantees, input profiles, and behavioral gaps.

## Team handoff (2026-10-10)

Versioned targets (F01) and normal Cargo flags/workspace fixtures (F02) are
qualified for glibc 2.17/2.18/2.22, including both 2.17 routes, with Rust 1.99
and Clang 21/24. Latest qualified producer code is c442310; CI pins consumer
6b072609e1eafd27904ee3102bb40d2374f63270.
[All nine jobs passed](https://github.com/rustcommons/libc-abi-tools/actions/runs/38077381465).

Next coordinator task is F03, cargo libc check; its checklist is in the
[companion roadmap](https://github.com/rustcommons/cargo-libc/blob/main/docs/ROADMAP.md).
The next independent provider rollout is glibc 2.19, following the provider
implementation plan below. Historical research and the symbol matrix remain
separate from provider/runtime qualification. Public authenticated SDK releases
and additional platform/distribution/kernel qualification remain unfinished.

Read CLAUDE.md before working. Preserve exact source/tool/consumer pins and update
qualification references after new code passes. The feature review is an inventory
for individual decisions, not a blanket commitment to match all Zig features.

## Resume here: first-release goal and next decision

The goal is to provide the compatibility files needed to **build against a
selected libc version that is not installed on the build host**. Initially this
means dynamically linked x86_64 Linux executables targeting glibc 2.17 or later.
`libc-abi-tools` supplies ABI metadata and the future compatibility artifacts;
`cargo-libc` selects those artifacts and coordinates Cargo builds. A complete
release for this goal must go beyond collecting and inspecting metadata.

**Current checkpoint:** milestone 1 is implemented and tested. Exact-release ABI
snapshots, source collection, attribution, and a pinned upstream are in place.
Beyond the tested 2.17/2.18/2.22 providers, compact consolidation, authenticated SDK packaging, and additional releases
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

The [history and standard test design](GLIBC-HISTORY.md) now organizes this work.
The [interface tables](reference/GLIBC-SYMBOLS.md) identify every covered tuple as
`symbol@GLIBC_version`, link to source/profile evidence, and distinguish a new
default from compatibility retention or removal. The historical research scope
extends to glibc's beginnings without extending the supported provider floor.

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
- [x] Observe hosted Windows/Linux CI, all snapshot jobs, and the MSRV check.
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
Cargo integration separately with Clang 21 and 24. Modern host tools still use
compatible target CRT/nonshared files and the old real unwinder. Building glibc
2.17 itself retains the qualified historical GCC builder; changing that compiler
is a separate source-build task. GNU binutils remain the qualified assembler/linker;
LLD needs separate qualification. The primary Clang 24 job intentionally uses
the development snapshot from `llvm-toolchain-noble`, with a mandatory major-version
check so a future feed change fails visibly. Clang 21 remains a release-compiler
check. Record exact package revisions because these APT inputs can move.

The first hosted companion run with the corrected package feed passed Rust
1.99 Windows/Linux checks, Rust 1.90, and conventional sysroot linker smoke tests
under Clang 21 and 23. The selected CI matrix is now Clang 21/24 at the user's request;
glibc 2.17 provider/runtime qualification remains a separate gate.

- [x] Configure per-snapshot CI and 2.17 integration with Rust 1.99/Clang 21/24.
- [x] Land the companion provider integration and pin the extended consumer revision
  `edfca5b9f9fb811069f302e8d2d5ef2f99f7fd33` in CI (including 2.18 selection).
- [x] Observe hosted Rust 1.99, Clang 21/24, all 28 snapshots, and both real 2.17 input routes passing.
- [ ] Add pinned headers/CRT/nonshared/compiler runtime, full-ABI generation,
  Cargo positive/negative tests, and real runtime jobs for each remaining version.
- [ ] Include native C dependencies, TLS/unwind, historical data views, and actual
  target distro/kernel qualification in each release's acceptance evidence.
- [ ] Package authenticated relocatable SDKs and publish only qualified versions.

| Snapshot label | Snapshot CI configured | Remaining real input / provider / runtime setup |
| --- | --- | --- |
| 2.17 | Yes | Both input routes and Cargo tested locally; hosted Clang 21/24 passed; old distro/kernel qualification pending |
| 2.18 | Yes | Pinned GNU source inputs, eight libraries, Cargo, exhaustive links, and real runtime tests passed locally and in Clang 21/24 hosted jobs; distro/kernel qualification pending |
| 2.19 | Yes | Pin inputs; generate provider; add runtime job |
| 2.20 | Yes | Pin inputs; generate provider; add runtime job |
| 2.21 | Yes | Pin inputs; generate provider; add runtime job |
| 2.22 | Yes | Source inputs, all eight libraries, Cargo, and Clang 21/24 hosted runtime jobs passed; qualify SLES 12 SP5 |
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

## glibc 2.22 checkpoint

- [x] Pin and source-build GNU 2.22 headers, CRT, nonshared files, and runtime.
- [x] Verify all 2922 pinned public numeric definitions across eight libraries.
- [x] Extend the v1 descriptor and Cargo selection without changing 2.17 behavior.
- [x] Pass local Cargo host build/proc-macro, thread, real runtime, and path checks.
- [x] Run both fmemopen versions under 2.22; reject the 2.22 interface under 2.17.
- [x] Configure Rust 1.99 / Clang 21 and 24 CI for both releases.
- [x] Observe all four hosted provider jobs with the extended consumer passing.
- [ ] Qualify actual SLES 12 SP5 inputs, native dependencies, and target kernels.

The [2.22 provider](GLIBC-2.22.md) uses a generic source-built GNU runtime.
Distribution qualification and authenticated SDK packaging remain separate gates.

## Additional provider implementation plan

The provider workflow now prepares each release once and transfers checked
inputs to Rust 1.99/Clang 21 and 24 application jobs. ABI probes are separate
from compiler-specific application checks; both 2.17 routes are preserved.
See [CI and release artifacts](CI-AND-RELEASE-ARTIFACTS.md).

- [x] Split preparation/ABI auditing from Rust/C application integration.
- [x] Transfer checksummed input inventories with permissions/relative links and rematerialize descriptors on each runner.
- [x] Preserve matching NSS runtime modules and audit initialized dynamic modules after relocation.
- [x] Observe all three preparation and six Rust/Clang application jobs passing at `8939827` ([run](https://github.com/rustcommons/libc-abi-tools/actions/runs/38069527581)).
- [x] Add wrapper parsing for targets such as `x86_64-unknown-linux-gnu.2.17`, forwarding the normal Rust triple and selecting the exact provider.
- [ ] Implement authenticated user SDK packaging, provider discovery/download, cache identity, and install contracts before publishing releases.

The [GNU 2.18 provider](GLIBC-2.18.md) now passes all eight-library native/stub
audits, exhaustive accepted/rejected/default linking, Cargo host/target checks,
real-library behavior, and pthread GLIBC_2.18 boundaries. All six hosted
Clang 21/24 provider jobs passed at `75945c2` with pinned consumer `edfca5b`.
The historical reference now includes four native profiles. Establish 2.19 next;
distribution/kernel and SDK qualification remain separate gates.

Follow [GLIBC-PROVIDER-PLAN.md](GLIBC-PROVIDER-PLAN.md) for the ordered rollout:
fill 2.18–2.21 first, then 2.23–2.33, qualify the 2.34 library merger, and
extend subsequent releases individually. Each provider must pass the same
input, ABI, relocation, Cargo, real runtime, and hosted CI gates.

## Historical matrix and standardized conformance

- [x] Generate the complete covered-snapshot union: 4671 interfaces across 28 labels.
- [x] Document all adjacent membership/kind/size differences and measured native changes.
- [x] Preserve 11673 native observations with input provenance and default/compatibility flags, including the new GNU 2.18 profile.
- [x] Publish generated Markdown interface tables and machine-readable matrices.
- [x] Inventory early official archives/patches referring to 1.00 and 71 upstream tag candidates.
- [x] Model unknown evidence separately from known absence; avoid guessing introduction/defaults.
- [x] Use the matrix for common version-boundary tests and add an ordinary default-binding probe.
- [x] Add CI regeneration checks for all generated reference outputs.
- [x] Observe hosted reference regeneration, all 28 snapshot jobs, and all four 2.17/2.22 Clang 21/24 conformance jobs passing.
- [ ] Classify historical tags, patches, releases, target applicability, and evidence completeness.
- [ ] Pin/archive hashes and recover missing early source material; record unrecoverable gaps.
- [ ] Normalize pre-2.17 x86_64 evidence and early target/unversioned history with dedicated adapters.
- [ ] Extend scope to other glibc libraries and architectures with explicit completeness evidence.
- [ ] Collect native defaults/properties for every supported and historically covered profile.
- [x] Generate exhaustive isolated-DSO function/object link probes and measured default checks from the matrix; test TLS relocation machinery with a synthetic fixture (current union has no TLS rows).
- [x] Observe the expanded exhaustive suite passing in all four hosted provider jobs at `c3faf20` ([run](https://github.com/rustcommons/libc-abi-tools/actions/runs/38064818474)).
- [ ] Add behavioral/header/feature-change ledgers with primary source references and semantic tests.
- [ ] Apply the same artifact/runtime/distribution checks to every subsequently qualified provider.
- [x] Add focused real-runtime behavior suites for libm, libdl, librt, libutil, and libresolv; all 15 local profile/library cases pass.
- [x] Qualify the new behavior suites in all four hosted Clang 21/24 provider jobs at `a055e8f` ([run](https://github.com/rustcommons/libc-abi-tools/actions/runs/38065649690)).
- [ ] Expand semantic boundary coverage beyond the focused library behavior suites.
- [ ] Inventory additional glibc libraries/modules and establish explicit scope before expanding the matrix.

## Versioned-target checkpoint

- [x] Pin consumer ed4c997 and qualify suffix selection for 2.17/2.18/2.22 with Rust 1.99 and Clang 21/24, including both 2.17 routes.
- [x] Verify ordinary Rust artifact paths, host units, real-loader execution, provider/version conflicts and too-new-symbol rejection ([run](https://github.com/rustcommons/libc-abi-tools/actions/runs/38076305526)).

Provider acquisition and authenticated release SDKs remain open.

## Cargo workflow checkpoint (F02)

- [x] Pin consumer 6b07260 and qualify workspace/exclusion, package/bin/example selection, features/no-default-features, custom/release profiles, locked/offline and fresh cached output for 2.17/2.18/2.22 with Rust 1.99 and Clang 21/24, including both 2.17 routes.
- [x] Execute all three workflow fixtures with matching real loaders and reject an existing executable not reported by the current invocation.

Producer c442310: [all nine jobs passed](https://github.com/rustcommons/libc-abi-tools/actions/runs/38077381465). See the companion [Cargo workflow reference](https://github.com/rustcommons/cargo-libc/blob/main/docs/CARGO-WORKFLOWS.md). Workspace-wide inspection, arbitrary flag merging and new link modes remain separate work.

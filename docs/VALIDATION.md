# Validation — bootstrap milestone

## Provider contract and Cargo integration

The local version-1 exporter and companion `--provider` mode passed in WSL with
Rust 1.99.0, Clang 18.1.3, and GNU binutils 2.42 against both glibc 2.17 input
trees. Cargo ran host build scripts/procedural macros and the real 2.17 loader
ran the target thread example. Their highest direct GLIBC symbol requirement was 2.16,
without RPATH/RUNPATH. Independent link-map/runtime checks confirmed generated
DSOs at link time and real target glibc/compatible unwinder at runtime. Mismatched
provider versions failed; SHA-256 executable/input evidence is retained in logs.

Hosted qualification passed on 2026-10-10 for commit
`084d2b6316a3af6ab16f3d25cdd67187e0b852c3`, consuming cargo-libc commit
`7980d63873282cb9fef68bcaf08f8123546387ac`:

| Hosted gate | Result |
| --- | --- |
| Rust 1.99 Windows/Linux tests, formatting, and linting | Passed |
| Rust 1.90 all-targets minimum-version check | Passed |
| All 28 vendored ABI snapshot jobs | Passed |
| glibc 2.17 provider, Clang 21.1.8 | Passed with both real input routes |
| glibc 2.17 provider, Clang 24.0.0 development snapshot | Passed with both real input routes |

[ABI/host CI](https://github.com/rustcommons/libc-abi-tools/actions/runs/38055872899),
[provider/runtime CI](https://github.com/rustcommons/libc-abi-tools/actions/runs/38055872917),
and [companion CI](https://github.com/rustcommons/cargo-libc/actions/runs/38055844632)
retain the checks and downloadable evidence. The Clang 24 package was
`1:24~++20260917082238+28f36e6d3912-1~exp1~20260917082250.1854`; Clang 21 was
`1:21.1.8~++20251221032922+2078da43e25a-1~exp1~20251221153059.70`.
Both used GNU binutils 2.42. Per job, 5830 definitions across 16 generated DSOs
were independently checked. Cargo host build scripts/procedural macros, target
threads, wrong-version rejection, and link/runtime path audits passed. The broader
GNU-linker experiment also passed C historical data views and Rust allocation,
TLS destruction, and unwinding under both real 2.17 loaders.

The highest directly required GLIBC symbol version in these Cargo examples is
`GLIBC_2.16`. This is a property of their imported interfaces, not qualification
for glibc 2.16: the supported/tested target remains 2.17. Hosted execution used a
modern Linux kernel; actual old distro/kernel and SLES qualification remain open.
See [the trusted local contract](PROVIDER-CONTRACT.md) and [per-version roadmap](ROADMAP.md).

Validated 2026-10-09 with Rust 1.99.0 on Windows and Rust 1.98.1 in Ubuntu WSL.

| Check | Result |
| --- | --- |
| `cargo test --locked --offline` | 10 tests passed on Windows and Linux |
| `cargo fmt --all -- --check` | Passed |
| `cargo clippy --all-targets --locked --offline -- -D warnings` | Passed on Windows and Linux |
| Pinned source-data integrity | All 224 SHA-256 checksums match |
| Release coverage | Every snapshot from 2.17 through 2.44 parses all eight selected libraries |
| Historical versus modern format | Equivalent source rows produce identical records |
| 2.17 baseline | Preserves both memcpy versions and GLIBC_2.2.5 startup symbols |
| Library migration | pthread_sigmask membership differs correctly across 2.31/2.32 |
| Startup version change | GLIBC_2.34 __libc_start_main absent on 2.22, present on 2.34 |
| Collector | Fixture Git checkout verifies exact tags, clean tree, byte preservation, and overwrite refusal |
| CLI | Real 2.22 symbol query succeeds; ambiguous/unknown options fail |

Collector tests use a temporary local Git fixture. Collection from a full official
glibc checkout has not been independently exercised. The vendored data comes
from the pinned Zig tree, not a new glibc build.

Rust 1.90 is the declared minimum; an MSRV check is configured in CI but was not
run locally. Hosted CI has not run before the first push. At bootstrap there were
no generated DSOs or runtime qualification results. Passing metadata tests does not establish
that an executable runs on SLES or glibc 2.17.

## Real input sourcing experiment

On 2026-10-09, the [two-path C example](../examples/glibc-two-paths/README.md)
compiled the same non-PIE executable on Ubuntu 24.04 WSL with GCC 13.3.0 using
both extracted CentOS 7 glibc 2.17 inputs and upstream glibc 2.17 built with
CentOS GCC 4.8.5. Both executables ran with their respective old loaders and
libc, requiring `GLIBC_2.2.5` for `puts` and `__libc_start_main`. Link maps show
real CRT inputs and `libc_nonshared.a(elf-init.oS)` in both builds.

The normal Ubuntu control requires `__libc_start_main@GLIBC_2.34` and fails under
the extracted glibc 2.17 runtime. The source recipe includes a narrow workaround
for GCC 4.8 implicit-header dependency generation, with the patch recorded in
the experiment logs. Shell syntax checks and `git diff --check` passed.

This used real libc at link and runtime, not generated stubs. It tests one C
program on a newer WSL kernel, not an old distribution, SLES, Rust integration,
threads/TLS, unwinding, native dependencies, or reproducible release packaging.

## Minimal generated libc experiment

On 2026-10-09, `examples/minimal_glibc_stub.rs` generated x86_64 assembly and a
version script for only `puts` and `__libc_start_main` from the pinned glibc 2.17
snapshot. Both exports use default version `GLIBC_2.2.5`; weak `puts`, global
startup binding, and default visibility were verified against both real ELF
libraries. Selected reference exports and library hashes are in the local
`logs/stub-real-elf-evidence.tsv`.

Both prebuilt/source routes linked the C example using the generated DSO,
explicit real CRT objects, and `libc_nonshared.a`, with no real shared libc
input. Both ran successfully using the respective real glibc 2.17 loader/libc.
Checks verified SONAME `libc.so.6`, required versions, the absence of RPATH and
RUNPATH, link inputs, and runtime library selection. Repeated assembly/linking
produced byte-identical stubs on this toolchain. A `memfd_create` negative test
failed to link in both routes; pinned 2.17/2.27 snapshots confirm its absence
and newer version respectively.

This is a two-function Cargo example, not general generation. It adds no public
generator API or main CLI command and makes no full ABI/alias/TLS or runtime
qualification claim. The stub functions trap if executed; all runtime checks
use real libc on the newer WSL kernel.

Validation passed on Windows and Ubuntu WSL: `cargo fmt --all -- --check`,
`cargo test --all-targets --locked --offline` (10 existing tests plus 2 example
scope-validation tests), and `cargo clippy --all-targets --locked --offline --
-D warnings`. The default locked/offline test command also passed on Windows;
the example tests were explicitly run. Shell syntax and whitespace checks passed.
Current experiment logs and generated artifacts are retained in the ignored
`validation-output/glibc-two-paths` directory.

## Native ELF metadata collection

On 2026-10-09, the native `glibc elf --input FILE` collector read all eight
selected libraries in both real glibc 2.17 trees. All 6035 defined dynamic
symbols matched GNU readelf for name/version/default, type, binding, visibility,
section/value, and 64-bit size. Public numeric GLIBC membership, normalized kind,
and object/TLS sizes had zero differences from the pinned 2.17 ABI snapshot.
IFUNC is retained in ELF metadata and normalized to F only for abilist comparison.
Private/unversioned definitions are retained without treating them as public ABI.

The reports include raw readelf output, input/output SHA-256 hashes, input-source
provenance, and explicit pinned ABI differences. Same-range groups are observations
within each file, not semantic aliases. Complete alias rules and other glibc
releases remain unqualified. See [ELF metadata](ELF-METADATA.md) for bounds/schema.

The original committed binutils fixture tests multi-version exports, weak/protected
properties, object/TLS sizes, IFUNC, imports, and shared ranges. Tests reject table
corruption and every byte truncation without panics and preserve 64-bit size fields.
Windows locked/offline tests passed (15 integration tests plus 3 example tests);
Linux all-target tests passed (18). Formatting and warning-free all-target Clippy
passed on both platforms. The generator now derives bindings from reference ELF;
both input routes still pass the old-runtime and newer-symbol rejection checks.

Reports are retained under `validation-output/glibc-two-paths/elf-metadata`;
current selected generator evidence is in `generated-stub/selected-elf.tsv`.

## Expanded selected symbol generation

On 2026-10-09, the experimental `stub::render` emitter generated multi-version
FUNC/IFUNC/OBJECT/TLS definitions with observed binding/visibility, sized BSS/TBSS,
and declared section alignment. Exact observed nonzero shared ranges were retained;
partial/incompatible overlaps and unsupported properties were rejected.

Both real glibc 2.17 input routes generated seven definitions from six libc names.
Their C executables called `memcpy@GLIBC_2.2.5` and default `memcpy@GLIBC_2.14`,
then used `stdout` via a copy relocation. Both ran successfully with the respective
real old loader/libc; memfd_create still failed to link. Link maps included only
generated shared libraries plus explicit real startup/nonshared/compiler objects.

An original fixture additionally checked both versions of foo, IFUNC dispatch,
protected weak binding, TLS initialization/read/write, data contents, and a
source-defined alias's address identity. Both fixture executables ran with each
old loader/libc and the original real fixture, using only generated DSOs at link
time. ELF audits confirmed TPOFF64 TLS and COPY data relocations, export versions,
types, sizes, binding, visibility, alignment, SONAME, and shared ranges.

All 6035 real definitions still matched readelf after adding section-alignment
collection, with zero public pinned ABI differences. The committed fixture was
regenerated with explicit pointer/TLS alignment and verified SHA-256 hashes.
Windows tests passed (18 integration tests plus 3 original example tests); Linux
all-target tests passed (21). Formatting and warning-free all-target Clippy passed.

All four executables used real runtime libraries on the newer WSL kernel. This
does not qualify full libc generation, general semantic alias rules, multithreaded
TLS, unwinding, Rust integration, other releases, or old-distribution/SLES execution.
See [stub generation](STUB-GENERATION.md) for supported cases and reproduction.
Expanded reports and hashes are in the ignored validation directory's logs.

## Full pinned library generation and direct Rust build

On 2026-10-09, all eight selected x86_64 glibc 2.17 libraries were generated for
both real input trees. Each set has 2915 public numeric GLIBC definitions; an
independent readelf audit checked all 5830 for pinned membership, reference
versions/defaults, kinds, binding, visibility, sizes, section alignment, exact
shared ranges, and overlapping object/TLS view offsets. Private/unversioned
definitions are excluded from this public ABI contract.

Libc's historical sys_errlist views share storage with different declared sizes.
The emitter now allocates bounded union backing and preserves per-version sizes
and offsets. C tests passed both memcpy versions/stdout copy-relocation and
legacy/current sys_errlist copies; reserved null errno slots were compared as
valid contents. Real-library controls also passed. Strong memfd_create references
still failed to link.

Rust 1.98.1 built a static-standard-library, non-PIE executable using generated
glibc DSOs, explicit target CRT/libc_nonshared/libpthread_nonshared objects, host
GCC 13 crtbegin/crtend, and the real compatible CentOS GCC 4.8.5 libgcc_s.
Both input routes passed allocation, spawned-thread/TLS value isolation, TLS
destruction, and caught panic unwinding under their respective old real runtimes.
Their highest required numeric GLIBC version is 2.16; target qualification remains
2.17. Link maps and loader lists verified every shared link/runtime input stays
in the specified generated or real input trees.

The full manifest records source/reference/generated/runtime/executable hashes,
persistent link input hashes (including the installed Rust SDK archives),
compiler version, link arguments, and real-input provenance. Both compiler runtime
copies match the recorded old source library. See [full libraries](FULL-LIBRARIES.md).

Windows tests passed (20 integration tests plus 3 original example tests); Linux
all-target tests passed (23). The final ABI-duplicate/backing-alignment guards passed
targeted regressions on both platforms. Formatting and warning-free all-target
Clippy passed on both. Reports and both generated sets are retained under
`validation-output/glibc-two-paths`.

This is one qualified glibc release/reference pair on the newer WSL kernel. A
release provider contract, Cargo host/target integration, MSRV qualification,
old-distribution/SLES execution, broader APIs/native dependencies, version-parent
graphs/general alias semantics, and redistribution/packaging remain open.

## GNU glibc 2.22 hosted extension

The [2.22 provider](GLIBC-2.22.md) passed source-build, all eight real DSO/readelf
comparisons, all 2922 public ABI definitions, C historical data views, Rust
allocation/thread/TLS-destruction/unwind tests, Cargo host build/proc-macro and
runtime tests, incompatible-release selection, and newer-symbol rejection.
Both fmemopen versions ran under 2.22; both 2.17 routes rejected the 2.22 interface.

[The hosted provider matrix](https://github.com/rustcommons/libc-abi-tools/actions/runs/38058240922)
passed all four 2.17/2.22 and Clang 21/24 combinations with Rust 1.99.0 on
2026-10-10. This qualified producer commit 39101a0bdf9d824e3fb89931552b0a771fee5e0e
and companion commit 695ac536a791650d0d351c74562e008c47a82392.
Windows/Linux checks, the Rust 1.90 all-targets check, and all 28 snapshot jobs
also passed. SLES 12 SP5, old distro/kernel, and SDK redistribution remain separate.

## Historical matrix and shared conformance harness

On 2026-10-10, commit `a7c48acbd27025ccdeb7f67c99fb462072d61900` passed
[reference/host CI](https://github.com/rustcommons/libc-abi-tools/actions/runs/38061222375)
and [all four provider conformance jobs](https://github.com/rustcommons/libc-abi-tools/actions/runs/38061222447).
The same matrix-driven boundaries and measured-default probe ran for glibc
2.17/2.22 under Clang 21/24. Both 2.17 input routes and the 2.22 source route
passed, preserving host build/proc-macro separation and real runtime checks.
Ordinary fmemopen bound to GLIBC_2.2.5 in the measured 2.17 profiles and GLIBC_2.22
in the 2.22 source profile; explicit older compatibility remained available.

Rust 1.99 Windows/Linux checks, the Rust 1.90 all-targets check, all 28 snapshot
jobs, and byte-identical regenerated matrices/Markdown passed. Local suites
contain 29 producer tests including unknown-history, migration, removal,
reintroduction, and kind/size-change regressions. The interface reference covers
4671 tuples and 8752 native observations; historical candidates before 2.17 are
inventoried rather than normalized or runtime-qualified. Exhaustive generated
per-interface link probes and behavioral/header history remain planned.

## Exhaustive linking, library behavior, and GNU 2.18 extension

The subsequent [exhaustive suite](https://github.com/rustcommons/libc-abi-tools/actions/runs/38064818474)
passed all four existing Clang 21/24 provider jobs. The five dedicated library
[behavior suites](https://github.com/rustcommons/libc-abi-tools/actions/runs/38065649690)
then passed all four jobs as well. These supersede the earlier planned status
of exhaustive link probes; complete behavioral/header history remains open.

The new [GNU 2.18 profile](GLIBC-2.18.md) passed a no-patch source build, native
reader/readelf comparison, and exact snapshot reconciliation for all eight DSOs.
Full stub auditing and isolated strong relocation tests passed 2921 accepted
interfaces, 1750 rejected interfaces, and 2711 measured ordinary defaults.
The five real-library behavior cases passed under Clang 18.1.3. Rust 1.99.0
allocation, threads/TLS destruction, unwinding, historical data, compatibility
interfaces, and Cargo host build/proc-macro separation passed with GNU binutils
2.42. The Cargo program's highest direct numeric GLIBC import tag was 2.18;
the matching real 2.18 loader and DSOs actually executed it successfully.

The new pthread default-attribute fixture imports both GLIBC_2.18 APIs, changes
and restores the stack-size default, and checks the round trip under 2.18 and
2.22. Both 2.17 input routes reject the names at the linker. All four real input
profiles were rechecked through the full shared suite and Cargo integration;
memfd_create still fails at the target linker and mismatched release requests
fail before building. Generated stubs were never used as runtime implementations.

Native evidence now contains 11673 public numeric observations across four
profiles. `scripts/import-glibc-elf-profile.py` verifies complete matrix
membership, input/metadata/archive/package/patch hashes, and unresolved ABI
differences before adding a profile. Its duplicate-profile guard was exercised:
reimporting 2.18 failed and both committed evidence files remained unchanged.
Changed metadata, input/package hashes, and incomplete-library fixtures were
also rejected before writing; both evidence files remained unchanged in each case.
Generated references regenerate byte-identically. Both repositories passed
formatting, all-target tests (29 producer / 42 consumer), and clippy with warnings
denied. Distribution/kernel/native-dependency and SDK qualification remain open.

Producer commit `75945c278187899c537af636fdc311a07e27132f` pins consumer commit
`edfca5b9f9fb811069f302e8d2d5ef2f99f7fd33`. The expanded six-job
[hosted provider matrix](https://github.com/rustcommons/libc-abi-tools/actions/runs/38066726167)
passed all six Clang 21/24 and 2.17/2.18/2.22 jobs. The producer's
[host/reference/snapshot CI](https://github.com/rustcommons/libc-abi-tools/actions/runs/38066726169)
and [companion CI](https://github.com/rustcommons/cargo-libc/actions/runs/38066613985)
also passed, including Windows/Linux Rust 1.99, Rust 1.90 minimum checks,
all 28 snapshots, regenerated tables, and conventional companion sysroot smoke tests.

## Shared preparation and restored application qualification

Producer commit `8939827c12bf0354b81c8c2f2707d0a2b7985b1b` passed all nine
[split-provider jobs](https://github.com/rustcommons/libc-abi-tools/actions/runs/38069527581):
three once-per-release preparation jobs and six Rust 1.99/Clang 21 or 24
application jobs. Preparation preserved all previous native/export/storage and
exhaustive accepted/rejected/default tests. Applications downloaded the same
prepared bundle for each version, verified inventory/commit/target/pin hashes,
preserved aliases/permissions, regenerated local descriptors and GCC startup
selection, and passed Rust/C/Cargo linking and matching real-runtime execution.
Both 2.17 input routes remain covered.

Local restored-input checks also passed all four profiles with Clang 18.1.3.
An initial minimal bundle failed the pseudoterminal case because NSS runtime
modules were omitted. Matching NSS/libnsl inputs are now transferred, and
actual runtime initialization traces and module hashes are audited; known
executables/plugins are allowed explicitly, while host module leakage fails.
This extends observed runtime dependency coverage without claiming a complete
NSS/libnsl public ABI matrix or arbitrary application dependency resolver.

[Main CI](https://github.com/rustcommons/libc-abi-tools/actions/runs/38069527607)
passed Rust formatting/tests/clippy, the minimum-version check, all snapshots,
reference regeneration, and eight original Python archive fixtures. Fixtures
cover byte/permission/relative-alias preservation, archive and member hash
changes, wrong release/pin, traversal/unlisted members, outside-root/cyclic
symlinks, duplicates, and privileged modes. Rejected inputs fail before creating
the extraction directory. The unchanged consumer pin is
`edfca5b9f9fb811069f302e8d2d5ef2f99f7fd33`.

These transfer artifacts are trusted workflow inputs, not authenticated public
SDK releases. Version-suffixed target selection is documented as planned,
with the current explicit provider/glibc interface retained. See
[CI and release artifacts](CI-AND-RELEASE-ARTIFACTS.md).

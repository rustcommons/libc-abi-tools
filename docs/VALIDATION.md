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

[ABI/host CI](https://github.com/rustcommons/linux-abi-tools/actions/runs/38055872899),
[provider/runtime CI](https://github.com/rustcommons/linux-abi-tools/actions/runs/38055872917),
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
glibc checkout has not been independently exercised by this collector. The ABI
data comes from glibc's release tags through glibc-abilists, whose collector
verified every file against glibc's own repository (2026-10-10); it is not a new
glibc build.

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

[The hosted provider matrix](https://github.com/rustcommons/linux-abi-tools/actions/runs/38058240922)
passed all four 2.17/2.22 and Clang 21/24 combinations with Rust 1.99.0 on
2026-10-10. This qualified producer commit 39101a0bdf9d824e3fb89931552b0a771fee5e0e
and companion commit 695ac536a791650d0d351c74562e008c47a82392.
Windows/Linux checks, the Rust 1.90 all-targets check, and all 28 snapshot jobs
also passed. SLES 12 SP5, old distro/kernel, and SDK redistribution remain separate.

## Historical matrix and shared conformance harness

On 2026-10-10, commit `a7c48acbd27025ccdeb7f67c99fb462072d61900` passed
[reference/host CI](https://github.com/rustcommons/linux-abi-tools/actions/runs/38061222375)
and [all four provider conformance jobs](https://github.com/rustcommons/linux-abi-tools/actions/runs/38061222447).
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

The subsequent [exhaustive suite](https://github.com/rustcommons/linux-abi-tools/actions/runs/38064818474)
passed all four existing Clang 21/24 provider jobs. The five dedicated library
[behavior suites](https://github.com/rustcommons/linux-abi-tools/actions/runs/38065649690)
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
[hosted provider matrix](https://github.com/rustcommons/linux-abi-tools/actions/runs/38066726167)
passed all six Clang 21/24 and 2.17/2.18/2.22 jobs. The producer's
[host/reference/snapshot CI](https://github.com/rustcommons/linux-abi-tools/actions/runs/38066726169)
and [companion CI](https://github.com/rustcommons/cargo-libc/actions/runs/38066613985)
also passed, including Windows/Linux Rust 1.99, Rust 1.90 minimum checks,
all 28 snapshots, regenerated tables, and conventional companion sysroot smoke tests.

## Shared preparation and restored application qualification

Producer commit `8939827c12bf0354b81c8c2f2707d0a2b7985b1b` passed all nine
[split-provider jobs](https://github.com/rustcommons/linux-abi-tools/actions/runs/38069527581):
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

[Main CI](https://github.com/rustcommons/linux-abi-tools/actions/runs/38069527607)
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

## GNU glibc 2.19 provider (local, 2026-10-11)

The pinned GNU 2.19 archive (SHA-256 `2d3997f5…a2a2`, detached signature verified
with the GNU keyring) built unchanged with the 2.18 CentOS 7 builder, GCC 4.8.5,
and the same configuration. All eight DSOs matched GNU readelf and the pinned
snapshot with zero differences: 2921 public definitions, identical in count to
2.18, consistent with the snapshot diff recording no x86_64 changes between them.
Preparation passed the export/storage audit and the exhaustive suite (2921
accepted, 1750 rejected, 2711 ordinary defaults). The applications passed
allocation, thread/TLS, unwinding, data views, the 2.18 pthread default-attribute
fixture (accepted), the fmemopen 2.22 rejection, and five behavior suites.
cargo-libc `2a6d0e57f915c774d71b1261b6707014d2bde47e` passed the provider build,
tier 1 scan, tier 2 local run under a Linux 3.10 filter, workflow fixtures,
`check`, mismatched-release and conflicting `--glibc` rejection, and the
too-new memfd_create link failure. The profile was imported as
`gnu-source-2.19`; the reference regenerated and verified. Both repositories
passed formatting, tests, and clippy with warnings denied. Tools: Rust 1.99.0,
Clang 18.1.3, GNU binutils 2.42, podman 5.8.2 on WSL Ubuntu 24.04.

Producer `fb5d99f977a34d8f75c455b947a22f83ef57ea35` pins that consumer. The
[provider workflow](https://github.com/rustcommons/linux-abi-tools/actions/runs/38142660054)
passed all twelve jobs: four preparations (2.17, 2.18, 2.19, 2.22) and eight
Rust 1.99/Clang 21 or 24 applications. The 2.19 job logs show the same 2921/1750/2711
exhaustive results, the tier 1 scan pass, the tier 2 run, `check`, and the
rejection tests. Main CI also passed at that commit.

## GNU glibc 2.20 provider (local, 2026-10-11)

The pinned GNU 2.20 archive (SHA-256 `f84b6d42…6ba5`, detached signature verified
with the GNU keyring) built unchanged with the 2.18 CentOS 7 builder, GCC 4.8.5,
and the same configuration. All eight DSOs matched GNU readelf and the pinned
snapshot with zero differences: 2921 public definitions, as in 2.19.
Preparation passed the export/storage audit and the exhaustive suite (2921
accepted, 1750 rejected, 2709 ordinary defaults). Two fewer defaults than 2.19
come from libpthread's `vfork` and `__vfork`, which 2.20 keeps only as
compatibility IFUNC symbols redirecting to libc's; `memset`/`__memset_chk` became
IFUNC and three `pthread_rwlock_*` symbols became WEAK (see
[glibc 2.20](GLIBC-2.20.md)). The applications passed allocation, thread/TLS,
unwinding, data views, the 2.18 pthread default-attribute fixture (accepted),
the fmemopen 2.22 rejection, and five behavior suites.
cargo-libc `8444108c1b83e72ef484b95f756308de34725844` passed the provider build,
tier 1 scan, tier 2 local run under a Linux 3.10 filter, workflow fixtures,
`check`, mismatched-release and conflicting `--glibc` rejection, and the
too-new memfd_create link failure. The profile was imported as
`gnu-source-2.20`; the reference regenerated and verified. Both repositories
passed formatting, tests, and clippy with warnings denied. Tools: Rust 1.99.0,
Clang 18.1.3, GNU binutils 2.42, podman 5.8.2 on WSL Ubuntu 24.04.

Producer `5ed1e12856f98566dfb2c61d0c64ce7922306b27` pins that consumer. The
[provider workflow](https://github.com/rustcommons/linux-abi-tools/actions/runs/38144128128)
passed all fifteen jobs: five preparations (2.17, 2.18, 2.19, 2.20, 2.22) and ten
Rust 1.99/Clang 21 or 24 applications. The 2.20 job logs show the same 2921/1750/2709
exhaustive results, the tier 1 scan pass, the tier 2 run, `check`, and the
rejection tests. Main CI also passed at that commit.

## GNU glibc 2.21 provider (local, 2026-10-11)

The pinned GNU 2.21 archive (SHA-256 `aeeb3624…23b8`, detached signature verified
with the GNU keyring) built unchanged with the 2.18 CentOS 7 builder, GCC 4.8.5,
and the same configuration. All eight DSOs matched GNU readelf and the pinned
snapshot with zero differences: 2921 public definitions, as in 2.18–2.20.
Preparation passed the export/storage audit and the exhaustive suite (2921
accepted, 1750 rejected, 2708 ordinary defaults). The one fewer default than 2.20
is libc's `sigvec`, which 2.21 keeps only as a compatibility symbol (2.21 NEWS).
The new `sigvec-default-2.21.c` fixture follows each provider's measured default:
it was rejected at link time for 2.21 and linked, bound to `sigvec@GLIBC_2.2.5`,
and delivered a signal under the real 2.19 and 2.20 runtimes. The applications also
passed allocation, thread/TLS, unwinding, data views, the 2.18 pthread
default-attribute fixture (accepted), the fmemopen 2.22 rejection, and five
behavior suites. cargo-libc `16a464ea88aa3b54d652583303fe6ed6c4d26e58` passed the provider build,
tier 1 scan, tier 2 local run under a Linux 3.10 filter, workflow fixtures,
`check`, mismatched-release and conflicting `--glibc` rejection, and the
too-new memfd_create link failure. The profile was imported as
`gnu-source-2.21`; the reference regenerated and verified. Both repositories
passed formatting, tests, and clippy with warnings denied. Tools: Rust 1.99.0,
Clang 18.1.3, GNU binutils 2.42, podman 5.8.2 on WSL Ubuntu 24.04.

Producer `85b69c8ad27be4ae486e6723ca0698a06ba96c92` pins that consumer. The
[provider workflow](https://github.com/rustcommons/linux-abi-tools/actions/runs/38147732823)
passed all eighteen jobs: six preparations (2.17 through 2.22) and twelve
Rust 1.99/Clang 21 or 24 applications. The 2.21 logs show the same 2921/1750/2708
exhaustive results, the tier 1 scan pass, the tier 2 run, `check`, and the
rejection tests. The retained Clang 24 diagnostics show the sigvec fixture
linking `sigvec@GLIBC_2.2.5` and running on 2.17 (prebuilt and source), 2.18,
2.19 and 2.20, and failing to link on 2.21 and 2.22. Main CI also passed.

## GNU glibc 2.23 through 2.29 providers (local, 2026-10-11)

All seven archives verified against the GNU keyring and built without patches:
2.23–2.25 with the CentOS 7 builder, and 2.26–2.29 with a new digest-pinned Debian 9
builder (GCC 6.3, binutils 2.28), since 2.26 needs GCC 4.9. Every DSO matched GNU
readelf and the pinned snapshot with zero differences. Per release, the exhaustive
suite passed (accepted / rejected / ordinary defaults / compatibility-only):
2.23 2931/1740/2699/181, 2.24 2938/1733/2693/193, 2.25 2994/1677/2733/209,
2.26 3143/1528/2878/213, 2.27 3594/1077/3312/223, 2.28 3674/997/3382/233,
2.29 3682/989/3384/234. The applications, five behavior suites, both boundary
fixtures, and cargo-libc `a259c167741c6b1d31eaa3b7c664df8faff3b7a7` (build, tier 1 scan, tier 2 local run,
`check`, rejection tests) passed for each.

Changes made along the way, each tested on these releases:

- The suite now also rejects new unversioned references to every name that has only
  compatibility versions in a library (for example `sigvec`, `bdflush`, and Sun RPC,
  which source builds leave compatibility-only without `--enable-obsolete-rpc`).
- Too-new link tests use a matrix-selected symbol (`too-new-symbol.py`), because
  `memfd_create` itself exists from 2.27.
- `libpthread_nonshared.a` is required and linked only before 2.28.
- Experiment scripts accept any release with pinned `examples/glibc-X/INPUTS.toml`,
  and `fetch-glibc.sh` falls back to the kernel.org GNU mirror.

Tools: Rust 1.99.0, Clang 18.1.3, GNU binutils 2.42, podman 5.8.2 on WSL Ubuntu 24.04.

Producer `27caf3dbf9b7b5b018763629d3a3d2ebba377123` with consumer `a259c16` passed all 39
[hosted provider jobs](https://github.com/rustcommons/linux-abi-tools/actions/runs/38155189880)
(13 preparations, 26 Clang 21/24 applications) and main CI on 2026-10-11.

## GNU glibc 2.30 through 2.44 providers (local, 2026-10-11)

All fifteen archives verified against the GNU keyring and built without patches:
2.30–2.36 with the Debian 9 builder, and 2.37–2.44 with a digest-pinned Debian 12 builder
(GCC 12.2, binutils 2.40, snapshot.debian.org 2026-09-01), because binutils 2.28 cannot
assemble 2.37's AVX-512 string functions and 2.42 needs GCC 12.1. Every DSO matched GNU
readelf and the pinned snapshot with zero differences, and every release passed the
exhaustive suite, the applications, five behavior suites, both boundary fixtures, and
cargo-libc `bfe04693f6685770b47c9346e32de556a8029f4e` (build, tier 1 scan, tier 2 local run, `check`, rejection
tests). Per-release counts are in [PROVIDERS.md](PROVIDERS.md).

The 2.34 merger needed no contract change: libpthread, libdl, librt and libutil remain as
placeholder DSOs. Two fixtures became release-aware: the pthread default-attribute
boundary expects the measured default version (`GLIBC_2.34` in libc from 2.34), and the
resolver suite accepts 2.34's `dn_expand` spelling. Every 2.34+ executable imports
`__libc_start_main@GLIBC_2.34` from `crt1.o`, as measured. 2.43 and 2.44 have no later
libc interface in the matrix, so their too-new tests use a name absent from every snapshot.

Tools: Rust 1.99.0, Clang 18.1.3, GNU binutils 2.42, podman 5.8.2 on WSL Ubuntu 24.04.

Producer `ae68bf63f5ddde7fc7852e76eab83ad2c1beb2c0` with consumer `bfe04693f6685770b47c9346e32de556a8029f4e` passed all 84
[hosted provider jobs](https://github.com/rustcommons/linux-abi-tools/actions/runs/38158161339) on 2026-10-11: 28 preparations
(2.17 through 2.44, both 2.17 routes) and 56 Rust 1.99/Clang 21 or 24 applications, plus
main CI. The hosted exhaustive counts equal the local ones for every release; the 2.34
and 2.44 job logs show the scan, the tier 2 run, `check`, the rejection tests and the
matrix-selected too-new failures (`__memcmpeq`, and the absent-name probe).

## Host glibc check for test/run (2026-10-11)

cargo-libc `b809806500415b2053e26b4dedc55847fe5fb054` makes `libc-runner` compare a binary's
non-weak `GLIBC_*` requirements with the version definitions of the libraries the
loader would use whenever the binary runs on the host userland (no runner, or the
local runner), and refuse it with exit code 2 when a version is missing. Unit tests
cover the comparison, loader-order search paths and the new DT_VERDEF parser; the
Linux smoke test shows the passing path. Producer `1dabe9e761d41cfb8742b3a121ab3909c8efb0c2`'s
[84-job provider run](https://github.com/rustcommons/linux-abi-tools/actions/runs/38166901453) adds an end-to-end check for every provider newer
than the CI host's glibc 2.39: a program using `sched_getattr@GLIBC_2.41` built with
the 2.41–2.44 providers was refused ("the host's libraries lack libc.so.6
GLIBC_2.41"), while every provider's normal local run still passed. Releases whose
libc defines no version beyond the host skip that check.

## libgcc_s stub for every provider (2026-10-12)

Every provider now links against a generated `libgcc_s.so.1` stub with GCC 4.8.5's
interface instead of its builder's real library ([design](PROVIDER-DELIVERY.md#the-libgcc_s-stub)).
The [84-job provider run](https://github.com/rustcommons/linux-abi-tools/actions/runs/38193829047) at `b5c926a` passed for all 28 releases:

- CI regenerated the recorded table from CentOS 7's `libgcc-4.8.5-44.el7` in the
  pinned image and found it unchanged.
- Every release's stub exported exactly the 138 recorded definitions with GCC's
  version chain, and contained only traps. For 2.17–2.25 it also equalled the builder's
  own GCC 4.8.5 library; the Debian 9 (GCC 6.3) and Debian 12 (GCC 12.2) run-time
  libraries provided every recorded definition.
- Every Rust executable, built directly and through cargo-libc (debug and release),
  imported 12 `libgcc_s` definitions, all in GCC 4.8.5's runtime, and ran with the
  real library; loader lists showed the stub was never loaded.
- A link needing `__divmodti4@GCC_7.0.0` failed against the stub and succeeded
  against the host's GCC 13 library.

Local runs of every stage for 2.17, 2.28 and 2.44 gave the same results before the
hosted run.


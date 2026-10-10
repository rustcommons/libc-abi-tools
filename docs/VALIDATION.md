# Validation — bootstrap milestone

## Provider contract and Cargo integration

The local version-1 exporter and companion `--provider` mode passed in WSL with
Rust 1.99.0, Clang 18.1.3, and GNU binutils 2.42 against both glibc 2.17 input
trees. Cargo ran host build scripts/procedural macros and the real 2.17 loader
ran the target thread example. Both executables required at most GLIBC_2.16,
without RPATH/RUNPATH. Independent link-map/runtime checks confirmed generated
DSOs at link time and real target glibc/compatible unwinder at runtime. Mismatched
provider versions failed; SHA-256 executable/input evidence is retained in logs.

Hosted Rust 1.99/Clang 21 and 23 jobs are configured but have not run. Clang 18
local success does not establish Clang 21/23 success. See
[the trusted local contract](PROVIDER-CONTRACT.md) and [per-version roadmap](ROADMAP.md).

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

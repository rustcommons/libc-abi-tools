# Current glibc coverage

Coverage recorded on 2026-10-10. The target is **x86_64 Linux, ELF64 little-endian,
dynamically linked applications**. The experimental providers currently select
glibc **2.17 or 2.22**. Having a snapshot for a version does not establish a provider
or qualify its runtime.

All eight libraries below already have snapshot parsing, historical matrix
entries, native ELF comparison, generated stubs, export audits, and exhaustive
isolated-library link tests for the established provider inputs. We do not need
to add basic support for libc, libm, libpthread, libdl, librt, or libutil.

## Libraries and versioned interfaces

Counts are distinct **library / symbol / version tuples**, not unique function
names. Compatibility versions count separately. Objects are included. The union
column covers the 28 vendored snapshot labels, 2.17 through 2.44.

| Matrix library | Provider SONAME | Union interfaces | Present in 2.17 | Present in 2.22 | Dedicated real-runtime coverage |
| --- | --- | ---: | ---: | ---: | --- |
| `c` | `libc.so.6` | 2847 | 2125 | 2127 | Selected C/Rust allocation, stdio, both memcpy versions, historical data views, and fmemopen version/default tests |
| `m` | `libm.so.6` | 1375 | 395 | 398 | Dedicated math behavior tests pending |
| `pthread` | `libpthread.so.0` | 274 | 233 | 235 | Rust thread creation/join, thread-local storage and destructor checks |
| `dl` | `libdl.so.2` | 12 | 9 | 9 | Dedicated dlopen/dlsym/dlvsym/dlclose tests pending |
| `rt` | `librt.so.1` | 52 | 47 | 47 | Dedicated realtime API behavior tests pending |
| `util` | `libutil.so.1` | 7 | 6 | 6 | Dedicated utility API behavior tests pending |
| `resolv` | `libresolv.so.2` | 91 | 91 | 91 | Dedicated resolver behavior tests pending |
| `ld` | `ld-linux-x86-64.so.2` | 13 | 9 | 9 | Matching real loader executes programs; dependency listings and runtime paths are audited |
| **Total** | **Eight libraries** | **4671** | **2915** | **2922** | **Selected behavior; not every interface** |

Full names, namespace provenance, retained compatibility definitions, and measured
default roles appear in [the interface reference](reference/GLIBC-SYMBOLS.md).
Actual library membership comes from each snapshot. Later releases can move
symbols between libraries; we do not project current membership backward.

## What each test stage establishes

| Stage | Libraries / versions | What is checked | Limit |
| --- | --- | --- | --- |
| Snapshot and history | All eight libraries, all 28 vendored labels | Input checksums, parsing, deterministic export, exact membership, symbol version, function/object/TLS kind, object/TLS size, adjacent changes, generated reference consistency | Labels do not establish upstream release availability, default binding, or runtime support |
| Native ELF evidence | All eight, 2.17 CentOS prebuilt, 2.17 GNU source, 2.22 GNU source | Native reader agrees with GNU readelf; public numeric GLIBC exports match the snapshot; defaults, compatibility, FUNC/IFUNC/OBJECT/TLS, binding, visibility, sizes, alignment and shared ranges are recorded | These are measured input profiles, not every distribution build |
| Generated export audit | All eight, the three input profiles | Exact exported interface set and measured properties; preserved data sizes and overlapping/shared storage; correct SONAME; no DSO dependencies | Generated libraries contain link-only stubs, not implementations |
| Exhaustive relocation linking | All eight, the three input profiles | Each present versioned interface links; every absent union interface individually fails against its assigned isolated DSO; successful imports are inspected; all measured ordinary defaults bind to their measured versions | Strong direct references; does not establish arbitrary dependency-closure or optional weak-import policy |
| TLS probe machinery | Original synthetic fixture | General-dynamic TLS binding succeeds, a missing version fails, DTPMOD64/DTPOFF64 relocations are verified | Current collected union has no TLS rows; this is not exhaustive real exported-TLS coverage |
| Cargo integration | 2.17 and 2.22 providers | Explicit release/descriptor selection, host build script and proc macro separation, target startup/nonshared/runtime inputs, positive program and too-new negative link, artifact imports and link-map paths | Experimental dynamic executable integration; not a published SDK or arbitrary native-dependency qualification |
| Real execution | Three real input profiles | Selected C/Rust programs run under matching real loader/DSOs; allocation, thread/TLS destruction, unwinding, historical data, versioned/default interfaces; runtime dependency paths checked | Does not establish every interface's behavior or old distro/kernel compatibility |

An absent symbol is rejected only inside known matrix scope. Missing historical
or architectural evidence is **unknown**, not evidence that linking should fail.
Default versions come from real ELF evidence, not the highest numeric tag.
No generated stub is executed. Runtime tests always use real implementations.

## Exhaustive local results and hosted status

| Real input profile | Accepted versioned links | Rejected versioned links | Ordinary default bindings |
| --- | ---: | ---: | ---: |
| 2.17 CentOS prebuilt | 2915 | 1756 | 2847 |
| 2.17 GNU source | 2915 | 1756 | 2705 |
| 2.22 GNU source | 2922 | 1749 | 2703 |
| **Total** | **8752** | **5261** | **8255** |

All these local checks passed with GNU binutils 2.42. Different default counts
reflect actual input profiles. The shared provider script now runs the exhaustive
suite and retains individual results and input/tool hashes in `link-matrix/`.

Hosted provider CI uses Rust 1.99.0 and Clang 21/24 for application builds and
tests both provider releases; historical glibc source builds use their pinned
qualified builder. Snapshot/reference and Rust host checks at commit `c3faf20`
[passed](https://github.com/rustcommons/libc-abi-tools/actions/runs/38064818450).
The [expanded provider run](https://github.com/rustcommons/libc-abi-tools/actions/runs/38064818474)
also passed all four 2.17/2.22 Clang 21/24 jobs, including the exhaustive suite.
The earlier
[provider run](https://github.com/rustcommons/libc-abi-tools/actions/runs/38061222447)
passed all four jobs, before exhaustive per-interface relocation tests were added.

## Remaining coverage work

- Add focused real-runtime suites for libm, libdl, librt, libutil, and libresolv.
  Use correct declarations and deterministic fixtures: selected math cases,
  loading a controlled DSO, supported realtime operations, pseudoterminal
  operations, and resolver packet parsing without external DNS dependence.
  Each case needs explicit expected behavior and resource cleanup.
- Qualify native C dependencies and broader whole-program dependency resolution
  separately from isolated-library membership. A symbol absent from one DSO
  may legitimately resolve through another library in a complete application.
- Establish each additional provider, including the 2.34 merged-library layout,
  using [the provider rollout plan](GLIBC-PROVIDER-PLAN.md).
- Qualify actual intended distributions and kernels, including SLES 12 SP5;
  modern-host execution of source-built runtimes does not establish that result.
- Inventory other glibc-produced DSOs, modules, and archives before expanding
  scope. NSS modules, additional glibc libraries such as libanl, distribution
  compatibility libraries, and external dependencies are not included merely
  because an application can find them at runtime. The eight-library matrix
  cannot determine their acceptance or rejection.
- Extend architectural and pre-2.17 historical evidence with explicit adapters
  and completeness evidence. Other architectures, static linking, private
  `GLIBC_PRIVATE` interfaces, and unversioned interfaces are not covered by this
  public numeric GLIBC acceptance matrix.
- Maintain header/feature and behavioral change ledgers with source references;
  an unchanged exported interface does not prove unchanged implementation.

For implementation details see [history and conformance](GLIBC-HISTORY.md),
[provider contract](PROVIDER-CONTRACT.md), and [the roadmap](ROADMAP.md).

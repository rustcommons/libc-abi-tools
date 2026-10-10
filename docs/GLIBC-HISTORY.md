# glibc history and the standard conformance suite

Use one test collection for every supported target, with expectations supplied
by an evidence-backed ABI matrix. A test must not assume that every release
exports the newest function, or that an older version tag guarantees availability.

Start with the [interface reference](reference/GLIBC-SYMBOLS.md),
[adjacent snapshot differences](reference/GLIBC-DIFFS.md), and
[measured native differences](reference/GLIBC-NATIVE-DIFFS.md).
These tables are generated, searchable, and linked to source/profile evidence.

## What an interface means

The identity is **target + input profile + library/SONAME + symbol + ABI version**.
For example: `x86_64-unknown-linux-gnu`, GNU source-built 2.22, `libc.so.6`,
`fmemopen@GLIBC_2.22`. Use `GLIBC`, not `GLIB`.

Three dates/labels must stay distinct:

| Field | Meaning |
| --- | --- |
| ABI namespace, e.g. GLIBC_2.22 | The versioned binary interface identifier for this target |
| First observed snapshot | Earliest collected release exporting this library/symbol/version tuple |
| Historical API introduction | First documented source/header/runtime implementation; unknown until historical evidence establishes it |

For example, a `GLIBC_2.2.5` interface first observed in our 2.17 snapshot was
not introduced in 2.17. An existing interface can also move libraries: in the
2.34 snapshots, `pthread_create@GLIBC_2.2.5` appears in libc rather than libpthread.
An old numeric tag alone cannot determine library membership.

The namespace is architecture-specific. Upstream identifies GLIBC_2.2.5 as the
[earliest x86_64 symbol set](https://sourceware.org/legacy-ml/libc-alpha/2012-05/msg01022.html).
It is not the beginning of glibc's history. Early glibc targets, unversioned
interfaces, object formats, threading implementations, and additional libraries
require their own adapters. They must not be fabricated as x86_64 GLIBC tags.

## New defaults, compatibility, and removal

GNU symbol versions use `name@@GLIBC_x.y` for a default export and
`name@GLIBC_x.y` for a compatibility export. The tables label the interface with
one `@`, then show default/compatibility explicitly for each measured profile.

| Versioned interface | GNU 2.17 source build | GNU 2.22 source build |
| --- | --- | --- |
| fmemopen@GLIBC_2.2.5 | Present; default | Present; compatibility interface |
| fmemopen@GLIBC_2.22 | Absent | Present; new default |

The new default changes which interface an ordinary unversioned reference
selects. It does not erase the older explicitly versioned interface. Record these
as **default promotion**, **compatibility retention**, and, when evidenced,
**removal**. Avoid the ambiguous term "override" without identifying which occurred.

The highest numeric tag is not proof of the default. Availability lists do not
contain this information; measured ELF version flags do. Defaults are unknown for
unmeasured profiles, including 2.19–2.21 between our measured 2.18 and 2.22
ELF samples. Older interfaces may also receive bug fixes without acquiring a new
tag; source/NEWS references and behavioral tests must record those separately.

## Current coverage and provenance

| Evidence | Current coverage | Limits |
| --- | --- | --- |
| Public ABI membership/kind/object or TLS size | 28 pinned x86_64 snapshot labels, 2.17–2.44; eight libraries; 4671 distinct library/symbol/version tuples | Other libraries, architectures, and earlier releases are not collected |
| Native defaults, FUNC/IFUNC, binding, visibility, storage/alignment/ranges | 11673 observations from CentOS prebuilt 2.17 and GNU source-built 2.17/2.18/2.22 | Input-specific observations; no inferred defaults for other releases |
| Provider/link/runtime qualification | Locally and hosted-qualified 2.17/2.18/2.22 experimental providers | Metadata coverage does not qualify every snapshot or distribution as a build/runtime target |
| Early history | Official archive/diff candidates referring back to 1.00; 71 upstream tag candidates before 2.17 | Unverified release classification and target applicability; normalized symbol sets not collected |

[PROVENANCE.json](../data/glibc-history/PROVENANCE.json) records the pinned Zig ABI
dataset, upstream release-source archives, build configuration, source adjustments,
package manifests, and per-library ELF/metadata SHA-256 evidence. Actual data
observations come from the independent readelf-qualified local input trees;
hosted jobs independently rebuild and check their corresponding inputs.

The historical catalogs are an inventory, not a release chronology or an ABI
specification. Development tags and patch artifacts must be classified before use.
Archive hashes currently marked `unknown` must be obtained and verified before
using those artifacts as pinned symbol evidence. The
[official old archive](https://gcc.gnu.org/pub/glibc/old-releases/) includes patches
such as `glibc-1.00-1.01.diff.bz2`; locating a patch does not establish a complete
1.00 source tree or its target ABI.

## Machine-readable contract

| File | Contents |
| --- | --- |
| [symbols.tsv](../data/glibc-history/symbols.tsv) | One tuple per row; first observed snapshot; F / D:bytes / T:bytes / absent in each covered release |
| [changes.tsv](../data/glibc-history/changes.tsv) | Every added, removed, or kind/size-changed tuple between consecutive covered snapshots |
| [elf-evidence.tsv](../data/glibc-history/elf-evidence.tsv) | Profile-labelled defaults, native types/binding/visibility, sizes, alignment/range observations, and input hashes |
| [native-differences.tsv](../data/glibc-history/native-differences.tsv) | Measured membership/default/native-type/binding/visibility/storage/alignment changes between GNU 2.17 and 2.22 source builds |
| [historical-tags.tsv](../data/glibc-history/historical-tags.tsv) | Upstream tag objects and peeled commit candidates; classification/applicability still unknown |
| [historical-archives.tsv](../data/glibc-history/historical-archives.tsv) | Official old archive/patch candidates and collection gaps |

Function code size, address, and section/range-group numbers are artifact
observations, not behavioral or language-level ABI guarantees. Shared ranges
do not establish semantic aliasing. Header signatures, macro/feature changes,
version-node parent graphs, NSS/loader dependencies, and distribution patches
need additional evidence fields and dedicated tests.

`accept` means the requested explicit versioned export is present in a covered
library snapshot; `reject` means it is absent in that known scope; `unknown`
means the release/library is outside collected evidence. Unknown never means
absent. Complete source lists for tracked libraries establish known absence;
incomplete or unavailable historical inputs do not.

This is a direct export-membership decision, not a whole-program promise. A
library migration may allow a link through another library or dependency closure.
Unversioned references need measured defaults; optional weak references can
validly remain unresolved; private namespaces require a separate explicit policy.

## The standard test collection

Apply the same collection to each qualified profile, changing the matrix data
and real inputs rather than maintaining separate hand-written expectations.

| Stage | Standard checks | Current status |
| --- | --- | --- |
| Evidence integrity | Pins, checksums, complete declared scope, canonical versions, duplicate/conflicting records | Existing input checks; reference regeneration added |
| Metadata conformance | Exact membership, retained versions, removals/reintroductions, library moves, object/TLS changes, unknown history | Matrix regression tests and all 28 snapshot jobs |
| Native ELF conformance | Independent readelf comparison, SONAME, versions/defaults, types, binding/visibility, storage/alignment, range evidence | Existing complete checks for qualified providers; native-difference reference added |
| Link conformance | Accept each present explicit interface; reject each absent interface from an isolated library; check default binding separately | Matrix-driven fmemopen/memfd boundaries and a default-binding probe added; exhaustive generated relocations still planned |
| Artifact conformance | Direct version needs, intended provider inputs, compatible CRT/runtime, no host DSO leakage or generated-stub runtime | Existing complete provider checks |
| Runtime/behavior | Defined, type-correct programs testing both old/new semantics, TLS, unwinding, data views, loader behavior | Existing programs plus default fmemopen; per-interface semantic coverage remains incomplete |
| Distribution/kernel | Real target OS, native dependencies, patches, CPU/kernel assumptions | Separate qualification; SLES remains open |

Future exhaustive linker probes should take the union of collected interfaces
and generate strong, type-appropriate function/object/TLS relocations. Probe one
DSO without implicit libc or other libraries when testing direct membership;
separately test whole-link dependency resolution. Never call arbitrary functions
with invented signatures or execute generated stubs. A successful export inventory
check is not a substitute for actually checking relocations/default binding.

For behavioral history, add an evidence ledger keyed by interface/profile with
source commit/NEWS reference, changed contract, applicable range, test ID, and
result. "Same ABI tag" must not be interpreted as "unchanged implementation".

## Commands and reproduction

```bash
cargo run --locked --offline -- glibc matrix --data data/glibc --symbol fmemopen
cargo run --locked --offline -- glibc diff --data data/glibc --from 2.17 --to 2.22
cargo run --locked --offline -- glibc changes --data data/glibc
cargo run --locked --offline -- glibc expect --data data/glibc \
  --release 2.17 --library c --symbol fmemopen --version 2.22
# reject
cargo run --locked --offline -- glibc expect --data data/glibc \
  --release 2.16 --library c --symbol fmemopen --version 2.2.5
# unknown (not fabricated historical absence)
cargo build --locked --offline
python3 scripts/build-glibc-reference.py --binary target/debug/libc-abi --check
```

On Windows, use `python` and `target/debug/libc-abi.exe`. Omit `--check` to regenerate
the machine matrices and Markdown from the pinned ABI lists and collected native
evidence. CI checks byte-identical regeneration and publishes the reference.
Provider tests export a fresh matrix and use it to choose version-boundary
accept/reject cases; default probes use their actual reference ELF evidence.

`examples/glibc-two-paths/check-link-matrix.py` checks every union interface
individually using GNU `ld -shared -z defs` and one explicit stub DSO, with no
implicit C runtime or other libraries. Functions and objects use address
relocations; TLS uses general-dynamic relocations. Every absent interface must
fail with an undefined-reference diagnostic naming that exact interface. Every
accepted import and measured ordinary default is verified with GNU readelf.
The current union contains no TLS interfaces, so an original synthetic fixture
checks successful TLS binding, missing-version rejection, and both TLS relocation
types. Nothing generated by this suite is executed. JSON results and input/tool
provenance are retained in `link-matrix/` by provider CI.

See [the provider rollout plan](GLIBC-PROVIDER-PLAN.md) for the remaining releases.

Local exhaustive validation on 2026-10-10 used the existing readelf-qualified
WSL input trees and GNU binutils 2.42:

| Input route | Accepted versioned interfaces | Rejected versioned interfaces | Measured ordinary defaults |
| --- | ---: | ---: | ---: |
| 2.17 CentOS prebuilt | 2915 | 1756 | 2847 |
| 2.17 GNU source | 2915 | 1756 | 2705 |
| 2.22 GNU source | 2922 | 1749 | 2703 |

The different default counts reflect measured input profiles; they are not
inferred from namespace ordering. The synthetic positive/negative TLS fixture
also passed. [Hosted qualification](https://github.com/rustcommons/linux-abi-tools/actions/runs/38064818474)
of this expanded suite passed all four Clang 21/24 provider jobs at `c3faf20`.

The immediate next work is to classify/pin early sources, establish each target's
historical applicability, normalize historical interface sets, and extend the
same conformance collection. Historical research does not change the currently
supported provider versions or promise that modern Rust runs on glibc 1.x.

The first hosted [shared conformance matrix](https://github.com/rustcommons/linux-abi-tools/actions/runs/38061222447) and [reference regeneration gate](https://github.com/rustcommons/linux-abi-tools/actions/runs/38061222375) passed on 2026-10-10 at commit a7c48acbd27025ccdeb7f67c99fb462072d61900. This qualifies the collected-snapshot/reference and existing-provider stages; early-history normalization and exhaustive relocation/behavior probes remain open.

# CI stages, versioned targets, and release artifacts

## Rust and Clang in application integration

Each application job **builds Rust with rustc 1.99.0**. Its selected Clang is the
link driver used by `cargo-libc` and the compiler for the C fixtures. Clang does
not compile Rust or replace rustc's LLVM backend. We expect matching ABI and
runtime outcomes from both Clang versions; driver/header/compiler regressions
can still produce different results. Both currently use GNU ld.bfd, so this is
not a GNU ld versus LLD comparison.

| Stage | Matrix | Responsibilities |
| --- | --- | --- |
| Main host CI | Rust 1.99 Windows/Linux; Rust 1.90 minimum; 28 snapshot labels; reference/integrity gate | Formatting, Rust unit tests/clippy, snapshot checksums/determinism, generated reference, original CI-bundle extraction fixtures |
| Provider preparation | glibc 2.17, 2.18, 2.19, 2.22: four jobs | Build pinned real inputs, collect native ELF/readelf evidence, generate eight stub DSOs, audit exports/storage/defaults, run exhaustive accepted/rejected/default relocation probes, package verified inputs |
| Application integration | Each provider under Clang 21 and 24: eight jobs | Restore prepared inputs, compile/run Rust allocation/thread/TLS/unwind and C boundary/data programs, run five library behavior suites, regenerate local descriptors, build/run Cargo host-unit and target examples, audit versions/paths and newer-symbol rejection |

Applications wait for the preparation stage to succeed. The 2.17 bundle preserves
both prebuilt and GNU source routes; 2.18/2.19/2.22 preserve the GNU source route.
Historical glibc itself retains the pinned qualified historical builder, while
modern tools build applications. Companion unit tests are covered by the pinned
consumer's own CI, rather than repeated in every application job.

`prepare-and-check.sh` owns stub generation and ABI probes.
`application-and-check.sh` owns application/runtime tests; `provider-and-check.sh`
owns Cargo integration. `full-and-check.sh` remains a local convenience wrapper
for preparation plus application tests, with Cargo integration invoked separately.
The audit emits separate `abi-manifest.json` and `application-manifest.json`.

Hosted qualification passed on 2026-10-10 at producer commit
`8939827c12bf0354b81c8c2f2707d0a2b7985b1b`, consuming pinned cargo-libc commit
`edfca5b9f9fb811069f302e8d2d5ef2f99f7fd33`: all [three preparation and six application jobs](https://github.com/rustcommons/linux-abi-tools/actions/runs/38069527581)
passed. [Main CI](https://github.com/rustcommons/linux-abi-tools/actions/runs/38069527607)
also passed Rust host/MSRV checks, all 28 snapshots, reference regeneration,
and all eight original integrity/extraction fixture tests.

## CI transfer artifacts

The preparation job uploads a `.tar.gz` bundle and SHA-256 sidecar. Tar preserves
relative aliases and executable permissions, which a direct upload of the input
directories would not reliably preserve. `ci-bundle.py` checks the archive and
each member's inventory, hashes, target, selected release, ABI source pin, and
producer commit before extraction. Invalid/duplicate paths, outside-root links,
link cycles, privileged modes, and unexpected members fail before creating the
destination. Existing archives and extraction directories are refused.

The bundle contains:

- Matching target headers, CRT/nonshared files, all eight real runtime DSOs and
  their versioned targets, eight generated stub libraries and evidence.
- Matching NSS modules and libnsl runtime inputs. `openpty` can request group
  lookup through NSS; transferring only the primary eight DSOs caused a real
  historical-loader failure. These modules are runtime inputs, not newly covered
  libraries in the public ABI matrix. Execution traces now audit initialized
  modules, including NSS and the controlled dynamic-loading fixture, against
  the selected real tree and the exact known application/fixture paths.
- The compatible real compiler unwinder, native metadata and preparation
  manifests, original glibc archive, package/compiler/patch records, and project
  attribution/separate ABI data license. The archive includes glibc's source
  and license files; this transport format does not establish a complete SDK
  license/redistribution inventory.

It excludes runner Cargo build directories, application outputs, absolute
provider descriptors, and host GCC startup directories. Each application runner
uses its explicitly selected GCC startup objects and generates a new six-field
v1 descriptor after extraction. Runtime tests load real libraries, never stubs.
Preparation and compiler-specific application diagnostics are retained separately.

These are **workflow transfer artifacts**, scoped to a trusted GitHub Actions
run. A checksum is an integrity check, not publisher authentication. They are
not published user release SDKs, and this job does not install or replace host
glibc. Broader arbitrary dependency-closure qualification remains separate.

## Qualified versioned target syntax

The coordinator supports:

```text
cargo libc build --target x86_64-unknown-linux-gnu.2.17 --provider FILE --artifact EXECUTABLE -- --release
```

The coordinator parses two independent selections:

| User selection | Forwarded / resolved selection |
| --- | --- |
| `x86_64-unknown-linux-gnu` | Cargo/rustc's ordinary Rust target triple and standard-library target |
| `.2.17` | Exact qualified glibc build provider |

The dotted value is a wrapper convention, not a new built-in Rust target.
Users would install the normal Rust target with rustup. The wrapper must reject
unsupported releases, unknown architectures, and conflicting suffix/`--glibc`
selections before building. Its provider cache must separate releases and
artifact identities. A selected compatibility baseline does not guarantee that
every executable imports a symbol with that exact highest version, or qualify
its entire dependency closure, CPU, kernel, or distribution.

Today the equivalent selection is explicit `--glibc 2.17 --provider FILE`, with
the application's Cargo options after `--`; see [the current contract](PROVIDER-CONTRACT.md).
Target-suffix parsing is implemented. Provider discovery/download, authenticated
cache installation, and user release SDKs remain future work.

## How cargo-zigbuild and Zig handle this

[cargo-zigbuild](https://github.com/rust-cross/cargo-zigbuild#specify-glibc-version)
documents version-suffixed targets such as `aarch64-unknown-linux-gnu.2.17`,
installs/uses the normal Rust target, and invokes Zig as its native toolchain.
The glibc machinery resides in Zig; it is not simply a cargo-zigbuild collection
of full distribution sysroots for each release.

[Zig's glibc implementation](https://github.com/ziglang/zig/blob/master/src/libs/glibc.zig)
loads the bundled `libc/glibc/abilists` metadata, selects target architecture and
glibc version, generates link-only shared libraries, and caches them by inputs
including compiler version, architecture/ABI, requested glibc version, and the
ABI data file. It also uses bundled glibc headers and source fragments to build
startup/nonshared support with version-specific choices. The real runtime on
the destination supplies the actual shared-library implementations.

Thus the "database" is an essential part, alongside headers, startup/nonshared
sources, generators, linker/compiler integration, and cache management. Our
symbol matrix is the metadata counterpart. Our qualified providers currently
obtain real headers/CRT/nonshared support from pinned historical input trees.
This is a narrower implementation; broad Zig parity is not established.

## What user release artifacts still require

Future releases should distinguish host tool executables, pinned ABI datasets,
and qualified build-provider packages for each supported target/glibc pair.
Test runtimes can remain separate from user build inputs; generated stubs must
never be deployed as the application's glibc implementation. Exact packaging
layout and a relocatable/authenticated contract still need implementation.

Before publishing those packages, establish immutable identity, complete
input/source/license inventory, supported host tools and target scope, install
and cache rules, integrity/publisher verification, compiler-support selection,
and permanent compatibility tests. Then add target-suffix resolution and native
dependency adapters. A CI tar bundle is useful validation of transfer and
rematerialization, but does not complete those release requirements.

See [current coverage](GLIBC-COVERAGE.md), [provider rollout](GLIBC-PROVIDER-PLAN.md),
and [the roadmap](ROADMAP.md).

## Versioned-target qualification

Producer `cbf31dd` pinned consumer `ed4c997` and passed all three
preparation and six Rust 1.99/Clang 21 or 24 application jobs for 2.17/2.18/2.22
([run](https://github.com/rustcommons/linux-abi-tools/actions/runs/38076305526)).
Both real 2.17 input routes passed. The suffix selected matching providers;
Cargo reported ordinary-triple artifact paths; host scripts/proc macros, target
threads, direct ELF/path audits and real-loader execution passed. Mismatched
providers, conflicting explicit versions and memfd_create were rejected with
the expected diagnostics. This does not qualify new platforms or user SDK delivery.

# Provider delivery: three ways to obtain a provider

Status: **design, not implemented.** Today a provider exists only where the
repository's example scripts built it (CI or a developer machine), and cargo-libc
needs `--provider FILE` pointing at its descriptor. This document defines how users
will obtain the provider for any release, 2.17 through 2.44, in three ways. All
three produce the **same provider layout** and must pass the **same qualification**.
The [roadmap](ROADMAP.md#provider-delivery) tracks the work.

## What a provider contains

| Part | Origin | Size (all releases, deduplicated, xz) |
| --- | --- | --- |
| Link stubs for the eight libraries | Generated from the ABI data by `libc-abi-tools` | ≈0.9 MiB, together with the next row |
| `crt1.o`, `crti.o`, `crtn.o`, `libc_nonshared.a` (and `libpthread_nonshared.a` before 2.28) | Built from that release's glibc source | (see above) |
| Installed headers | That release's glibc, plus Linux UAPI headers | ≈1.6 MiB |
| Compiler runtime (`libgcc_s`) and GCC `crtbegin.o`/`crtend.o` | Today: the builder's real `libgcc_s` and the host GCC | see [open questions](#open-questions) |

Measured on 2026-10-11 across 25 locally built releases: 30,480 files and 164 MiB
uncompressed, but only 7,576 distinct files (60 MiB). Stored once each, they compress
to 2.6 MiB, because consecutive releases share almost every header and many objects.
**One data crate can therefore hold every release**; there is no need for 28 crates.

## The three modes

### Mode 2: `source` (the default)

Like Zig: ship glibc **source**, not binaries, and compile the startup and
nonshared objects on the user's machine with Clang the first time a release is used.

- **Shipped:**
  - each release's installed headers;
  - the original glibc source files for `crt1.o`, `crti.o`, `crtn.o` and every
    `libc_nonshared.a` member (the list changes by release; for example the `stat`
    wrappers leave in 2.33 and `libpthread_nonshared.a` merges in 2.28);
  - the generated configuration headers those files need;
  - the exact compile commands, captured from our pinned container builds.
- **Generated locally:** the link stubs, from the ABI data, as today.
- **Why it is the default:** no container engine, no binaries from us, and no
  network once the crates are fetched. Shipping source satisfies glibc's
  source obligation directly.
- **Risk:** glibc officially builds with GCC. These few files must be shown to
  compile correctly with Clang for every release (Zig does this, but we must prove
  it ourselves). The fallback for a release or file where that fails is a captured
  GCC command for users who have GCC.
- **Qualification:** for every release, compare the locally compiled objects with
  the container-built ones: same symbols, versions, sections and relocations. Then
  run the full application, exhaustive and Cargo suites against the result, as for
  container-built providers.

### Mode 1: `prebuilt`

Use exactly the files our CI built and qualified.

- **Shipped:** the startup objects and nonshared archives from the CI container
  builds, byte for byte, with their SHA-256 and the CI run that qualified them. The
  headers are shared with mode 2. The stubs are generated locally as in every mode
  (deterministic), or optionally shipped too.
- **Why:** it is exactly what passed the hosted matrix; it is also the fallback if a
  mode 2 compile fails on an unusual host.
- **Licensing:** this mode redistributes glibc object code, so the LGPL-2.1
  corresponding source must be available to its recipients. That source is the
  pinned GNU archive plus our build scripts and configuration; the archives total
  roughly 400 MB, too large for crates.io. Options to decide:
  - host the archives ourselves next to the binaries (an exception to crates.io-only);
  - ship a written offer of the source, valid for three years (LGPL-2.1 section 6(c));
  - split the source into source-only crates.

  Decide this with legal review before publishing mode 1. See
  [DISTRIBUTION-DIFFERENCES.md](DISTRIBUTION-DIFFERENCES.md#licensing-if-distribution-inputs-are-ever-packaged).

### Mode 3: `container`

Run the same pinned container build that CI runs, on the user's machine.

- **Needs:** podman or docker, network access to the pinned images and package
  archives (CentOS vault, archive.debian.org, snapshot.debian.org), and several
  minutes per release on first use.
- **Why:** the reference for the other two modes, for anyone who wants to build
  everything themselves, and the only mode that also produces the real runtime
  libraries used for runtime testing.
- **Limitation:** it depends on vendor images and archives staying available.
  Replacing them with toolchains built from source is a long-term
  [roadmap](ROADMAP.md#long-term-source-built-builder-toolchains) item.

## Packaging

The current crates, glibc-abilists and the five workspace crates, stay as they are.
New data crates keep licenses separate, as glibc-abilists already does for the ABI lists:

| Crate (names to be decided) | Contents | License |
| --- | --- | --- |
| `glibc-provider-sources` | Deduplicated headers, startup/nonshared sources, generated configuration headers, compile commands, per-release manifests | LGPL-2.1-or-later (glibc), GPL-2.0 WITH Linux-syscall-note (UAPI headers) |
| `glibc-provider-binaries` | Deduplicated CI-built startup objects and nonshared archives, per-release manifests with hashes and CI run | LGPL-2.1-or-later, plus the mode 1 source arrangement |

`libc-abi-tools` gains a `provider` module, following the existing
`bundled-glibc-data` pattern:
- **Feature `provider-sources`** (default) depends on `glibc-provider-sources`.
- **Feature `provider-binaries`** (optional) depends on `glibc-provider-binaries`.
- **Mode 3** needs neither crate.

Because the data arrives as ordinary crate dependencies, `cargo vendor` and offline
builds work unchanged. Each manifest records, per file: path, SHA-256, size, origin
(source or the CI run that produced it), and license.

## Commands

The logic lives in `libc-abi-tools`; cargo-libc depends on it and exposes it. Both
CLIs offer the same operations.

### Using a provider (cargo-libc)

```sh
# Default: no provider flags. A cached provider is used if present; otherwise one
# is prepared in the default mode (source) and cached.
cargo libc build --target x86_64-unknown-linux-gnu.2.28 -- --release

# Choose the mode explicitly (also CARGO_LIBC_PROVIDER_MODE, or config).
cargo libc build --target x86_64-unknown-linux-gnu.2.28 --provider-mode prebuilt -- --release

# Existing explicit forms keep working.
cargo libc build --provider /path/provider.txt --target x86_64-unknown-linux-gnu.2.28
cargo libc build --sysroot DIR --gcc-runtime DIR --glibc 2.28
```

`check`, `test`, `run`, `inspect` and `doctor` accept the same selection.
`--provider FILE` and `--sysroot` take precedence over automatic preparation, and
a mismatched release is still refused.

### Managing providers

```sh
cargo libc provider prepare --glibc 2.28 [--mode source|prebuilt|container]
cargo libc provider list                    # cached providers: release, mode, hashes, origin
cargo libc provider path --glibc 2.28       # descriptor path, for scripts and plain Cargo
cargo libc provider verify --glibc 2.28     # recheck hashes and rerun the stub/ABI audit
cargo libc provider export --glibc 2.28 --to DIR   # copy to an offline machine
cargo libc provider import DIR
cargo libc provider remove --glibc 2.28 [--mode M]
```

The same operations exist as `libc-abi provider ...` for users who do not use
cargo-libc. `prepare` without `--glibc` accepts `--all`.

### Cache

```text
$CARGO_HOME/libc-providers/x86_64-unknown-linux-gnu/glibc-2.28/<mode>-<content-hash>/
    provider.txt  sysroot/  stubs/  compiler-runtime/  manifest.json
```

The cache location can be overridden with `CARGO_LIBC_PROVIDER_DIR`. The content
hash covers every input (manifest hashes, tool versions, compile commands), so
changed inputs never reuse a stale provider. Each entry's `manifest.json` records
its mode and origin.

### Descriptor

The v1 descriptor stays six fields. A provider prepared by any mode records its mode
and content hash in `manifest.json` next to it. If tools need the mode, a v2
descriptor adds an explicit `mode` field, documented before use, and v1 stays
accepted.

## Qualification of all three modes

Hosted CI already qualifies the container builds for every release (84 jobs). For
delivery it additionally:

1. publishes the data crates only from a commit whose container builds passed;
2. for each release, prepares the provider in each mode from the published (or
   about-to-be-published) crates and compares the results:
   - stubs and headers byte-identical in all modes;
   - mode 1 objects byte-identical to the container build;
   - mode 2 objects equivalent in symbols, versions, sections and relocations;
3. runs the application, Cargo, scan, runner and rejection suites against each
   mode's provider;
4. records per release and mode, in [PROVIDERS.md](PROVIDERS.md), which modes
   passed and on which run.

A mode that fails for a release is reported for that release and not offered there,
rather than silently falling back to another mode.

## Open questions

- **Crate names**, and whether the data crates follow the workspace version or
  their own (as glibc-abilists does).
- **Mode 1 source arrangement** (see above).
- **Compiler runtime:**
  - Today, providers use a real `libgcc_s` from the builder and `crtbegin.o`/`crtend.o`
    from the host GCC.
  - Options:
    - generate a link-only `libgcc_s` stub from GCC's symbol versions;
    - ship the CI-built one under its GCC Runtime Library Exception;
    - use Clang's compiler-rt `crtbegin`/`crtend`, which would remove the host-GCC
      requirement.
  - Each needs its own qualification.
- **Headers:** mode 2 needs no network but adds about 1.6 MiB of headers to every
  install. A headers-optional build is possible for pure-Rust users, whose builds
  only need `features.h` for validation.

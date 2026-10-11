# Workflows

How the linux-abi-tools crates fit together, which workflow to use for what, and
how each one maps onto [cargo-libc](https://github.com/rustcommons/cargo-libc).
Every row marks what is **implemented and tested** and what is **planned**; a
planned row is a design, not a claim.

Shipping a Rust program to an older Linux system raises three separate questions:

| Question | When it matters | Answered by |
| --- | --- | --- |
| Which glibc symbol versions may the binary require? | Link time | `libc-abi-tools` (ABI data, stub libraries, providers) and `cargo libc build` |
| Which syscalls and `madvise` advice will the kernel accept? | Run time | `kernel-abi-tools` (syscall data, seccomp filter/profile, runners) |
| Which glibc and kernel does "the target" mean? | Both | `linux-targets` (distribution presets and selection rules) |
| Does a built file already require more than the target has? | After the build | `linux-abi-scan` (static inspection) |

The `linux-abi-tools` crate re-exports all four (`linux_abi_tools::{libc,
kernel, targets, scan}`).

## Workflow catalog

Every workflow, who runs it, and its status. **Implemented** means tested as
described in the linked section; **planned** and **long-term** rows are designs.

| Workflow | Who runs it | Status | Section |
| --- | --- | --- | --- |
| Collect glibc's ABI lists from signed release tags | Maintainers (glibc-abilists) | Implemented | [9.1](#91-glibc-abi-lists) |
| Derive kernel syscall and `madvise` data from Linux release tags | Maintainers | Implemented | [9.2](#92-kernel-data) |
| Build, audit, and qualify a glibc provider for one release | Maintainers and CI | Implemented, 2.17–2.44 | [9.3](#93-provider-pipeline) |
| Record measured evidence and regenerate the reference | Maintainers | Implemented | [9.4](#94-evidence-and-reference) |
| Hosted qualification of every provider | CI | Implemented (84 jobs) | [9.5](#95-hosted-qualification) |
| Release the crates | Maintainers | Implemented (0.1.0, 0.2.0) | [9.6](#96-release) |
| Choose a target (glibc and kernel floors) | Users | Implemented | [1](#1-choose-a-target) |
| Build against an older glibc with a provider | Users | Implemented (provider supplied by the user) | [2](#2-build-against-an-older-glibc) |
| Type-check without linking (`cargo libc check`) | Users | Implemented | [6](#6-how-this-translates-into-cargo-libc) |
| Inspect a built binary (tier 1) | Users | Implemented | [3](#3-inspect-a-built-binary-tier-1) |
| Run or test under an older kernel (tier 2) | Users | Implemented | [4](#4-run-under-an-older-kernel) |
| Check runners and inputs (`cargo libc doctor`) | Users | Implemented | [6](#6-how-this-translates-into-cargo-libc) |
| Use the libraries directly | Developers | Implemented | [5](#5-use-the-libraries-directly) |
| Obtain a provider automatically (`source`, `prebuilt`, `container`) | Users | Planned | [10.1](#101-provider-delivery) |
| Distribution-specific providers | Maintainers and users | Planned, later release | [10.2](#102-distribution-specific-providers) |
| Build the builder toolchains from source | Maintainers and CI | Long-term | [10.3](#103-source-built-builder-toolchains) |
| Qualify on a real target host (tier 3) | Users | Outside these tools; required for any compatibility claim | [Testing tiers](#testing-tiers) |

### Testing tiers

Checks get more reliable, and more expensive, from tier to tier:

| Tier | How | Reliability | Tool |
| --- | --- | --- | --- |
| 1. Inspect | Read the built ELF files; nothing runs | Exact for symbols, versions, ELF notes; heuristic for syscalls in code | `linux-abi-scan`, `cargo libc inspect` |
| 2. Simulate | Run the tests under a seccomp filter (host userland) or in a container (target userland) | Catches what executes; syscall availability and `madvise` only | `kernel-seccomp run`, `cargo libc test`/`run` |
| 3. Qualify | Run on a real host at the target's minimums, e.g. a SLES 12 SP5 VM | The only qualification | Your target environment |

Tier 1 is documented as the least reliable: it reports what a file *could*
do. Tiers 1 and 2 use the same kernel data, so they cannot disagree about
what a kernel provides.

## 1. Choose a target

One selection drives both floors. The rules are implemented in
`linux_targets::Selection::resolve` and tested:

| User gives | glibc floor | Kernel floor | Runtime image |
| --- | --- | --- | --- |
| `--distro sles-12-sp5` | 2.22 | 4.12 (plus sourced backports) | the preset's |
| `--glibc 2.22` or `--target x86_64-unknown-linux-gnu.2.22` | 2.22 | none: host kernel, **not simulated** | none |
| `--kernel 4.12` | host | 4.12 | none |
| `--glibc 2.22 --kernel 4.12` | 2.22 | 4.12 | none |

`--distro` cannot be combined with `--glibc`, `--kernel`, or a versioned
`--target`. `kernel-seccomp distros` lists the 28 presets.

## 2. Build against an older glibc

**Status: implemented, experimental** (glibc 2.17 through 2.44 on x86_64; see
[PROVIDER-CONTRACT.md](PROVIDER-CONTRACT.md)).

1. Produce a provider: generated link-only stub libraries plus the real startup
   objects, `libc_nonshared.a`, headers, and compiler runtime, described by a
   `libc-abi-provider-v1` descriptor. Today this is the
   [provider pipeline](#93-provider-pipeline); planned automatic delivery is in
   [10.1](#101-provider-delivery).
2. Build: `cargo libc build --target x86_64-unknown-linux-gnu.2.22 --provider
   provider.txt --artifact ... -- --release`.

The stubs are used only for linking and never deployed. The provider is still
supplied by the user; [PROVIDER-DELIVERY.md](PROVIDER-DELIVERY.md) designs how
users will obtain one without that step.

## 3. Inspect a built binary (tier 1)

**Status: implemented in `linux-abi-scan`** (tested on synthetic ELF files and
real binaries on Linux 6.18; called by `cargo libc inspect --distro`/`--kernel`).

```sh
linux-abi-scan target/release/app --distro sles-12-sp5 [--strict] [--verbose] [--tsv]
```

| Check | Confidence | Fails when |
| --- | --- | --- |
| `GLIBC_*` version requirements against the glibc floor | Exact | A non-weak requirement is above the floor |
| `DT_RELR` relocations | Exact | The glibc floor is older than 2.36 |
| Imported syscall wrappers (`statx`, `getrandom`, `pidfd_open`, ...) against the kernel floor | Exact | A non-weak import's syscall is missing (weak: warning) |
| `NT_GNU_ABI_TAG` minimum kernel | Exact | Warning only |
| `syscall(N, ...)` calls and raw `syscall` instructions with a constant number | Heuristic | Warning only (error with `--strict`) |
| `madvise()` calls with constant advice | Heuristic | Warning only (error with `--strict`) |

In cargo-libc, `cargo libc inspect FILE --distro ID` (or `--kernel X.Y`, or
`--scan` with `--max-glibc`) runs this scan after its own direct glibc check,
with the same `--strict` and `--verbose` meanings, and adds a `scan` object to
its `--json` report. A scan cannot see syscalls inside the C library, numbers computed
at run time, `dlopen`ed libraries, new flags on old syscalls, or whether a
fallback works; see the [crate README](../crates/linux-abi-scan/README.md).

## 4. Run under an older kernel

Both runners make the host kernel answer like the selected kernel:
syscalls it lacks return `ENOSYS`, `madvise` advice it lacks returns `EINVAL`,
and everything else behaves as on the host. Both use a **seccomp filter**, which
is a classic-BPF program. They differ in who builds the filter and which
userland the program sees.

| | Local filter (`--runner local`) | Container (`--runner container`) |
| --- | --- | --- |
| Status | Implemented; all 47 profiles verified on a 6.18 kernel | Implemented; generic kernels and three presets verified with crun in CI |
| Kernel simulated | Yes | Yes |
| Userland (glibc, `/etc`, tools) | **The host's** | The image's (e.g. real SLES 12 SP5) |
| Filter built by | This crate: BPF from compiled-in data | The runtime's libseccomp, from the generated OCI JSON profile |
| Needs | Linux x86_64 host with seccomp filters (3.5+) | podman or docker; crun with libseccomp 2.6+ to name the newest syscalls |
| Root | **No** (sets `no_new_privs`, then installs the filter) | No with rootless podman; docker needs access to its daemon |
| Catches | Code that needs a newer kernel | The same, plus the binary actually loading and running on the old glibc |
| Cost | Process start only | Image pull and container start |

Choose the local runner for fast tests of kernel behavior and when no container
runtime is available. Choose the container when you need the real target
userland, which is the only way to *run* (not just link-check) against the old
glibc.

### Default runner

`kernel_abi_tools::Runner::choose` applies these rules (tested):

| Options | Runner |
| --- | --- |
| `--runner local` or `--runner container` | As requested |
| `--image IMAGE`, or `--distro ID` (the preset's image) | Container |
| `--kernel X.Y` alone | Local |
| `--runner local --image IMAGE` | Error: the local runner cannot use an image |
| `--runner container --kernel X.Y` without an image | Error: the container needs one |

`--runner local --distro ID` is allowed. It simulates the preset's kernel,
including backports, and reports that the userland is the host's.

### Commands

```sh
# Local filter: host userland, no container runtime, no root.
kernel-seccomp run --kernel 4.12 -- ./target/release/app
kernel-seccomp run --runner local --distro sles-12-sp5 -- ./target/release/app

# Container: the preset's real userland under the equivalent profile.
kernel-seccomp run --distro sles-12-sp5 -- ./target/release/app
kernel-seccomp run --kernel 4.12 --image registry.example/old:tag -- ./app
```

The local runner's filter cannot be removed and is inherited by every child.
Inside it, setuid programs such as `sudo` cannot gain privileges.

### Through Cargo

Cargo's runner setting applies the simulation to `cargo run`, `cargo test`, and
`cargo bench` target executables. Build scripts and procedural macros are not
affected. Tested with the local runner:

```sh
export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER="kernel-seccomp run --kernel 3.10 --"
cargo test
```

### Why not eBPF

seccomp's classic BPF is the right tool: an unprivileged process can install
it. eBPF programs (the `bpf()` syscall, tracing, BPF-LSM) need `CAP_BPF` or
`CAP_SYS_ADMIN`, so effectively root, and add nothing needed to answer
`ENOSYS`/`EINVAL`.

## 5. Use the libraries directly

| Need | API |
| --- | --- |
| Inspect a built file (tier 1) | `linux_abi_scan::scan_file`, `ScanTarget::from_selection`, `Report::{to_text, to_tsv, exit_code}` |
| Resolve user options to floors | `linux_targets::Selection::resolve`, `Selection::describe` |
| What a target lacks | `kernel_abi_tools::missing_syscalls`, `madvise_accepts` |
| OCI profile for a container runtime | `kernel_abi_tools::seccomp_profile` |
| BPF program for a target | `kernel_abi_tools::filter::program` (with `filter::evaluate` for tests) |
| Pick a runner | `kernel_abi_tools::Runner::choose` |
| Run a program | `kernel_abi_tools::local::run`, `kernel_abi_tools::container::run` |

All data is compiled in, so no network or data files are needed at run time.

## 6. How this translates into cargo-libc

cargo-libc depends on the published crates: either `linux-abi-tools` or the
individual crates. crates.io does not accept git dependencies, so a published
cargo-libc cannot pin a commit of this repository. Offline builds use
`cargo vendor`.

| cargo-libc | Calls | Status |
| --- | --- | --- |
| `--distro`, `--glibc`, `--kernel`, `--target triple.X` on every command | `Selection::resolve`; print `Selection::describe` so "kernel not simulated" is visible | Implemented on `build`, `test`, `run`, and `inspect` (`inspect` names the glibc option `--max-glibc`); `doctor` takes `--glibc`/`--target` only |
| `cargo libc build` | Provider for the glibc floor | Implemented (provider supplied by the user) |
| `cargo libc check` (F03) | Same selection and target configuration, no link | Implemented (cargo-libc 772548dbc95b515dfb224702fdbb85b62f48a751); checked with every glibc provider in CI |
| `cargo libc inspect` | `linux_abi_scan::scan` on the bytes it already read, with `ScanTarget::from_selection`; `--strict` maps to `Report::exit_code(true)` | Implemented (cargo-libc 0a500d4db5339f4819990090e1533783e920a9d7); floors resolved with `Selection::resolve` |
| `cargo libc build` (after linking) | With `--distro`/`--kernel`, the same scan on the selected `--artifact`; `--strict` | Implemented (cargo-libc 2936c476d960b1cc9c860bd9396bb7cd5411f83f) |
| `cargo libc test` / `run` with `--runner local\|container` | Sets Cargo's target runner to the `libc-runner` helper (also for doctests, with the target flags given to rustdoc). The helper refuses a binary that needs a newer glibc than selected or, on the host userland, than the host's libraries define (warning when it cannot tell), then calls `Runner::choose` and `local::run` or `container::run` | Implemented (cargo-libc 2936c476d960b1cc9c860bd9396bb7cd5411f83f; host glibc check in b809806500415b2053e26b4dedc55847fe5fb054); local runner checked in its Linux smoke test and in this repository's provider integration for every release 2.17–2.44, including refusal on providers newer than the CI host |
| `cargo libc doctor` | With `--distro`/`--kernel`: runs `/bin/sh` under `local::run` and `true` under `container::run` in the selected image; prints the engine and runtime versions; exit 2 when the runner `test` would choose fails | Implemented (cargo-libc, 2026-10-11) |

A typical flow (`build` and `test` take the same `--provider`, or `--sysroot` and `--gcc-runtime`, inputs):

```sh
cargo libc build --distro sles-12-sp5 --provider provider.txt --artifact ... -- --release
cargo libc inspect target/.../app --distro sles-12-sp5          # glibc + kernel floors
cargo libc test  --distro sles-12-sp5 --provider provider.txt --runner local   # fast: kernel only
cargo libc test  --distro sles-12-sp5 --provider provider.txt                  # container: image userland
```

The runner helper follows the existing `libc-linker` pattern: a sibling
executable installed with `cargo-libc` that Cargo invokes, so Cargo's own
process and the build scripts never run under the filter.

### Release order

1. Publish `glibc-abilists` (its own repository), then `linux-targets`,
   `kernel-abi-tools`, `linux-abi-scan`, `libc-abi-tools`, and `linux-abi-tools`. Each depends
   only on crates published before it.
   `cargo package --workspace` verifies the set together before anything is
   uploaded.
2. Point cargo-libc at the published versions, implement the rows above, and
   qualify them in cargo-libc's CI.

## 7. glibc ABI data

The ABI lists come from glibc's own signed release tags through the
[glibc-abilists](https://github.com/rustcommons/glibc-abilists) crate
(LGPL-2.1-or-later), which records each tag's signature check and each file's
blob id and SHA-256.

| Need | How |
| --- | --- |
| Default | `libc-abi-tools` compiles the lists in (`bundled-glibc-data` feature) |
| Leave the LGPL data out of a build | `default-features = false`; pass `--data DIR` (or `Snapshot::load`) |
| Files on disk for scripts or audits | `libc-abi glibc export-data --output DIR` writes the lists, `SOURCE.tsv`, and `COPYING.LIB` |
| cargo-libc | Same choice, passed through as its own feature (planned) |

## 8. Open decisions

- **Provider delivery:** data crate names and versioning, the corresponding-source
  arrangement for `prebuilt`, and the compiler runtime
  (see [PROVIDER-DELIVERY.md](PROVIDER-DELIVERY.md#open-questions)).
- **Distribution backports** remain unsourced for most presets; until sourced, a
  preset simulates its mainline base kernel.

## 9. Producing the data and providers

### 9.1 glibc ABI lists

Repository [glibc-abilists](https://github.com/rustcommons/glibc-abilists); its
collector is MIT and not published.

1. Clone glibc's repository without file contents (`git clone --bare --filter=blob:none`).
2. For each release tag in `upstream.toml`'s range (2.17–2.44), require the tag object
   to agree between sourceware and the GitHub mirror.
3. Verify the tag's signature against the release managers' keys in `keys/`, merged
   from every source. 2.30's key is unpublished, which is recorded as an exception.
4. Select the x86_64 `*.abilist` files by the rule in `upstream.toml`, and record each
   file's path, git blob id, SHA-256, size, and the release's license evidence.
5. Write `data/` and `src/generated.rs`; `--check` regenerates and compares instead.
6. Publish the crate. `libc-abi-tools` compiles the lists in (feature
   `bundled-glibc-data`).

### 9.2 Kernel data

`crates/kernel-abi-tools/scripts`, run online only when refreshing:

1. `fetch-syscall-tables.sh`: for each mainline Linux tag, download the x86_64 syscall
   table and the generic `mman` header.
2. `build-syscall-data.sh` and `build-madvise-data.sh`: derive the first release of every
   64-bit syscall and `madvise` advice value into the committed TSV data. The 3.3
   floor is reported as the floor, not as the true origin.
3. Build: the data is compiled in, and the seccomp profiles (OCI JSON and classic BPF)
   are generated from it; unit tests check both against the data.
4. `self-test.sh` with `syscall-probe`: run every profile with the local runner and in
   a container (crun with libseccomp 2.6+).

### 9.3 Provider pipeline

One release at a time, in `examples/` (`glibc-two-paths/` holds the shared scripts
and 2.17's two routes; `glibc-X/` holds each later release's pinned inputs). Step
by step:

1. **Pin the inputs:** `examples/glibc-X/INPUTS.toml` records:
   - the GNU archive URL and SHA-256, and its signature check;
   - the digest-pinned builder image and the signed package archive;
   - the configure options and any source patch.

   Builders:
   - CentOS 7 for 2.17–2.25;
   - Debian 9 for 2.26–2.36;
   - Debian 12 for 2.37–2.44.
2. **Fetch the source:** `fetch-glibc.sh` downloads from ftp.gnu.org, falling back to
   the kernel.org GNU mirror, and requires the pinned SHA-256.
3. **Build glibc from source in the builder:** `run.sh` starts the pinned image and
   `build-inputs.sh` inside it:
   - installs the signature-checked packages and records them;
   - copies the builder's real `libgcc_s.so.1`;
   - configures (`--prefix=/usr --libdir=/usr/lib64 --disable-werror --enable-kernel=3.2`),
     builds, and installs into an isolated `source/` tree;
   - copies the kernel UAPI headers.

   2.17 also has a `prebuilt` route made of CentOS 7 packages, and a one-line
   source adjustment for its freestanding header build.
4. **Audit the real libraries:** `elf-and-check.sh`:
   - reads each of the eight real DSOs with `libc-abi glibc elf` and compares the result
     with GNU `readelf`;
   - compares every public versioned definition with the pinned ABI list
     (`libc-abi glibc snapshot`), requiring zero differences;
   - writes the release's matrix and a manifest of input hashes.
5. **Generate and audit the stubs:** `prepare-and-check.sh`:
   - generates the eight link-only DSOs from the measured metadata;
   - checks the exact exported set, default and compatibility versions, types,
     binding, visibility, object sizes, alignment, shared storage, SONAME and
     absence of dependencies (`check-full.py --abi-only`).
6. **Exhaustive linking:** `check-link-matrix.py`, against each isolated stub library.
   - Every interface present in the release links.
   - Every absent interface fails.
   - Every measured default binds to its version.
   - Every compatibility-only name rejects a new unversioned reference.
   - A synthetic TLS fixture covers TLS relocations.
7. **Package for CI** (CI only): `ci-bundle.py pack` writes a checksummed inventory
   for the application jobs; `unpack` verifies it on each runner.
8. **Applications on the real runtime:** `application-and-check.sh` and
   `behavior-and-check.sh`.
   - **Rust program:** allocation, threads and TLS destructors, and unwinding.
   - **C programs:** both `memcpy` versions, copy-relocated historical data views,
     `fmemopen` versions and default, the pthread default-attribute boundary, and the
     `sigvec` default removal.
   - **Too-new link test:** a matrix-chosen symbol (`too-new-symbol.py`) must fail to
     link.
   - **Five library behavior suites:** libm, libdl, librt, libutil and libresolv.

   Everything runs under the release's real loader. Runtime dependency paths and
   loaded modules are audited (`check-full.py --application-only`,
   `check-behavior.py`).
9. **Cargo integration:** `provider-and-check.sh` exports a v1 descriptor
   (`provider_glibc` example) and uses cargo-libc for:
   - `build` with the versioned target and the tier 1 scan;
   - `run` under a Linux 3.10 local filter;
   - the host-glibc refusal (for providers newer than the host);
   - `check`;
   - conflicting `--glibc` and mismatched-release rejection;
   - workspace, feature, profile and cached-build fixtures;
   - the matrix-chosen too-new Cargo link failure.

   Link maps and runtime paths are audited (`check-provider.py`).

### 9.4 Evidence and reference

1. `scripts/import-glibc-elf-profile.py` adds the release's measured profile to
   `data/glibc-history/` after rechecking hashes, membership and differences, and
   refuses to replace an existing profile.
2. `scripts/build-glibc-reference.py` regenerates the matrices and the Markdown
   interface reference; `--check` verifies it.
3. The release's `docs/GLIBC-X.md` and the [provider index](PROVIDERS.md) record the
   inputs, results, measured changes from the previous release and hosted runs.

### 9.5 Hosted qualification

`.github/workflows/glibc-2.17.yml`, on every push:
- one **preparation** job per release runs steps 2–7;
- two **application** jobs per release, one per Clang (21 and 24), run steps 8–9 with
  Rust 1.99 and the pinned cargo-libc commit;
- diagnostics are kept as artifacts.

Main CI covers formatting, tests, clippy (Windows and Linux), the minimum Rust
version, the ABI lists and the reference check.

### 9.6 Release

[RELEASING.md](RELEASING.md):
1. Set one version across the five crates.
2. Run the full validation and `cargo package --workspace --locked`.
3. Publish in dependency order from a commit with green CI.
4. Tag `vX.Y.Z` and record the release.

glibc-abilists releases from its own repository first.

## 10. Planned workflows

### 10.1 Provider delivery

Design: [PROVIDER-DELIVERY.md](PROVIDER-DELIVERY.md). The user runs
`cargo libc build --target x86_64-unknown-linux-gnu.2.28`; with no provider given:

1. **Resolve** the release and target, and look for a cached provider whose content
   hash matches.
2. **Prepare** one in the selected mode, if none is cached:
   - **`source` (default):**
     1. Read the release's headers, startup and nonshared sources, generated
        configuration headers and recorded compile commands from
        `glibc-provider-sources`, and check their hashes.
     2. Compile `crt1.o`, `crti.o`, `crtn.o` and `libc_nonshared.a` with Clang.
     3. Generate the stubs from the ABI data.
   - **`prebuilt`:**
     1. Read the CI-built objects from `glibc-provider-binaries` and require their
        recorded SHA-256 and CI run.
     2. Take the headers from the sources crate.
     3. Generate the stubs.
   - **`container`:** run the [provider pipeline](#93-provider-pipeline) steps 1–5
     locally with podman or docker.
3. **Assemble** the provider layout (sysroot, stubs, compiler runtime), write the
   descriptor and `manifest.json`, and audit the stubs as in step 5 of 9.3.
4. **Cache** it under `$CARGO_HOME/libc-providers/...`, then build as in
   [section 2](#2-build-against-an-older-glibc).

`cargo libc provider prepare|list|path|verify|export|import|remove` (and
`libc-abi provider ...`) expose the same steps. CI qualifies every release in every
mode before publishing the data crates:
- stubs and headers byte-identical across modes;
- `prebuilt` byte-identical to the qualified container build;
- `source` objects equivalent in symbols, sections and relocations;
- full suites against each mode's provider.

### 10.2 Distribution-specific providers

Later release ([roadmap](ROADMAP.md#future-release-distribution-specific-providers)).

1. Pin a redistributable distribution's binary and source packages, with checksums,
   signatures, licenses and terms.
2. Extract the same provider parts from them instead of building GNU glibc.
3. Measure the profile and compare it with the generic build, recording every
   difference in [DISTRIBUTION-DIFFERENCES.md](DISTRIBUTION-DIFFERENCES.md).
4. Run steps 4–9 of the provider pipeline, plus a run in the distribution's own image.
5. Publish with the input's identity in the descriptor.

### 10.3 Source-built builder toolchains

Long-term ([roadmap](ROADMAP.md#long-term-source-built-builder-toolchains)).

1. Start from a minimal auditable seed.
2. Build era-matched binutils, GCC (with libgcc), make, bison, gawk and Python from
   signed upstream sources, and the UAPI headers from kernel.org.
3. Produce the builder images from committed Containerfiles, recording every input.
4. Build twice on different hosts, and require identical results.
5. Use these images in step 3 of the provider pipeline, and show the providers match
   today's vendor-built ones.

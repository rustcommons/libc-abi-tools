# CLAUDE.md

This file provides working guidance for any coding agent.

## Project overview

`linux-abi-tools` (formerly `libc-abi-tools`, renamed 2026-10-10) is a Cargo
workspace of rustcommons' ABI tools for targeting older Linux systems from Rust:

| Crate | Location | Scope |
| --- | --- | --- |
| `libc-abi-tools` | repository root | glibc ABI metadata, stub generation, and providers, inspired by (and partly adapted from) [Zig's libc ABI tools](https://codeberg.org/ziglang/libc-abi-tools); ABI data from glibc via [glibc-abilists](https://github.com/rustcommons/glibc-abilists); other libc implementations are future work |
| `kernel-abi-tools` | `crates/kernel-abi-tools` | Linux kernel syscall and `madvise` data and seccomp profiles that simulate older kernels |
| `linux-targets` | `crates/linux-targets` | Distribution presets (glibc and kernel floors, image) and target selection rules |
| `linux-abi-scan` | `crates/linux-abi-scan` | Static inspection of built executables against the floors: tier 1 of the testing tiers |
| `linux-abi-tools` | `crates/linux-abi-tools` | Re-exports the four crates above; no code of its own |

The companion Cargo integration is `rustcommons/cargo-libc`, which depends on
the published crates. `cargo` in the root acts on `libc-abi-tools` alone unless
given `--workspace` or `-p`. [docs/WORKFLOWS.md](docs/WORKFLOWS.md) describes
every workflow and its cargo-libc mapping; keep it in step with changes.

## Scope and implementation: libc-abi-tools

- Support glibc target releases starting at 2.17. Retain earlier GLIBC symbol
  versions: later releases still export and require them.
- glibc ABI data comes only from glibc's own release tags, through the
  `glibc-abilists` crate (default `bundled-glibc-data` feature, opt-out with
  `default-features = false` and `--data DIR`). Never vendor ABI lists here or
  take them from Zig or another redistributor.
- Pin upstream references to a commit, document derivations, and preserve the
  Zig contributors' copyright and MIT license with code adapted from Zig.
- Keep ABI collection/serialization separate from linker stub generation and
  real startup objects, headers, and runtime implementations.
- Preserve symbol versions, library identity, symbol kind, and object/TLS sizes.
- Use deterministic outputs and validate malformed inputs without panics.
- Keep implementation dependencies minimal; start with Rust's standard library.
- Ask about unresolved product decisions; do not claim Zig parity prematurely.

## Scope and implementation: kernel-abi-tools

- Support target kernels from 3.10; the syscall data floor is v3.3. Rust std's
  documented minimum (3.2) is treated as the floor and must be reported as such.
- Derive kernel data from upstream kernel tables and headers at release tags,
  record provenance, and regenerate with the scripts rather than editing
  generated files by hand.
- Profiles are fail-closed allowlists: default `ENOSYS`, allow only what the
  target kernel provides. Never switch to a blocklist; runtimes drop unknown
  names and a blocklist fails open. Argument rules must not overlap.
- Profiles simulate syscall availability and `madvise` advice only. Other
  flags, behavior, and vendor backports must be documented as planned until
  implemented and validated.
- Two runners apply the simulation: the local runner (in-process classic-BPF
  filter from `filter::program`, host userland, no root) and the container
  runner (OCI profile, image userland). The BPF filter and the JSON profile
  must make the same decisions; `filter` tests check both against the data.

## Scope and implementation: linux-targets

- Presets are aliases for a glibc floor, kernel floor, and image. Only mark
  values as checked when they were observed; backports must be sourced, never
  guessed.
- `--distro` excludes `--glibc`, `--kernel`, and a versioned `--target`. A
  missing kernel floor means the host kernel and must be reported as "not
  simulated".

## Scope and implementation: linux-abi-scan

- Tier 1 of three (inspect, simulate, qualify); always document it as the
  least reliable tier and never present a clean scan as compatibility.
- Every finding states its confidence: `Exact` only for ELF metadata the
  loader uses; anything recovered from machine code is `Heuristic`.
- Take kernel facts only from `kernel-abi-tools` and floors only from
  `linux-targets`, so the scan and the seccomp simulation cannot disagree.
- Parse untrusted ELF input with bounds checks and no panics.

## Shared rules

- Every crate stays std-only apart from rustcommons crates: `kernel-abi-tools`
  may depend on `linux-targets`, `linux-abi-scan` on both of those,
  `libc-abi-tools` on `glibc-abilists`, and `linux-abi-tools` on the other four.
- Releases go to crates.io only (see [docs/RELEASING.md](docs/RELEASING.md)).
  Keep each manifest's `include` list minimal. The glibc ABI lists
  (LGPL-2.1-or-later) are published only in `glibc-abilists`, which must be
  released before `libc-abi-tools`.
- Ask about unresolved product decisions; do not invent compatibility claims.

## Validation

Run `cargo fmt --all -- --check`, `cargo test --locked --offline`, and
`cargo clippy --all-targets --locked --offline -- -D warnings` for Rust changes;
add `--workspace` (or `-p kernel-abi-tools -p linux-targets`) when the other
crates change.
Use pinned upstream fixtures to validate the glibc port. Document which
architectures/releases and which stages have actually been tested.
Kernel profile or filter changes also require
`crates/kernel-abi-tools/scripts/self-test.sh` on Linux, both with `local`
(no root needed) and with a container runtime (crun with libseccomp 2.6+, e.g.
from `scripts/install-crun.sh`, covers every profile). Packaging changes
require `cargo package --workspace --locked`; never publish to crates.io
without the user's go-ahead. Distinguish host simulation from actual
oldest-target runtime qualification on a real kernel.

## Git conventions

Do **not** add `Co-Authored-By: Claude ...` trailers to commit messages on this repo, even if the harness's default instructions suggest it. Commit messages are the human-authored record of intent; tool attribution belongs in tool logs, not history. This overrides the default trailer behavior.

Apply the same no-tool-coauthor convention to all coding agents.

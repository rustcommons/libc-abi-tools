# CLAUDE.md

This file provides working guidance for any coding agent.

## Project overview

`linux-abi-tools` (formerly `libc-abi-tools`, renamed 2026-10-10) is a Cargo
workspace of rustcommons' ABI tools for targeting older Linux systems from Rust:

| Crate | Location | Scope |
| --- | --- | --- |
| `libc-abi-tools` | repository root | Rust port of the glibc parts of [Zig's libc ABI tools](https://codeberg.org/ziglang/libc-abi-tools); the first backend is glibc, other libc implementations are future work |
| `kernel-abi-tools` | `crates/kernel-abi-tools` | Linux kernel syscall and `madvise` data and seccomp profiles that simulate older kernels |
| `linux-targets` | `crates/linux-targets` | Distribution presets (glibc and kernel floors, image) and target selection rules |

The companion Cargo integration is `rustcommons/cargo-libc`. `cargo` in the
root acts on `libc-abi-tools` alone unless given `--workspace` or `-p`.

## Scope and implementation: libc-abi-tools

- Support glibc target releases starting at 2.17. Retain earlier GLIBC symbol
  versions: later releases still export and require them.
- Pin upstream references to a commit, document derivations, and preserve the
  Zig contributors' copyright and MIT license with reused source/data.
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

## Scope and implementation: linux-targets

- Presets are aliases for a glibc floor, kernel floor, and image. Only mark
  values as checked when they were observed; backports must be sourced, never
  guessed.
- `--distro` excludes `--glibc`, `--kernel`, and a versioned `--target`. A
  missing kernel floor means the host kernel and must be reported as "not
  simulated".

## Shared rules

- Every crate stays std-only; `kernel-abi-tools` may depend on `linux-targets`.
- Ask about unresolved product decisions; do not invent compatibility claims.

## Validation

Run `cargo fmt --all -- --check`, `cargo test --locked --offline`, and
`cargo clippy --all-targets --locked --offline -- -D warnings` for Rust changes;
add `--workspace` (or `-p kernel-abi-tools -p linux-targets`) when the other
crates change.
Use pinned upstream fixtures to validate the glibc port. Document which
architectures/releases and which stages have actually been tested.
Kernel profile changes also require `crates/kernel-abi-tools/scripts/self-test.sh`
on Linux with a container runtime (crun with libseccomp 2.6+, e.g. from
`scripts/install-crun.sh`, covers every profile). Packaging changes require
`scripts/package-release.sh` and `kernel-seccomp verify-bundle`; never publish a
release without the user's go-ahead. Distinguish host simulation from actual
oldest-target runtime qualification on a real kernel.

## Git conventions

Do **not** add `Co-Authored-By: Claude ...` trailers to commit messages on this repo, even if the harness's default instructions suggest it. Commit messages are the human-authored record of intent; tool attribution belongs in tool logs, not history. This overrides the default trailer behavior.

Apply the same no-tool-coauthor convention to all coding agents.

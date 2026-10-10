# CLAUDE.md

This file provides working guidance for any coding agent.

## Project overview

`kernel-abi-tools` is rustcommons' Linux kernel ABI data and seccomp-based
kernel simulation for testing Rust binaries against older target kernels.
The first architecture is x86_64. Its companions are `rustcommons/cargo-libc`
(Cargo integration) and `rustcommons/libc-abi-tools` (glibc ABI data).

## Scope and implementation

- Support target kernels from 3.10; the syscall data floor is v3.3. Rust std's
  documented minimum (3.2) is treated as the floor and must be reported as such.
- Derive syscall data from pinned upstream kernel tables, record provenance,
  and regenerate with the scripts rather than editing generated files by hand.
- Profiles are fail-closed allowlists: default `ENOSYS`, allow only what the
  target kernel provides. Never switch to a blocklist; runtimes drop unknown
  names and a blocklist fails open.
- Distribution presets are aliases for a kernel floor, glibc floor, and image.
  Only mark values as checked when they were observed; backports must be
  sourced, never guessed.
- Profiles simulate syscall availability only. Flags, behavior, and vendor
  backports must be documented as planned until implemented and validated.
- Keep Rust implementation dependencies minimal. The initial tool uses std only.
- Ask about unresolved product decisions; do not invent compatibility claims.

## Validation

Run `cargo fmt --all -- --check`, `cargo test --locked --offline`, and
`cargo clippy --all-targets --locked --offline -- -D warnings` for Rust changes.
Profile changes also require `scripts/self-test.sh` on Linux with a container
runtime (crun with libseccomp 2.6+, e.g. from `scripts/install-crun.sh`, covers
every profile). Packaging changes require `scripts/package-release.sh` and
`kernel-seccomp verify-bundle`; never publish a release without the user's
go-ahead. Distinguish host simulation from actual oldest-target runtime
qualification on a real kernel.

## Git conventions

Do **not** add `Co-Authored-By: Claude ...` trailers to commit messages on this repo, even if the harness's default instructions suggest it. Commit messages are the human-authored record of intent; tool attribution belongs in tool logs, not history. This overrides the default trailer behavior.

Apply the same no-tool-coauthor convention to all coding agents.
Source: [cargo-libc/CLAUDE.md](https://github.com/rustcommons/cargo-libc/blob/main/CLAUDE.md)
and [libc-abi-tools/CLAUDE.md](https://github.com/rustcommons/libc-abi-tools/blob/main/CLAUDE.md).

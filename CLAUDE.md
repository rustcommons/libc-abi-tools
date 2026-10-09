# CLAUDE.md

This file provides working guidance for any coding agent.

## Project overview

`libc-abi-tools` is rustcommons' Rust port of the glibc parts of
[Zig's libc ABI tools](https://codeberg.org/ziglang/libc-abi-tools).
The first backend is glibc; other libc implementations are future work.
The companion Cargo integration is `rustcommons/cargo-libc`.

## Scope and implementation

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

## Validation

Run `cargo fmt --all -- --check`, `cargo test --locked --offline`, and
`cargo clippy --all-targets --locked --offline -- -D warnings` for Rust changes.
Use pinned upstream fixtures to validate the glibc port. Document which
architectures/releases and which stages have actually been tested.

## Git conventions

Do **not** add `Co-Authored-By: Claude ...` trailers to commit messages on this repo, even if the harness's default instructions suggest it. Commit messages are the human-authored record of intent; tool attribution belongs in tool logs, not history. This overrides the default trailer behavior.

Apply the same no-tool-coauthor convention to all coding agents.

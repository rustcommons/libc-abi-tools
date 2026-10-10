# Releasing

The workspace publishes four crates to **crates.io only**, after
[glibc-abilists](https://github.com/rustcommons/glibc-abilists), which holds the
glibc ABI lists and is released from its own repository first. No standalone
binary bundles or GitHub release assets are published. Nothing is published
without the maintainer's go-ahead.

| Crate | Depends on | Contains |
| --- | --- | --- |
| `linux-targets` | none | Library; distribution presets compiled in |
| `kernel-abi-tools` | `linux-targets` | Library and the `kernel-seccomp`/`syscall-probe` binaries; syscall and `madvise` data compiled in |
| `libc-abi-tools` | `glibc-abilists` (default `bundled-glibc-data` feature) | Library and the `libc-abi` binary; no data of its own |
| `linux-abi-tools` | the three above | Re-exports only |

Each manifest has an `include` list, so only sources, compiled-in data,
README, and license/notice files are packaged. `libc-abi-tools` leaves out
`data/`, `docs/`, `examples/`, and `tests/`; its own crate is MIT. A default
build of it includes glibc-abilists' LGPL-2.1-or-later data; build with
`default-features = false` to leave that out.

Before publishing `libc-abi-tools`, `glibc-abilists` must be on crates.io: the
manifest's `git` source is for development, and `cargo publish` uses its
`version` requirement.

## Cutting a release

1. Set the same `version` in the four manifests and in the internal
   dependency requirements (`version = "X.Y.Z"` next to each `path`). Run
   `cargo check --workspace` to update `Cargo.lock`, and record the release in
   the roadmaps.
2. Run the full validation from [CLAUDE.md](../CLAUDE.md). Kernel changes also
   require `crates/kernel-abi-tools/scripts/self-test.sh` with both `local` and
   a container image.
3. Dry run: `cargo package --workspace --locked`. Cargo packages all four
   crates and builds each from its packaged sources against a temporary
   registry that holds the others. Inspect the file lists with
   `cargo package --list -p <crate>`.
4. With CI green on the commit, publish in dependency order:

   ```sh
   # first, in the glibc-abilists repository: cargo publish
   cargo publish -p linux-targets
   cargo publish -p kernel-abi-tools
   cargo publish -p libc-abi-tools
   cargo publish -p linux-abi-tools
   ```

   (`cargo publish --workspace` does the same in one command on Cargo 1.90+.)
5. Tag the commit `vX.Y.Z` and push the tag.

A published version cannot be replaced. `cargo yank` only stops new lockfiles
from selecting it.

## Versioning

All four crates share one version while they are 0.x.

- **Patch:** data refreshes and preset corrections that do not loosen any
  simulation, documentation, and fixes.
- **Minor (0.x):** new presets, kernels, rules, API or CLI additions, and any
  breaking change. A simulation may become stricter (closer to the real
  kernel); release notes call this out.

## Offline and disconnected networks

Data is compiled into the crates, so nothing is fetched at run time.

- **Build from crates.io offline:** run `cargo vendor` on a connected machine
  for the consuming project (for example cargo-libc). Carry `vendor/` and the
  printed `.cargo/config.toml` snippet across, then build with `--offline`.
- **Install the binaries offline:** `cargo install kernel-abi-tools
  --locked --offline` against the vendored sources, or build once and copy
  `kernel-seccomp`.
- **Container runner:** `podman save`/`podman load` each image, or push it to
  an internal registry and pass `--image`. If the hosts' libseccomp is older
  than 2.6, carry the static crun from `scripts/install-crun.sh` and verify
  its pinned checksum.
- **Local runner:** needs nothing beyond the binary.

Checksums on crates.io protect integrity, not publisher identity. Mirror trust
is a separate decision (see the kernel roadmap).

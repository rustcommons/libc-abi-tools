# Releasing

The workspace publishes five crates to **crates.io only**, after
[glibc-abilists](https://github.com/rustcommons/glibc-abilists), which holds the
glibc ABI lists and is released from its own repository first. No standalone
binary bundles or GitHub release assets are published. Nothing is published
without the maintainer's go-ahead.

| Crate | Depends on | Contains |
| --- | --- | --- |
| `linux-targets` | none | Library; distribution presets compiled in |
| `kernel-abi-tools` | `linux-targets` | Library and the `kernel-seccomp`/`syscall-probe` binaries; syscall and `madvise` data compiled in |
| `libc-abi-tools` | `glibc-abilists` (default `bundled-glibc-data` feature), `glibc-provider-sources` (default `provider-sources` feature) | Library and the `libc-abi` binary; no data of its own except the `libgcc_s` table and the container build scripts (`examples/glibc-*/build-inputs.sh`, `INPUTS.toml`) |
| `linux-abi-scan` | `kernel-abi-tools`, `linux-targets` | Library and the `linux-abi-scan` binary (tier 1 static inspection) |
| `linux-abi-tools` | the four above | Re-exports only |

Each manifest has an `include` list, so only sources, compiled-in data,
README, and license/notice files are packaged. `libc-abi-tools` leaves out
`data/`, `docs/`, `examples/`, and `tests/`; its own crate is MIT. A default
build of it includes glibc-abilists' LGPL-2.1-or-later data; build with
`default-features = false` to leave that out.

`glibc-abilists` and `glibc-provider-sources` must be on crates.io in the
required versions before `libc-abi-tools` is published; both release from their own
repositories. A dependency that is still a git revision (with a `version` too, as
`glibc-provider-sources` was before 0.3.0) packages as its version, so `cargo package`
cannot verify `libc-abi-tools` until that version is on crates.io. Publish it first,
then drop the `git`/`rev` keys and update `Cargo.lock`.

## Release record

| Version | Date | Crates | Commit (tag) |
| --- | --- | --- | --- |
| 0.1.0 | 2026-10-11 | `glibc-abilists` | `4ffb9f698264d7d8bd834f6f2b5e03926bdc02fb` (glibc-abilists `v0.1.0`) |
| 0.1.0 | 2026-10-11 | `linux-targets`, `kernel-abi-tools`, `linux-abi-scan`, `libc-abi-tools`, `linux-abi-tools` | `f691c048fc2b55d6c7d891ac0a8a5e03db5ccb22` (`v0.1.0`) |
| 0.2.0 | 2026-10-11 | `linux-targets`, `kernel-abi-tools`, `linux-abi-scan`, `libc-abi-tools`, `linux-abi-tools` | `921cf79cc2780a88e917aa9542955a746214f6a1` (`v0.2.0`) |
| 0.1.0 | 2026-10-13 | `glibc-provider-sources` | `1ae578cf7123399b0c710f44defab93cc2021001` (glibc-provider `v0.1.0`) |
| 0.3.0 | 2026-10-13 | `linux-targets`, `kernel-abi-tools`, `linux-abi-scan`, `libc-abi-tools`, `linux-abi-tools` | `6c3dd725e8b7ed9d8265dba360b7db021c3afc54` (`v0.3.0`) |

**0.2.0:** `libc-abi-tools` accepts provider descriptors for every glibc release in the
pinned ABI lists, 2.17 through 2.44 (0.1.0 accepted 2.17, 2.18 and 2.22), exposes them as
`provider::RELEASES`, and requires `libpthread_nonshared.a` only before 2.28. The other
crates are unchanged apart from the shared version. The [provider index](PROVIDERS.md)
lists each release's qualification.

**0.3.0:** `libc-abi-tools` prepares providers itself: the `provider` module and the
`libc-abi provider` commands (`prepare`, `path`, `list`, `verify`, `export`, `import`, `remove`)
build one in `source` mode, from `glibc-provider-sources` with the host's GCC, or in
`container` mode, from glibc's release archive in a container, and cache it. Every
provider links against a GCC 4.8.5 `libgcc_s` stub. The host glibc check picks only
symbols newer than the host's glibc. Hosted CI qualifies both modes for every release
2.17 through 2.44 ([PROVIDERS.md](PROVIDERS.md)); `prebuilt` mode is still planned.
The other crates are unchanged apart from the shared version.

crates.io limits how many new crate names one account can publish in a short
period; a first release of several crates may need a pause (the sixth crate
here waited a few minutes).

## Cutting a release

1. Set the same `version` in the five manifests and in the internal
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
   # first, in the glibc-abilists and glibc-provider repositories: cargo publish
   cargo publish -p linux-targets
   cargo publish -p kernel-abi-tools
   cargo publish -p linux-abi-scan
   cargo publish -p libc-abi-tools
   cargo publish -p linux-abi-tools
   ```

   (`cargo publish --workspace` does the same in one command on Cargo 1.90+.)
5. Tag the commit `vX.Y.Z` and push the tag.

A published version cannot be replaced. `cargo yank` only stops new lockfiles
from selecting it.

## Versioning

All five crates share one version while they are 0.x.

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

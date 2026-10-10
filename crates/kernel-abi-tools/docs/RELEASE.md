# Releasing

> **Under review (2026-10-10):** cargo-libc is planned to depend on this crate
> directly, which makes the bundle optional. Whether standalone bundles keep
> being published is an open decision; until then this process stays working.

## What a release publishes

Each GitHub release `kernel-abi-tools-vX.Y.Z` (tags are prefixed because the
linux-abi-tools repository holds several crates) has two assets:

| Asset | Contents |
| --- | --- |
| `kernel-abi-tools-X.Y.Z-x86_64-linux.tar.gz` | The bundle described in [PROVIDER-CONTRACT.md](PROVIDER-CONTRACT.md): static binaries, all profiles, data, scripts, docs, descriptor, inventory |
| `kernel-abi-tools-X.Y.Z-x86_64-linux.tar.gz.sha256` | `sha256sum` line for the archive |

GitHub also attaches its automatic source archives. The crate is not published
to crates.io (`publish = false`).

## Cutting a release

1. Update `version` in `crates/kernel-abi-tools/Cargo.toml` (and the root
   `Cargo.lock` via `cargo check -p kernel-abi-tools`), add the release to the
   roadmap, and merge to `main` with CI green.
2. Run the **kernel-abi-tools release** workflow manually
   (`workflow_dispatch`) on `main` as a dry run. It builds and verifies the
   bundle, runs the self-test from the bundle's own binaries, and uploads the
   result as a workflow artifact without publishing anything.
3. Tag the same commit and push the tag:
   `git tag -a kernel-abi-tools-vX.Y.Z -m "kernel-abi-tools X.Y.Z" && git push origin kernel-abi-tools-vX.Y.Z`.
   The tag must match the crate's `Cargo.toml`; the workflow refuses a mismatch.
4. The workflow repeats the dry-run steps and creates the GitHub release with
   both assets. Check the release notes and the published checksum.

Builds are reproducible: the same commit produces the same archive bytes
(fixed file order, owners, modes, timestamps, and `gzip -n`), so a second
build is an independent check of a published asset.

## Taking a release onto a disconnected network

On a connected machine:

```bash
gh release download kernel-abi-tools-vX.Y.Z -R rustcommons/linux-abi-tools
sha256sum -c kernel-abi-tools-X.Y.Z-x86_64-linux.tar.gz.sha256
podman pull registry.suse.com/suse/sles12sp5:latest      # each image you need
podman save -o sles12sp5.tar registry.suse.com/suse/sles12sp5:latest
bash kernel-abi-tools-X.Y.Z-x86_64-linux/scripts/install-crun.sh ./crun   # if the
# disconnected hosts' runtime has libseccomp older than 2.6 (see README)
```

Transfer the archive, its `.sha256`, the image tarballs, and `crun` if
needed (its pinned checksum is in `scripts/install-crun.sh`). On the
disconnected side:

```bash
sha256sum -c kernel-abi-tools-X.Y.Z-x86_64-linux.tar.gz.sha256
tar -xzf kernel-abi-tools-X.Y.Z-x86_64-linux.tar.gz
kernel-abi-tools-X.Y.Z-x86_64-linux/bin/kernel-seccomp verify-bundle kernel-abi-tools-X.Y.Z-x86_64-linux
podman load -i sles12sp5.tar
KERNEL_ABI_ENGINE_ARGS="--runtime $PWD/crun" \
  kernel-abi-tools-X.Y.Z-x86_64-linux/scripts/self-test.sh \
  kernel-abi-tools-X.Y.Z-x86_64-linux/bin/syscall-probe registry.suse.com/suse/sles12sp5:latest sles-12-sp5
```

Images can instead be pushed to an internal registry; `--image` then names the
internal copy.

## Versioning

- **Patch:** data refreshes and preset corrections that do not change what an
  existing profile allows, documentation, and fixes.
- **Minor:** new presets, kernels, rules (such as new argument-level rules), or
  CLI additions. Existing profiles may become stricter (closer to the real
  kernel); this is called out in the release notes.
- **Major:** a new descriptor format or removed CLI behavior.

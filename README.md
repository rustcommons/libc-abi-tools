# kernel-abi-tools

**Test against the kernel you ship to.**

[![CI](https://github.com/rustcommons/kernel-abi-tools/actions/workflows/ci.yml/badge.svg)](https://github.com/rustcommons/kernel-abi-tools/actions/workflows/ci.yml)
![Rust 1.90+](https://img.shields.io/badge/Rust-1.90%2B-orange)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue)](LICENSE)
![Status: prototype](https://img.shields.io/badge/status-prototype-yellow)

Linux kernel ABI data and seccomp profiles that make a modern host kernel
behave, at the syscall-availability level, like an older one. Run a Rust
binary in a container of the target distribution with a profile for its
kernel, and every syscall that kernel lacks returns `ENOSYS`, exactly as it
would on the real machine.

Companion to [cargo-libc](https://github.com/rustcommons/cargo-libc) (build
for an old glibc) and [libc-abi-tools](https://github.com/rustcommons/libc-abi-tools)
(glibc ABI data). Those answer "will it link and load?"; this answers "will its
syscalls work on that kernel?". Initial scope: **x86_64 Linux**. Std-only Rust,
no dependencies, works offline after the one-time data refresh.

**Status: prototype.** Profiles and the probe are verified (see
[Verification](#verification)); distribution kernel backports are not modelled
yet. See [docs/ROADMAP.md](docs/ROADMAP.md).

## Contents

| Path | Purpose |
| --- | --- |
| `data/linux/syscalls-x86_64.tsv` | Every x86_64 syscall and the first mainline release whose table lists it (v3.3 to v7.2, derived from 81 kernel tags) |
| `data/distros.tsv` | 28 distribution presets: kernel floor, glibc, container image |
| `profiles/kernel/linux-X.Y.json` | 19 generic kernel profiles |
| `profiles/distro/ID.json` | One profile per distribution preset |
| `kernel-seccomp` | Generates profiles and lists missing syscalls |
| `syscall-probe` | Proves a profile simulates its kernel on your host and runtime |
| `scripts/run-under-kernel.sh` | Runs a binary under a profile; usable as a Cargo runner |
| `scripts/self-test.sh` | Runs the probe against every profile |

## Usage

```bash
cargo build --release
target/release/kernel-seccomp distros
target/release/kernel-seccomp missing --distro sles-12-sp5
target/release/kernel-seccomp profile --kernel 4.12 --output linux-4.12.json

# Run a binary in SLES 12 SP5 userland with Linux 4.12 syscall availability
scripts/run-under-kernel.sh --distro sles-12-sp5 -- ./my-app args
```

### As a Cargo runner

Cargo runs every `cargo run` / `cargo test` binary through the runner, so the
whole test suite executes in the simulated environment:

```toml
# .cargo/config.toml
[target.x86_64-unknown-linux-gnu]
runner = ["/opt/kernel-abi-tools/scripts/run-under-kernel.sh", "--distro", "sles-12-sp5", "--"]
```

The working directory and the binary's directory are mounted at their own
absolute paths, and `CARGO*`/`RUST*` variables are forwarded. The binary must
be built for a glibc no newer than the image's (use cargo-libc).

## How a profile works

A profile is an OCI seccomp profile (Podman, Docker, CRI-O): **default action
`ENOSYS`, allowlist of every syscall the target kernel has.** It is an
allowlist because runtimes resolve names with their own libseccomp and
silently drop names it does not know. A blocklist would therefore fail open for
the newest syscalls, which is exactly what we observed during development (13
syscalls from 6.8 onward reached the kernel). With the allowlist, an unknown
name fails closed and the probe reports it.

Profiles are a testing aid, not a security boundary.

## Runtime requirements

| Runtime | Works for | Why |
| --- | --- | --- |
| **crun ≥ 1.28 with libseccomp ≥ 2.6** | All profiles (3.10 to 6.12 verified) | Applies the filter immediately before `execve` |
| runc 1.3.x with libseccomp 2.5.x | Kernels 4.11 to 6.7 only | runc's init calls `statx` (4.11) after installing the filter, so older profiles cannot start; libseccomp 2.5 does not know 6.8+ names |

Select crun with `KERNEL_ABI_ENGINE_ARGS="--runtime crun"`, or make it
Podman's default.

## Verification

Measured on 2026-10-10. Host kernel 6.18 (WSL2), Podman 5.8, image
`registry.suse.com/suse/sles12sp5` (glibc 2.22).

- **Self-test, crun 1.28 / libseccomp 2.6.1:** all 19 generic profiles
  verified, 68 probed syscalls each. The only skip is `map_shadow_stack`, which
  this host kernel also lacks.
- **Self-test, runc 1.3.4 / libseccomp 2.5.5:** `sles-12-sp5`, 4.12 and 5.14 pass.
  3.10 fails to start, and 6.12 blocks six 6.8+ syscalls (fail-closed, reported).
- **End to end:** a memmap2 reader built with a glibc 2.17 floor runs correctly
  in SLES 12 SP5 userland under the 4.12 profile. The same program built
  normally on Ubuntu 24.04 is rejected by the SLES loader (`GLIBC_2.28` not
  found).

## What this does not cover

- **Flags and behavior:** a 4.12 kernel rejects `MADV_POPULATE_READ` with
  `EINVAL`, and `mmap` silently ignores unknown flags. The profile only
  simulates which syscalls exist. Argument-level filtering is on the roadmap.
- **Vendor backports:** presets use the mainline base, so a profile can be
  stricter than the real vendor kernel, never looser.
- **Kernels before 3.3:** the data floor is v3.3. Rust std's documented
  minimum, 3.2, is treated as 3.3.
- **Final qualification:** run on a real VM of the target release before you
  ship.

## Refreshing the data (online, occasional)

```bash
scripts/fetch-syscall-tables.sh /tmp/raw
scripts/build-syscall-data.sh /tmp/raw data/linux/syscalls-x86_64.tsv
cargo test && target/release/kernel-seccomp generate profiles
```

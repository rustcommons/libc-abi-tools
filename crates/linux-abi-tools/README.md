# linux-abi-tools

[![docs.rs](https://img.shields.io/docsrs/linux-abi-tools)](https://docs.rs/linux-abi-tools)

One dependency for building and testing Rust programs for older Linux
systems. It re-exports four crates from the
[linux-abi-tools](https://github.com/rustcommons/linux-abi-tools) workspace:

| Module | Crate | Purpose |
| --- | --- | --- |
| `linux_abi_tools::targets` | [`linux-targets`](https://crates.io/crates/linux-targets) | Distribution presets (glibc floor, kernel floor, image) and target selection |
| `linux_abi_tools::libc` | [`libc-abi-tools`](https://crates.io/crates/libc-abi-tools) | glibc ABI metadata, stub generation, and providers |
| `linux_abi_tools::kernel` | [`kernel-abi-tools`](https://crates.io/crates/kernel-abi-tools) | Kernel syscall data and seccomp simulation, on the host or in a container |
| `linux_abi_tools::scan` | [`linux-abi-scan`](https://crates.io/crates/linux-abi-scan) | Static inspection of built executables against the glibc and kernel floors |

Depend on an individual crate when you need only one part. The command-line
tools (`libc-abi`, `kernel-seccomp`, `syscall-probe`, `linux-abi-scan`) are installed from their
own crates, e.g. `cargo install kernel-abi-tools`. The companion Cargo
subcommand is [cargo-libc](https://github.com/rustcommons/cargo-libc).

See [WORKFLOWS.md](https://github.com/rustcommons/linux-abi-tools/blob/main/docs/WORKFLOWS.md)
for how the pieces fit together.

This crate and the four it re-exports are MIT licensed. Through
`libc-abi-tools`' default features it also pulls in
[glibc-abilists](https://crates.io/crates/glibc-abilists), the GNU C Library's
own ABI lists under LGPL-2.1-or-later, with provenance from glibc's signed
release tags.

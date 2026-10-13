# linux-targets

[![docs.rs](https://img.shields.io/docsrs/linux-targets)](https://docs.rs/linux-targets)

Target selection shared by the rustcommons Linux toolchain. Std-only, no
dependencies.

- **`distros()` / `distro(id)`:** 28 distribution presets from
  [`data/distros.tsv`](data/distros.tsv). Each has a glibc floor, a kernel floor
  (mainline base of the GA kernel), a container image with the real userland,
  and room for vendor kernel backports. Every glibc value was read from its
  image on 2026-10-10.
- **`Selection::resolve`:** turns user options into floors:

| User gives | glibc floor | Kernel floor | Runtime image |
| --- | --- | --- | --- |
| `--distro sles-12-sp5` | 2.22 | 4.12 | the preset's |
| `--glibc 2.22` or `--target x86_64-unknown-linux-gnu.2.22` | 2.22 | none: host kernel, not simulated | none |
| `--kernel 4.12` | host | 4.12 | none |
| `--glibc 2.22 --kernel 4.12` | 2.22 | 4.12 | none |

`--distro` cannot be combined with `--glibc`, `--kernel`, or a versioned
`--target`. A versioned target must agree with `--glibc` when both are given.
`Selection::describe` produces the line a tool should print, for example
`glibc 2.17, kernel not simulated (host)`, so a passing test is never mistaken
for an old-kernel qualification.

Consumers: [`kernel-abi-tools`](../kernel-abi-tools/README.md) today; cargo-libc
once it depends on this workspace.

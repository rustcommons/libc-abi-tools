# Workflows

How the linux-abi-tools crates fit together, which workflow to use for what, and
how each one maps onto [cargo-libc](https://github.com/rustcommons/cargo-libc).
Every row marks what is **implemented and tested** and what is **planned**; a
planned row is a design, not a claim.

Shipping a Rust program to an older Linux system raises three separate questions:

| Question | When it matters | Answered by |
| --- | --- | --- |
| Which glibc symbol versions may the binary require? | Link time | `libc-abi-tools` (ABI data, stub libraries, providers) and `cargo libc build` |
| Which syscalls and `madvise` advice will the kernel accept? | Run time | `kernel-abi-tools` (syscall data, seccomp filter/profile, runners) |
| Which glibc and kernel does "the target" mean? | Both | `linux-targets` (distribution presets and selection rules) |

The `linux-abi-tools` crate re-exports all three (`linux_abi_tools::{libc,
kernel, targets}`).

## 1. Choose a target

One selection drives both floors. The rules are implemented in
`linux_targets::Selection::resolve` and tested:

| User gives | glibc floor | Kernel floor | Runtime image |
| --- | --- | --- | --- |
| `--distro sles-12-sp5` | 2.22 | 4.12 (plus sourced backports) | the preset's |
| `--glibc 2.22` or `--target x86_64-unknown-linux-gnu.2.22` | 2.22 | none: host kernel, **not simulated** | none |
| `--kernel 4.12` | host | 4.12 | none |
| `--glibc 2.22 --kernel 4.12` | 2.22 | 4.12 | none |

`--distro` cannot be combined with `--glibc`, `--kernel`, or a versioned
`--target`. `kernel-seccomp distros` lists the 28 presets.

## 2. Build against an older glibc

**Status: implemented, experimental** (glibc 2.17, 2.18, 2.22 on x86_64; see
[PROVIDER-CONTRACT.md](PROVIDER-CONTRACT.md)).

1. Produce a provider: generated link-only stub libraries plus the real startup
   objects, `libc_nonshared.a`, headers, and compiler runtime, described by a
   `libc-abi-provider-v1` descriptor.
2. Build: `cargo libc build --target x86_64-unknown-linux-gnu.2.22 --provider
   provider.txt --artifact ... -- --release`.

The stubs are used only for linking and never deployed. The provider is still
supplied by the user; how released providers are sourced, authenticated, and
distributed is an open decision (see [ROADMAP.md](ROADMAP.md)).

## 3. Inspect a built binary

| Check | Status |
| --- | --- |
| `GLIBC_*` version requirements against the glibc floor (`cargo libc inspect --max-glibc`) | Implemented |
| `NT_GNU_ABI_TAG` above the kernel floor | Planned |
| `DT_RELR` with a glibc floor older than 2.36 | Planned |
| `libc::syscall`/`SYS_*` use and newer `MADV_*`/`MAP_*` constants in dependencies | Planned |

Inspection proves what the file *requires*; it does not run it.

## 4. Run under an older kernel

Both runners make the host kernel answer like the selected kernel:
syscalls it lacks return `ENOSYS`, `madvise` advice it lacks returns `EINVAL`,
and everything else behaves as on the host. Both use a **seccomp filter**, which
is a classic-BPF program. They differ in who builds the filter and which
userland the program sees.

| | Local filter (`--runner local`) | Container (`--runner container`) |
| --- | --- | --- |
| Status | Implemented; all 47 profiles verified on a 6.18 kernel | Implemented; generic kernels and three presets verified with crun in CI |
| Kernel simulated | Yes | Yes |
| Userland (glibc, `/etc`, tools) | **The host's** | The image's (e.g. real SLES 12 SP5) |
| Filter built by | This crate: BPF from compiled-in data | The runtime's libseccomp, from the generated OCI JSON profile |
| Needs | Linux x86_64 host with seccomp filters (3.5+) | podman or docker; crun with libseccomp 2.6+ to name the newest syscalls |
| Root | **No** (sets `no_new_privs`, then installs the filter) | No with rootless podman; docker needs access to its daemon |
| Catches | Code that needs a newer kernel | The same, plus the binary actually loading and running on the old glibc |
| Cost | Process start only | Image pull and container start |

Choose the local runner for fast tests of kernel behavior and when no container
runtime is available. Choose the container when you need the real target
userland, which is the only way to *run* (not just link-check) against the old
glibc.

### Default runner

`kernel_abi_tools::Runner::choose` applies these rules (tested):

| Options | Runner |
| --- | --- |
| `--runner local` or `--runner container` | As requested |
| `--image IMAGE`, or `--distro ID` (the preset's image) | Container |
| `--kernel X.Y` alone | Local |
| `--runner local --image IMAGE` | Error: the local runner cannot use an image |
| `--runner container --kernel X.Y` without an image | Error: the container needs one |

`--runner local --distro ID` is allowed. It simulates the preset's kernel,
including backports, and reports that the userland is the host's.

### Commands

```sh
# Local filter: host userland, no container runtime, no root.
kernel-seccomp run --kernel 4.12 -- ./target/release/app
kernel-seccomp run --runner local --distro sles-12-sp5 -- ./target/release/app

# Container: the preset's real userland under the equivalent profile.
kernel-seccomp run --distro sles-12-sp5 -- ./target/release/app
kernel-seccomp run --kernel 4.12 --image registry.example/old:tag -- ./app
```

The local runner's filter cannot be removed and is inherited by every child.
Inside it, setuid programs such as `sudo` cannot gain privileges.

### Through Cargo

Cargo's runner setting applies the simulation to `cargo run`, `cargo test`, and
`cargo bench` target executables. Build scripts and procedural macros are not
affected. Tested with the local runner:

```sh
export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER="kernel-seccomp run --kernel 3.10 --"
cargo test
```

### Why not eBPF

seccomp's classic BPF is the right tool: an unprivileged process can install
it. eBPF programs (the `bpf()` syscall, tracing, BPF-LSM) need `CAP_BPF` or
`CAP_SYS_ADMIN`, so effectively root, and add nothing needed to answer
`ENOSYS`/`EINVAL`.

## 5. Use the libraries directly

| Need | API |
| --- | --- |
| Resolve user options to floors | `linux_targets::Selection::resolve`, `Selection::describe` |
| What a target lacks | `kernel_abi_tools::missing_syscalls`, `madvise_accepts` |
| OCI profile for a container runtime | `kernel_abi_tools::seccomp_profile` |
| BPF program for a target | `kernel_abi_tools::filter::program` (with `filter::evaluate` for tests) |
| Pick a runner | `kernel_abi_tools::Runner::choose` |
| Run a program | `kernel_abi_tools::local::run`, `kernel_abi_tools::container::run` |

All data is compiled in, so no network or data files are needed at run time.

## 6. How this translates into cargo-libc

cargo-libc depends on the published crates: either `linux-abi-tools` or the
individual crates. crates.io does not accept git dependencies, so a published
cargo-libc cannot pin a commit of this repository. Offline builds use
`cargo vendor`.

| cargo-libc | Calls | Status |
| --- | --- | --- |
| `--distro`, `--glibc`, `--kernel`, `--target triple.X` on every command | `Selection::resolve`; print `Selection::describe` so "kernel not simulated" is visible | `--glibc` and versioned `--target` implemented; `--distro`/`--kernel` planned |
| `cargo libc build` | Provider for the glibc floor | Implemented (provider supplied by the user) |
| `cargo libc check` (F03) | Same selection and target configuration, no link | Next coordinator task |
| `cargo libc inspect` | Kernel and `DT_RELR` checks against both floors | glibc check implemented; kernel checks planned |
| `cargo libc test` / `run` with `--runner local\|container` | Sets Cargo's target runner to a cargo-libc helper binary. That helper calls `Runner::choose`, then `local::run` or `container::run` | Planned; the library side is implemented |
| `cargo libc doctor` | Reports runner availability: seccomp support, container engine, crun/libseccomp version | Planned |

A typical flow once implemented:

```sh
cargo libc build --distro sles-12-sp5 --provider provider.txt --artifact ... -- --release
cargo libc inspect target/.../app --distro sles-12-sp5          # glibc + kernel floors
cargo libc test  --distro sles-12-sp5 --runner local            # fast: kernel only
cargo libc test  --distro sles-12-sp5                           # container: real userland
```

The runner helper follows the existing `libc-linker` pattern: a sibling
executable installed with `cargo-libc` that Cargo invokes, so Cargo's own
process and the build scripts never run under the filter.

### Release order

1. Publish `linux-targets`, then `kernel-abi-tools`, then `libc-abi-tools`,
   then `linux-abi-tools`. Each depends only on crates published before it.
   `cargo package --workspace` verifies the set together before anything is
   uploaded.
2. Point cargo-libc at the published versions, implement the rows above, and
   qualify them in cargo-libc's CI.

## 7. Open decisions

- **glibc data for crates.io users.** The published `libc-abi-tools` crate
  leaves out `data/glibc` (GPL-2.0 ABI lists, see [NOTICE.md](../NOTICE.md)) and
  reads data from a checkout at run time. Generating a provider from a crates.io
  install therefore needs that data from elsewhere. The options are a separate
  GPL-2.0 data crate, released providers, or a documented checkout. This is
  tied to the release-sourcing decision in [ROADMAP.md](ROADMAP.md).
- **Distribution backports** remain unsourced for most presets; until sourced, a
  preset simulates its mainline base kernel.

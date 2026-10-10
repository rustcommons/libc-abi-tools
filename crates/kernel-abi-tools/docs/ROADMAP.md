# Roadmap

Plan established 2026-10-10. Initial scope: x86_64 Linux, kernels 3.10 and
later (Rust std's floor is 3.2; the data floor is 3.3).

## Generic kernels vs. distribution presets

**Generic kernel profiles are the core and are sufficient for syscall
availability.** A distribution profile is exactly the generic profile of its
kernel's mainline base. `sles-12-sp5.json` and `linux-4.12.json` allow the
same 333 syscalls; only their comment differs.

Distribution presets are still worth keeping, as a thin layer:

1. **One name selects everything.** `--distro sles-12-sp5` picks the kernel
   floor (4.12), the glibc floor (2.22, for cargo-libc) and the userland image.
2. **Backports.** Vendor kernels add syscalls beyond their mainline base. A
   preset can list them so its profile is not stricter than reality.
3. **Correct userland.** Runtime tests need the target's glibc, not just its
   kernel.

Users who only care about the kernel can pass `--kernel X.Y` and ignore the
presets.

## Distribution presets

Kernel = mainline base of the GA kernel. glibc was read from each image on
2026-10-10.

| Preset | Kernel | glibc | Status |
| --- | --- | --- | --- |
| sles-12-sp5 | 4.12 | 2.22 | ✅ profile verified, app runs in image |
| sles-15-sp1 | 4.12 | 2.26 | preset |
| sles-15-sp2 / sp3 | 5.3 | 2.26 / 2.31 | preset |
| sles-15-sp4 / sp5 | 5.14 | 2.31 | preset |
| sles-15-sp6 / sp7 | 6.4 | 2.38 | preset |
| sles-16.0 | 6.12 | 2.40 | preset |
| rhel-7, centos-7 | 3.10 | 2.17 | preset |
| rhel-8 | 4.18 | 2.28 | preset |
| rhel-9 | 5.14 | 2.34 | preset |
| rhel-10 | 6.12 | 2.39 | preset |
| ubuntu-14.04 / 16.04 / 18.04 | 3.13 / 4.4 / 4.15 | 2.19 / 2.23 / 2.27 | preset |
| ubuntu-20.04 / 22.04 / 24.04 | 5.4 / 5.15 / 6.8 | 2.31 / 2.35 / 2.39 | preset |
| debian-8 / 9 / 10 | 3.16 / 4.9 / 4.19 | 2.19 / 2.24 / 2.28 | preset |
| debian-11 / 12 / 13 | 5.10 / 6.1 / 6.12 | 2.31 / 2.36 / 2.41 | preset |
| amzn-2 / amzn-2023 | 4.14 / 6.1 | 2.26 / 2.34 | preset |

Candidates: Oracle Linux UEK kernels (UEK6 5.4, UEK7 5.15, which differ from
RHCK), Rocky/Alma (aliases of RHEL), Ubuntu HWE kernels (newer floors).

Generic profiles shipped: 3.10, 3.16, 4.4, 4.9, 4.12, 4.14, 4.15, 4.18, 4.19,
5.3, 5.4, 5.10, 5.14, 5.15, 6.1, 6.4, 6.6, 6.8, 6.12. Any other version can be
generated with `kernel-seccomp profile --kernel X.Y`.

## 1. Prototype (this checkpoint)

- [x] Derive per-syscall introduction data from 81 mainline syscall tables.
- [x] Generate fail-closed allowlist profiles for generic kernels and presets.
- [x] Probe that proves a profile on the actual host/runtime, with a baseline.
- [x] Harness usable directly and as a Cargo runner.
- [x] Verify all 19 generic profiles with crun; record runc limits.
- [x] Run a glibc-2.17-floor memmap2 binary in SLES 12 SP5 userland under 4.12.

## 2. Repository foundation

- [x] Created as `rustcommons/kernel-abi-tools` with CLAUDE.md/AGENTS.md (Git
  conventions from the companions), LICENSE, and line-ending settings.
- [x] CI: fmt/test/clippy on Linux and Windows, committed-profile drift check,
  MSRV 1.90.
- [x] Observe hosted CI passing: all jobs, 18 profiles verified on a 6.17 Azure
  kernel with crun 1.14.1 / libseccomp 2.5.5 (run 38072993763 in the former
  kernel-abi-tools repository).
- [x] NOTICE (data provenance: derived facts, no kernel source; statically
  linked Rust std and musl in release binaries) and README banner.
- [x] CI coverage for every profile, 6.8+ included, using a pinned static crun
  (`scripts/install-crun.sh`) in SLES 12 SP5 userland.
- [x] Runner in Rust: `container::run` in the library; `kernel-seccomp run`
  and the shell script are thin wrappers.
- [x] **Merged into `rustcommons/linux-abi-tools`** (2026-10-10) as
  `crates/kernel-abi-tools`, history kept by subtree merge. Distribution
  presets moved to the new `crates/linux-targets`, which also owns the
  `--distro`/`--glibc`/`--kernel`/`--target` selection rules. Workflows are
  `kernel-abi-tools.yml` and (until the crates.io-only decision, section 5)
  `kernel-abi-tools-release.yml`.
- [ ] An SLES 12 SP5 userland run of an application built with cargo-libc
  (needs cargo-libc changes; deferred).

## 3. Accuracy

- [x] `madvise` advice values, derived from `mman-common.h` at every tag:
  unknown advice returns `EINVAL`, verified for 107 values per profile.
- [ ] Distribution backport data, sourced from vendor kernel changelogs and
  confirmed on real installs (e.g. a SLES 12 SP5 VM: `syscall-probe dump`).
- [ ] Further argument-level rules: `memfd_create`, `openat`/`openat2`,
  `statx`, `clone` flags (EINVAL where the old kernel rejects them). `mmap`
  ignores unknown flags, which seccomp cannot reproduce; document per flag.
- [ ] Extend the probe beyond the 3.3 floor using safe argument sets.
- [ ] Additional architectures (aarch64 first) from their syscall tables.

## 4. Integration with cargo-libc (crate dependency)

Decided 2026-10-10: cargo-libc depends on the **published** crates
(`linux-abi-tools`, or the individual crates); crates.io does not accept git
dependencies. Data is compiled in. Users choose a local seccomp filter or a
container. See [WORKFLOWS.md](../../../docs/WORKFLOWS.md).

- [x] **Shared target selection:** `linux_targets::Selection::resolve`
  implements the agreed rules; a missing kernel floor means host kernel, and
  output says so (`Selection::describe`).
- [x] **Container runner:** `kernel_abi_tools::container::run`.
- [x] **Local runner:** `kernel_abi_tools::local::run` sets `no_new_privs`,
  installs a classic-BPF filter from `filter::program` and executes the
  program. No container and no root. The filter is checked against the data for
  every syscall number and advice value of every profile (unit tests), and all
  47 profiles pass `syscall-probe` on a 6.18 host as an unprivileged user.
- [x] **Runner choice:** `Runner::choose` and `kernel-seccomp run --runner
  local|container`. The default is the container when an image is known,
  otherwise local.
- [ ] **cargo-libc side:** depend on the crates; add `--distro`/`--glibc`/
  `--kernel`/`--target` and `--runner`; a runner helper binary that calls the
  library.
- [ ] **ELF check:** fail when `NT_GNU_ABI_TAG` exceeds the kernel floor, or
  when `DT_RELR` is present for a glibc older than 2.36.
- [ ] **Source scan:** report `libc::syscall`/`SYS_*` use and newer `MADV_*`/`MAP_*`
  constants in vendored dependencies against the kernel floor.
- [ ] **Runtime trace:** optional `strace -f -Z` capture to list every failing
  syscall during a test run (container or local).

## 5. Releases and offline distribution

Decided 2026-10-10: **crates.io only**, as part of the workspace release
([RELEASING.md](../../../docs/RELEASING.md)). The standalone bundle (static
binaries, `kernel-abi-provider-v1` descriptor, `verify-bundle`, the tag
release workflow) was removed before anything was published. The profile
format keeps the `kernel-abi-provider-v1` name.

- [x] Data and profiles compiled in; no network at run time.
- [x] `cargo package --workspace` verifies the four crates together.
- [x] Disconnected-network procedure (`cargo vendor`, `podman save`/`load`,
  pinned static crun) in RELEASING.md.
- [ ] First crates.io publication (needs the maintainer's go-ahead).
- [ ] Publisher authentication beyond crates.io checksums (for example an
  internal mirror trust chain).

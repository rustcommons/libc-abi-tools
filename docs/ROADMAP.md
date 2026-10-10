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

- [x] Create `rustcommons/kernel-abi-tools` with CLAUDE.md/AGENTS.md (Git
  conventions from the companions), LICENSE, and line-ending settings.
- [x] CI: fmt/test/clippy on Linux and Windows, committed-profile drift check,
  MSRV 1.90, and `self-test.sh` under crun for profiles through 6.6 plus
  `sles-12-sp5` (Ubuntu 24.04's libseccomp 2.5.5 cannot name 6.8+ syscalls).
- [x] Observe hosted CI passing: all jobs, 18 profiles verified on a 6.17 Azure
  kernel with crun 1.14.1 / libseccomp 2.5.5 ([run](https://github.com/rustcommons/kernel-abi-tools/actions/runs/38072993763)).
- [ ] NOTICE (kernel syscall tables are GPL-2.0 *source*; we ship derived
  facts, not code — record provenance) and banner to match the companions.
- [ ] CI coverage for 6.8+ profiles (a runner image or container with
  libseccomp 2.6+), and an SLES 12 SP5 userland run built with cargo-libc.
- [ ] Rewrite the shell harness in Rust (`kernel-run`) for Windows/WSL callers
  and structured output; keep the scripts as thin wrappers.

## 3. Accuracy

- [ ] Distribution backport data, sourced from vendor kernel changelogs and
  confirmed on real installs (e.g. a SLES 12 SP5 VM: `syscall-probe dump`).
- [ ] Argument-level rules for flags that old kernels reject or ignore:
  `madvise` advice values, `mmap`/`memfd_create`/`openat`/`statx`/`clone`
  flags. Rules return `EINVAL` where the old kernel would.
- [ ] Extend the probe beyond the 3.3 floor using safe argument sets.
- [ ] Additional architectures (aarch64 first) from their syscall tables.

## 4. Integration with cargo-libc

- [ ] **Shared target profile:** one preset id drives the glibc floor
  (libc-abi-tools data), the kernel floor (this repo's data), and the image.
- [ ] **`cargo libc test` / `cargo libc run`:** set the Cargo runner
  automatically when a preset is selected.
- [ ] **ELF check:** fail when `NT_GNU_ABI_TAG` exceeds the kernel floor, or
  when `DT_RELR` is present for a glibc older than 2.36.
- [ ] **Source scan:** report `libc::syscall`/`SYS_*` use and newer `MADV_*`/`MAP_*`
  constants in vendored dependencies against the kernel floor.
- [ ] **Runtime trace:** optional `strace -f -Z` capture inside the container to
  list every failing syscall during a test run.

## 5. Offline distribution

- [ ] Data and profiles are compiled in; no network at run time (done).
- [ ] Document mirroring the needed images into an internal registry and
  `podman save`/`load` bundles for disconnected networks.
- [ ] Release artifacts: static `kernel-seccomp`/`syscall-probe` binaries and
  a profile tarball, so consumers need neither this source nor Cargo.

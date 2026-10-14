# Roadmap

Open work only. Scope: x86_64 Linux, kernels 3.10 and later (Rust std's floor is 3.2;
the data floor is 3.3). The presets are in
[linux-targets](../../linux-targets/data/distros.tsv); the workspace roadmap is
[docs/ROADMAP.md](../../../docs/ROADMAP.md).

## Accuracy

- [ ] Distribution backport data, sourced from vendor kernel changelogs and
  confirmed on real installs (e.g. a SLES 12 SP5 VM: `syscall-probe dump`).
- [ ] Further argument-level rules: `memfd_create`, `openat`/`openat2`,
  `statx`, `clone` flags (EINVAL where the old kernel rejects them). `mmap`
  ignores unknown flags, which seccomp cannot reproduce; document per flag.
- [ ] Extend the probe beyond the 3.3 floor using safe argument sets.
- [ ] Additional architectures (aarch64 first) from their syscall tables.

## Presets

- [ ] Candidates: Oracle Linux UEK kernels (UEK6 5.4, UEK7 5.15, which differ from
  RHCK), Rocky and AlmaLinux (aliases of RHEL), Ubuntu HWE kernels (newer floors).

## Integration with cargo-libc

- [ ] Run an application built with cargo-libc in SLES 12 SP5 userland with the
  container runner (`cargo libc test --distro sles-12-sp5 --runner container`); CI
  uses only the local runner today.
- [ ] **Source scan:** `MAP_*` and other flag constants, which the binary scan
  does not cover.
- [ ] **Runtime trace:** optional `strace -f -Z` capture to list every failing
  syscall during a test run (container or local).

## Releases

- [ ] Publisher authentication beyond crates.io checksums (for example an
  internal mirror trust chain).

## Known issues

- [ ] `kernel-seccomp missing ... | head` panics with "failed printing to stdout:
  Broken pipe" when the reader closes early (seen 2026-10-13). Exit quietly on
  `BrokenPipe` instead.

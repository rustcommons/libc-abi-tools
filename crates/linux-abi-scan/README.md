# linux-abi-scan

[![docs.rs](https://img.shields.io/docsrs/linux-abi-scan)](https://docs.rs/linux-abi-scan)

Static inspection of built x86_64 Linux executables and shared objects
against a target's glibc and kernel floors. Part of the
[linux-abi-tools](https://github.com/rustcommons/linux-abi-tools) workspace and
called by [cargo-libc](https://github.com/rustcommons/cargo-libc).

```sh
linux-abi-scan target/release/app --distro sles-12-sp5
linux-abi-scan target/release/app --glibc 2.17 --kernel 3.10 --strict
linux-abi-scan target/release/app --distro rhel-7 --tsv      # for tools
```

## Tier 1 of three

| Tier | How | Reliability |
| --- | --- | --- |
| **1. Inspect** (this crate) | Read the ELF file; nothing runs | Exact for symbols, versions, and ELF notes; heuristic for syscalls in code |
| 2. Simulate | Run tests under a seccomp filter or in a container (`kernel-seccomp run`) | Catches what actually executes; syscall availability and `madvise` only |
| 3. Qualify | Run on a real host at the target's minimums (e.g. a SLES 12 SP5 VM) | The only qualification |

A scan is the least reliable tier: it reports what a file *could* do, not
what it *does*. Use it to fail fast; confirm with tiers 2 and 3.

## What it checks

| Check | Confidence | Severity |
| --- | --- | --- |
| `glibc-version`: required `GLIBC_x.y` versions against the glibc floor | Exact | Error above the floor; Warning if the requirement is weak (`VER_FLG_WEAK`) |
| `relr`: `DT_RELR` relocations, which need glibc 2.36 | Exact | Error below 2.36 |
| `glibc-wrapper`: imported wrappers for syscalls (`statx`, `getrandom`, `pidfd_open`, ...) against the kernel floor | Exact | Error if missing; Warning if the symbol is weak (the program must cope with its absence) |
| `abi-tag`: `NT_GNU_ABI_TAG` minimum kernel | Exact | Warning above the floor |
| `syscall-call`: `syscall(N, ...)` calls with a constant `N` | Heuristic | Warning if the kernel lacks it |
| `syscall-insn`: raw `syscall` instructions with a constant number in `eax` | Heuristic | Warning if the kernel lacks it |
| `madvise`: `madvise()` calls with constant advice | Heuristic | Warning if the kernel rejects it |

Exit status: 0 clean, 1 on errors (and on warnings with `--strict`), 2 for
usage or read failures. Informational findings appear with `--verbose`.

Note that a *weak symbol* does not make its *version requirement* weak: a
binary that weakly imports `getrandom@GLIBC_2.25` is still refused by glibc
2.17's loader. The scan reports the two separately.

## What it cannot see

Syscalls made inside the C library, numbers computed at run time, libraries
loaded with `dlopen`, new flags on old syscalls, and whether a program's
fallback works. Rust's standard library falls back for `statx`, `getrandom`,
`copy_file_range`, `pidfd_*`, `clone3`, and `futex_waitv`; findings for these
say so, but the scan cannot tell whether a call came from std.

## Data

Kernel facts come from `kernel-abi-tools` (the same data the seccomp profiles
are built from) and target floors from `linux-targets`, so the tiers cannot
disagree about what a kernel or preset provides. Code scanning recognizes
calls through the GOT and through PLT stubs (including IBT `endbr64`/`bnd`
stubs). The ELF reader is std-only and bounds-checked; files without section
headers are rejected.

Validated 2026-10-10 against synthetic ELF files for every pattern (any OS)
and real binaries on Linux 6.18 (WSL2): a C program with a raw `statx`
syscall, `syscall(SYS_memfd_create)`, and `madvise(MADV_FREE)` scanned against
RHEL 7 reports all three at their addresses.

Licensed under the MIT License.

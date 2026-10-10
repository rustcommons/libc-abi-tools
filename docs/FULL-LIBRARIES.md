# Full glibc 2.17 library experiment

The experimental generator now covers every public numeric GLIBC symbol/version
definition in the eight pinned x86_64 glibc 2.17 lists. Both the prebuilt CentOS
and source-built reference trees produce 2915 definitions across eight link-only
DSOs. Independent audits checked all 5830 definitions, and C/Rust executables
ran with each real glibc 2.17 loader and runtime on the newer WSL kernel.

This is complete symbol coverage for that pinned set, not a released SDK.
GLIBC_PRIVATE and unversioned definitions are outside the abilist contract.
Other libraries, releases, architectures, version-parent graphs, redistribution,
and Cargo integration remain separate work.

## Generation contract

`stub::render_glibc` joins one exact `Snapshot` with one reference DSO. It checks
the library's expected SONAME and public name/version membership, then compares
function kind (including IFUNC) and object/TLS sizes. Duplicate or mismatched
records fail before any output is written. Default versions, binding, visibility,
and section alignment come from the ELF evidence rather than guesses.

`examples/full_glibc_stubs.rs` validates all eight libraries before creating its
new output directory, then writes assembly, version scripts, and selected ELF TSV.
The script assembles them and assigns the reference SONAMEs and linker names.

| Library | SONAME | Public definitions |
| --- | --- | ---: |
| c | libc.so.6 | 2125 |
| m | libm.so.6 | 395 |
| ld | ld-linux-x86-64.so.2 | 9 |
| resolv | libresolv.so.2 | 91 |
| pthread | libpthread.so.0 | 233 |
| dl | libdl.so.2 | 9 |
| rt | librt.so.1 | 47 |
| util | libutil.so.1 | 6 |

Historical array versions require more than equal-size aliases. For example,
sys_errlist versions share a base address but declare 1000, 1008, 1056, and 1080
bytes. The emitter groups overlapping object/TLS ranges, allocates their bounded
union, and preserves each version's size and relative offset. It still rejects
incompatible storage classes and unsupported code overlaps. This preserves
observed byte views without claiming general semantic alias direction.

## Real inputs and Rust linking

The experiment uses these inputs explicitly:

| Input | Source |
| --- | --- |
| glibc link-only DSOs | Generated from the matched ABI/ELF evidence |
| CRT, libc_nonshared.a, libpthread_nonshared.a | Each real target input tree |
| C headers | Each real target input tree |
| libgcc_s.so.1 | Compatible CentOS GCC 4.8.5 runtime, used for both routes |
| crtbegin.o and crtend.o | Host GCC 13, explicitly selected and exercised |
| Rust compiler and static standard library | Installed Linux Rust 1.98.1 SDK |

The source route builds glibc; its compiler runtime is obtained prebuilt. The
wrapper calls GNU ld directly and inserts the target startup/nonshared objects,
generated library search directory, and explicit old compiler runtime.
Host GCC startup objects and Rust SDK archives are recorded in the link manifest.
It is a fixed experimental non-PIE configuration, not a general linker wrapper.

At runtime, the real old loader uses only the corresponding real libc/libraries
and old libgcc_s. No generated library directory is embedded in the executables.
The generated function/IFUNC bodies are traps and the data/TLS backing is
zero-filled; implementations and initial values always come from the real runtime.

## Reproduce

Requires the existing two input trees, x86_64 Linux, Cargo/Rust, GCC, GNU binutils,
and Python 3. With Cargo/Rust on PATH, run from the repository root:

```sh
bash examples/glibc-two-paths/full-and-check.sh /tmp/libc-abi-two-paths-clean
```

The `full-libraries-prebuilt` and `full-libraries-source` directories must not
exist. The retained compiler runtime is reused only when its bytes match the
reference file. Reports are overwritten on rerun; partial writes can leave a new
generation directory for inspection.

The Rust program tests allocation, a spawned thread, thread-local value isolation,
TLS destruction, and caught panic unwinding. Both Rust executables' highest
required numeric GLIBC version is 2.16; runtime qualification remains glibc 2.17.

The C programs test both memcpy versions/stdout copy relocation and legacy/current
sys_errlist array copies. Reserved errno slots may contain null pointers, so the
array test compares all common entries and checks a defined tail error. Real-library
controls also pass. A strong memfd_create reference still fails to link.

`check-full.py` compares every generated export with both the pinned ABI and
selected reference evidence, including defaults, kind, binding, visibility,
sizes, alignment, exact shared ranges, and overlapping data offsets. It checks
Rust version requirements and every shared link/runtime input for host leakage.
Preparation now records reference/generated and compiler runtime hashes in
`logs/abi-manifest.json`. Application checks record executable/source hashes,
persistent linker inputs, and the Rust compiler in `logs/application-manifest.json`.
The checker's combined mode retains the earlier `full-manifest.json` form.
Compiler arguments, link maps, and runtime lists are retained alongside them.
See [the CI split and transfer format](CI-AND-RELEASE-ARTIFACTS.md).

This qualifies the tested programs on these reference files and the newer WSL
kernel. SLES/old-distribution execution, broader standard-library/native dependency
coverage, other compiler runtimes, MSRV qualification, packaging, and the final
provider contract remain open. The next step is the contract and cargo-libc
integration with separate host build-script/procedural-macro behavior.

# Kernel provider contract (`kernel-abi-provider-v1`)

This is how a consumer, such as a future cargo-libc release, finds and uses a
kernel-abi-tools release bundle. It mirrors the style of libc-abi-tools'
`libc-abi-provider-v1` descriptor, with one deliberate difference: **paths are
relative to the descriptor's directory**, so a verified bundle can be unpacked
anywhere, including on a disconnected network, without being rewritten.

## Bundle layout

```text
kernel-abi-tools-0.1.0-x86_64-linux/
├── kernel-abi-provider.txt      descriptor (this contract)
├── SHA256SUMS                   inventory of every other file
├── bin/kernel-seccomp           static x86_64 executable
├── bin/syscall-probe            static x86_64 executable
├── profiles/kernel/linux-X.Y.json
├── profiles/distro/ID.json
├── data/linux/*.tsv, data/distros.tsv
├── scripts/run-under-kernel.sh, scripts/self-test.sh
├── docs/PROVIDER-CONTRACT.md, docs/RELEASE.md
└── LICENSE, NOTICE, README.md
```

## Descriptor

UTF-8, at most 16 KiB, LF line endings, final newline. The first line is
`kernel-abi-provider-v1`; then tab-separated key/value lines. Unknown,
duplicate, missing, or empty fields, control characters, and paths that are
absolute or contain `..` are rejected. Field order is immaterial; the packager
writes this order:

```text
kernel-abi-provider-v1
version<TAB>0.1.0
arch<TAB>x86_64
commit<TAB>9e8d9da...
kernel-seccomp<TAB>bin/kernel-seccomp
syscall-probe<TAB>bin/syscall-probe
profiles<TAB>profiles
distros<TAB>data/distros.tsv
inventory<TAB>SHA256SUMS
```

| Field | Meaning |
| --- | --- |
| `version` | Release version of kernel-abi-tools |
| `arch` | Architecture the data and profiles describe (`x86_64` only in v1) |
| `commit` | Full source commit the bundle was built from |
| `kernel-seccomp` | Profile generator and container runner |
| `syscall-probe` | Profile verifier, run inside the target image |
| `profiles` | Directory of pre-generated profiles |
| `distros` | Distribution presets: `id kernel glibc image backports status` |
| `inventory` | `sha256sum`-format list of every other file in the bundle |

The reference parser is `kernel_abi_tools::bundle::parse_descriptor`;
`kernel-seccomp verify-bundle DIR` performs the full check below.

## Verification

Before first use, a consumer verifies:

1. The archive against its published `.sha256` (transport integrity).
2. `kernel-seccomp verify-bundle DIR`, or the same checks independently: the
   descriptor parses; every named path exists with the right kind; the
   inventory lists every regular file except itself exactly once; every hash
   matches; there are no symlinks or special files.

A SHA-256 checksum is integrity, not publisher authentication. Authenticating
the publisher (signatures or an internal mirror's trust chain) is a separate,
planned step; see the roadmap.

## How a consumer uses a bundle

The intended integration does not need to parse profile JSON or link to this
crate, which keeps a std-only consumer std-only.

| Need | Use |
| --- | --- |
| Run binaries and tests under a target kernel | Set Cargo's runner to `<bundle>/bin/kernel-seccomp run --distro ID --` (or `--kernel X.Y --image IMAGE`) |
| Cross-check a glibc selection | Read the `glibc` column for the preset in `distros` |
| Hand a profile to another tool | `<bundle>/profiles/distro/ID.json` or `profiles/kernel/linux-X.Y.json` |
| Prove the profiles on a host/runtime | `scripts/self-test.sh <bundle>/bin/syscall-probe IMAGE` |

Example for a future `cargo libc test --kernel-provider FILE --distro sles-12-sp5`:

```text
CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER=
  "<bundle>/bin/kernel-seccomp run --distro sles-12-sp5 --"
```

## Compatibility

A new descriptor field or layout change that an existing consumer could
misread gets a new format line (`kernel-abi-provider-v2`). Consumers reject
format lines they do not know; there is no fallback.

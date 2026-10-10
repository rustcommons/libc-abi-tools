# glibc 2.22 provider

The second generated provider targets x86_64 Linux and GNU glibc 2.22. It uses
source-built glibc libraries, headers, CRT, and nonshared archives. This is a
generic GNU runtime environment; it does not establish SLES 12 SP5 compatibility.

Inputs are pinned in [INPUTS.toml](../examples/glibc-2.22/INPUTS.toml):

- GNU `glibc-2.22.tar.xz`, SHA-256
  `eb731406903befef1d8f878a46be75ef862b9056ab0cde1626d08a7a05328948`.
- The same digest-pinned CentOS 7 container and archived, signature-checked
  packages used to bootstrap 2.17, supplying GCC 4.8.5 and Linux 3.10 UAPI headers.
- Configuration: `--prefix=/usr --libdir=/usr/lib64 --disable-werror --enable-kernel=3.2`.
- No source patch: 2.22 already includes the freestanding syscall-header fix.
- A separate real CentOS GCC 4.8.5 unwinder; its required GLIBC versions remain
  compatible with 2.22. Host GCC supplies explicit compiler CRT begin/end objects.

The builder's glibc 2.17 is bootstrap-only. The work tree's `prebuilt` directory
holds the compiler-runtime source file; it is **not a glibc 2.22 target route**.
Only `source` is selected for 2.22 validation. The 2.17 experiment retains both
its prebuilt and source routes.

Build and check on x86_64 Linux:

```bash
export ABI_GLIBC_RELEASE=2.22
bash examples/glibc-2.22/run.sh /tmp/glibc-2.22-work
bash examples/glibc-two-paths/elf-and-check.sh /tmp/glibc-2.22-work
bash examples/glibc-two-paths/full-and-check.sh /tmp/glibc-2.22-work
bash examples/glibc-two-paths/provider-and-check.sh \
  /tmp/glibc-2.22-work ../cargo-libc /usr/bin/clang-24
```

Use a new absolute work directory. Runtime implementations install there,
leaving the host libc in place. The archive checksum pins retrieved bytes over
HTTPS; its upstream signature has not been independently verified.

| Library | Public numeric GLIBC definitions checked |
| --- | ---: |
| c | 2127 |
| m | 398 |
| ld | 9 |
| resolv | 91 |
| pthread | 235 |
| dl | 9 |
| rt | 47 |
| util | 6 |
| Total | 2922 |

The native reader matches GNU readelf for all eight real DSOs. Complete stub
checks cover ABI membership, symbol versions/defaults, SONAMEs, kinds, sizes,
binding/visibility, alignment, exact shared ranges, and historical data views.
Real-loader tests cover C data copy relocations and both memcpy interfaces,
Rust allocation, threads, TLS destruction, and unwinding. Cargo tests preserve
host build scripts/procedural macros and audit shared link/runtime paths.

The release-boundary example explicitly imports both `fmemopen@GLIBC_2.2.5`
and `fmemopen@GLIBC_2.22`; both run under the real 2.22 loader. The 2.17 jobs
must reject the 2.22 import. Both releases reject `memfd_create`, introduced
after their ceilings. Requests for a provider's other release fail before
Cargo runs; the consumer also requires headers to match the selected release.

The existing v1 six-field descriptor remains unchanged. Its `glibc` field now
accepts exactly `2.17`, `2.18` or `2.22`. Existing 2.17 descriptors and the old exporter
remain valid; older consumers correctly reject the newly supported release.
Use the generic exporter for explicit selection:

```bash
cargo run --locked --offline --example provider_glibc -- \
  data/glibc 2.22 /tmp/glibc-2.22-work/source \
  /tmp/glibc-2.22-work/full-libraries-source /usr/lib/gcc/x86_64-linux-gnu/13 \
  /tmp/glibc-2.22-work/compiler-runtime /tmp/glibc-2.22-work/provider.txt
```

Local validation used Rust 1.99.0, Clang 18.1.3, and GNU binutils 2.42. The Cargo
example's highest directly required symbol tag was GLIBC_2.18; the C boundary
example explicitly requires GLIBC_2.22. These measurements do not establish
support for older runtimes. Hosted CI adds Rust 1.99 with Clang 21 and Clang 24
for both release profiles.

Hosted qualification passed on 2026-10-10 for libc-abi-tools commit
`39101a0bdf9d824e3fb89931552b0a771fee5e0e` with cargo-libc commit
`695ac536a791650d0d351c74562e008c47a82392`. All four combinations of
2.17/2.22 and Clang 21/24 passed, including the original two 2.17 input routes,
2.22's source route, fmemopen version boundaries, and Cargo newer-symbol rejection.
The tested compilers were Clang 21.1.8 and Clang 24.0.0 development snapshot
`1:24~++20260917082238+28f36e6d3912-1~exp1~20260917082250.1854`.
Rust 1.99 Windows/Linux, Rust 1.90 all-targets, and all 28 snapshot jobs passed.

[Provider/runtime evidence](https://github.com/rustcommons/linux-abi-tools/actions/runs/38058240922),
[ABI/host checks](https://github.com/rustcommons/linux-abi-tools/actions/runs/38058240932),
and [companion checks](https://github.com/rustcommons/cargo-libc/actions/runs/38058191649)
retain the hosted results and downloadable artifacts.

SLES distribution patches, native dependencies, old-kernel behavior, signed
source verification, authenticated SDK inventories, and redistribution packaging
remain separate qualification work.

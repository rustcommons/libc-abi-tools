# Using glibc providers

A **provider** is everything needed to *link* an x86_64 Linux program for one glibc
release, 2.17 through 2.44, on a newer machine. It contains:

- that release's headers and startup objects (`crt1.o`, `crti.o`, `crtn.o`, the
  nonshared archives);
- link-only stubs of its eight libraries, which expose exactly the symbols and
  versions that release defines;
- a link-only `libgcc_s` stub with GCC 4.8.5's interface;
- your GCC's `crtbegin.o` and `crtend.o`.

A provider is described by a six-line descriptor, `provider.txt`
([PROVIDER-CONTRACT](https://github.com/rustcommons/cargo-libc/blob/main/docs/PROVIDER-CONTRACT.md)).
It is not a runtime: the program runs on the target system's own glibc.

You rarely prepare one by hand. [cargo-libc](https://github.com/rustcommons/cargo-libc)
prepares the provider for the release you select on first use. The commands below
are the same in both tools: `libc-abi provider ...` here, `cargo libc provider ...`
there.

## Choosing a mode

| Mode | What it does | Needs | Time | Use it when |
| --- | --- | --- | --- | --- |
| `source` (default) | Compiles glibc's startup sources, shipped in the tool through the [glibc-provider-sources](https://github.com/rustcommons/glibc-provider) crate, with your GCC. Every object is compared with the qualified container build before the provider is used; a difference refuses it. | GCC, GNU binutils | seconds | Almost always: offline, fast, nothing downloaded. |
| `container` | Runs the pinned container build that CI qualifies: the release's signed source archive, built in a digest-pinned CentOS 7, Debian 9 or Debian 12 image. | podman or docker, network, GCC, binutils, curl | 5–30 minutes per release | Source mode refused your GCC, or you want the objects CI built byte for byte. |
| `prebuilt` | CI-built objects, downloaded. | — | — | Not available: it waits on a legal review. |

Every mode generates the same stubs, `libgcc_s` stub and headers. They differ only
in how the startup objects are made. CI qualifies both available modes for every
release ([PROVIDERS.md](PROVIDERS.md#delivery-modes-per-release)). One mode never
falls back to another: if `source` refuses, nothing is used until you choose
`container`.

**Source mode and your GCC.** GCC 11 through 16 are qualified: hosted CI rebuilds
every release with them and checks the result
([glibc-provider's replay](https://github.com/rustcommons/glibc-provider/blob/main/docs/REPLAY.md)).
Another GCC is still tried, and its objects are verified the same way before use.
Clang cannot be used: it compiles some of glibc's startup code differently
([why](PROVIDER-DELIVERY.md#why-source-mode-compiles-with-gcc-not-clang)). Choose a
GCC with `--cc GCC` (cargo-libc: `CARGO_LIBC_PROVIDER_GCC`).

## Commands

```sh
libc-abi provider prepare --glibc 2.28                     # source mode
libc-abi provider prepare --glibc 2.28 --mode container
libc-abi provider prepare --all                            # every release, source mode
libc-abi provider list
libc-abi provider path --glibc 2.28                        # the descriptor to use
libc-abi provider verify --glibc 2.28                      # recheck files and stubs
libc-abi provider export --glibc 2.28 --to DIR
libc-abi provider import DIR
libc-abi provider remove --glibc 2.28                      # or --all
```

`prepare` prints the descriptor's path on standard output. It reuses a cached
provider with the same inputs, unless you pass `--force`. Container-mode options:
- `--engine PATH` selects the container engine;
- `--archives DIR` uses glibc source archives you already downloaded;
- `--keep-work DIR` keeps the build tree, for inspection.

## The cache

Providers are cached in `$CARGO_LIBC_PROVIDER_DIR`, otherwise in
`$CARGO_HOME/libc-providers`, otherwise in `~/.cargo/libc-providers`. Use
`--cache DIR` to choose another location. Each entry has this path:

```text
x86_64-unknown-linux-gnu/glibc-2.28/source-<hash>/
  provider.txt  manifest.json  libraries.tsv  verification.tsv
  sysroot/  stubs/  compiler-runtime/
```

The hash covers everything the provider depends on:
- the tool version and the release's data;
- for container mode, the image and archive;
- your GCC, binutils and `crtbegin.o`/`crtend.o`.

So an upgraded compiler yields a new entry rather than a stale one. `manifest.json`
records each file's SHA-256, and `verify` checks them all. Delete an entry with
`remove`, or delete the directory itself.

## Offline and air-gapped machines

**Source mode works offline.** The sources are compiled into the tool, so a
machine with the tool, GCC and binutils needs nothing else. Install the tool:
- with `cargo install` from a connected machine;
- or offline, from source vendored with `cargo vendor`;
- or by copying the binaries: cargo-libc's five executables must stay together.

**Container mode needs the network** to pull the image and download the source
archive. On a machine that cannot reach them:

1. Prepare and export the provider on a connected machine:

   ```sh
   cargo libc provider prepare --glibc 2.17 --mode container
   cargo libc provider export --glibc 2.17 --mode container --to /media/usb/providers
   ```

2. Import it on the offline machine. Import checks every file against the
   manifest, and points the provider at that machine's own `crtbegin.o` and
   `crtend.o`:

   ```sh
   cargo libc provider import /media/usb/providers
   ```

3. Use it. If both machines have the same GCC and binutils, automatic selection
   finds it. Otherwise the cache hash differs, so pass the imported descriptor
   explicitly; `import` prints its path, and so does `list`:

   ```sh
   cargo libc build --provider ~/.cargo/libc-providers/x86_64-unknown-linux-gnu/glibc-2.17/container-<hash>/provider.txt \
     --target x86_64-unknown-linux-gnu.2.17 --artifact target/x86_64-unknown-linux-gnu/release/app -- --release
   ```

Without exporting, `--archives DIR` avoids the source download, but the image
still has to be pulled from its registry.

## What a provider does not cover

A provider proves only that your program links against what the selected release
defines. It does not cover:
- the program's kernel needs (see the
  [testing tiers](https://github.com/rustcommons/cargo-libc/blob/main/docs/USER-GUIDE.md#testing-tiers));
- other shared libraries it loads;
- distribution changes to glibc ([DISTRIBUTION-DIFFERENCES.md](DISTRIBUTION-DIFFERENCES.md)).

Executables built with a 2.34 or later provider need glibc 2.34, because from 2.34
`crt1.o` references `__libc_start_main@GLIBC_2.34`.

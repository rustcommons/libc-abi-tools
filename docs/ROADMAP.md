# Roadmap

Open work only. What exists and how it was tested is in
[GLIBC-COVERAGE.md](GLIBC-COVERAGE.md), [VALIDATION.md](VALIDATION.md),
[PROVIDERS.md](PROVIDERS.md) and the [release record](RELEASING.md#release-record).
The kernel crate has its own [roadmap](../crates/kernel-abi-tools/docs/ROADMAP.md).

## Next

- [ ] **Tier 3 on real old systems.** Run representative programs on a real glibc 2.17
  distribution (CentOS 7, Linux 3.10) and on SLES 12 SP5 (glibc 2.22, Linux 4.12):
  threads/TLS, unwinding, C FFI, native C dependencies and rejected newer symbols.
  Every provider is so far tested on real glibc runtimes on a modern kernel and under
  the kernel simulation only. Waits on the maintainer providing or approving the hosts.
- [ ] **Legal review** (maintainer), covering:
  - the corresponding-source arrangement for `prebuilt` (the binaries crate depends on
    `glibc-provider-sources`);
  - the recorded `libgcc_s` interface (`src/libgcc_s-gcc-4.8.5.tsv`);
  - redistribution requirements for generated stubs and their inputs.
- [ ] **Decide** whether the libc side should consume `linux-targets` presets, for
  example to map a distribution to its provider.

## Provider delivery

Designed in [PROVIDER-DELIVERY.md](PROVIDER-DELIVERY.md). The `source` and `container`
modes are delivered; what remains depends on the legal review.

- [ ] Build and publish `glibc-provider-binaries` (the CI-built, qualified objects) from
  a qualified commit.
- [ ] Add the `provider-binaries` feature and `prebuilt` mode, with download and
  authentication of the binaries.
- [ ] Find an imported provider automatically even when the importing machine's GCC
  and binutils differ from the exporting one's (today the cache hash covers them, so
  such a provider must be named with `--provider`).

## glibc data and ABI metadata

- [ ] Establish authoritative binding, visibility, default-version, alias and TLS
  metadata, including general alias semantics.
- [ ] Port upstream's historical migration corrections and inclusion model.
- [ ] Implement bounded reading and writing of Zig's binary abilists format and compare
  the semantic output with the pinned Zig implementation (order is not canonical).
- [ ] Take up new glibc releases (2.45 and later) through glibc-abilists and the
  provider pipeline, with checksums, provenance and reviewed diffs.

## Historical matrix and conformance

- [ ] Classify historical tags, patches, releases, target applicability and evidence
  completeness.
- [ ] Pin and archive hashes and recover missing early source material; record what
  cannot be recovered.
- [ ] Normalize pre-2.17 x86_64 evidence and early target/unversioned history with
  dedicated adapters.
- [ ] Collect native defaults and properties for every supported and historically
  covered profile.
- [ ] Add behavioral, header and feature-change ledgers with primary source references
  and semantic tests.
- [ ] Expand semantic boundary coverage beyond the focused library behavior suites
  (libm, libdl, librt, libutil, libresolv).
- [ ] Inventory additional glibc libraries and modules, and set their scope, before
  expanding the matrix; then extend it to them and to other architectures with explicit
  completeness evidence.
- [ ] Apply the same artifact, runtime and distribution checks to every provider
  qualified later.

## Expand deliberately

- [ ] Other architectures (aarch64 first), each with its own mappings, sizes and
  runtime validation.
- [ ] Other libc families, once their compatibility model and use case are specified.
- [ ] Consider emitting ELF directly once the assembler/linker route is no longer
  needed.
- [ ] Authenticated relocatable provider SDKs for use outside cargo-libc, published only
  for qualified configurations.

## Long-term: source-built builder toolchains

Not planned for the next releases; heavy engineering. Today the `container` mode
and CI build glibc inside vendor images: a digest-pinned CentOS 7 image with
packages from the CentOS vault (2.17–2.25), Debian 9 with archive.debian.org
(2.26–2.36), and Debian 12 with a fixed snapshot.debian.org date (2.37–2.44). Each
builder's compiler, binutils, make, bison, Python, kernel UAPI headers and
`libgcc_s` therefore come from a vendor's binary packages. Those archives can
disappear, and their binaries cannot be audited back to source.

The goal is to build those toolchains ourselves from pinned upstream sources, and to
use them to build every provider.

- **Seed.** Start from a minimal, pinned, auditable seed rather than a full
  distribution. Candidates: a full-source bootstrap such as live-bootstrap or
  GNU Guix's bootstrap, or a small pinned image whose only role is to compile the
  first stage. Record the seed's identity, and its own provenance where available.
- **Era-matched toolchains.** From the seed, build:
  - GNU binutils, GCC (C only, plus libgcc), make, bison, gawk and Python from
    signed upstream releases;
  - Linux UAPI headers from kernel.org tags.

  The versions must be equivalent to what each glibc era needs today: GCC 4.8-era
  for 2.17–2.25, GCC 6-era for 2.26–2.36, and GCC 12-era for 2.37–2.44 (binutils
  new enough for 2.37's AVX-512 code).
- **Images from Containerfiles.** Produce each builder image from a committed
  Containerfile and recorded inputs. Publish the image digest, the build log and a
  manifest of every input (URL, checksum, signature, license).
- **Reproducibility.** Build every toolchain and provider twice, on different
  hosts, and require identical outputs. Investigate and document every difference.
  Optionally, cross-check the compiler with diverse double-compiling.
- **Equivalence.** For each release, the new builders must yield providers whose
  ABI profiles, stubs and headers match today's vendor-built providers, and they
  must pass every gate (exhaustive links, applications, Cargo, hosted CI).
  Explain and record any difference, for example in startup-object code.
- **Our own `libgcc_s`.** The compiler runtime in each provider then also comes from
  our GCC build, with its source and license recorded.
- **Costs to expect:**
  - toolchain builds take hours, so they need caching and separate CI workflows;
  - old GCC and binutils need fixes to build with newer seeds;
  - a matrix of toolchain eras to maintain;
  - substantial CI storage.
- **Exit criterion.** No provider, CI job or `container` mode depends on a
  vendor image or archive. The vendor-built path stays only as a comparison until
  then.

## Future release: distribution-specific providers

Planned for a later release. Every provider is a generic GNU build, and
[DISTRIBUTION-DIFFERENCES.md](DISTRIBUTION-DIFFERENCES.md) records how that can differ
from a distribution's glibc (measured for CentOS 7.9 and 2.17).

- [ ] Choose the first inputs among redistributable sources (Debian, Ubuntu, Rocky
  Linux, AlmaLinux, Red Hat UBI, SUSE BCI) and name them after their actual source.
- [ ] Pin each binary and source package by version and checksum, verify the
  distribution's signatures, and record each package's license and redistribution terms.
- [ ] Extend the descriptor with the input identity and keep generic and distribution
  providers for the same release distinguishable; document any schema change first.
- [ ] Measure each input as a native profile and compare it with the generic build;
  record every difference in DISTRIBUTION-DIFFERENCES.md.
- [ ] Apply the same ABI, exhaustive, application, Cargo and hosted gates as the generic
  providers; run the results in the distribution's own image.
- [ ] Before any SDK redistribution, settle source availability (distribution source
  packages), license notices, and trademark naming.

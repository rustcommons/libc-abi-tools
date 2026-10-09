# Roadmap

Plan established 2026-10-09. Start with x86_64 Linux and glibc target releases 2.17+.

## 1. Repository foundation and metadata port

- [x] Commit CLAUDE.md/AGENTS.md and imported Git conventions first.
- [x] Pin Zig libc-abi-tools and preserve source/data attribution and licenses.
- [x] Import only the eight selected x86_64 library lists for snapshots 2.17 onward.
- [x] Parse both historical grouped and modern flat ABI lists.
- [x] Preserve library, symbol version, function/object kind, and size.
- [x] Export deterministic exact-release snapshots and symbol queries.
- [x] Collect selected files from clean, exactly tagged glibc source checkouts.
- [x] Test all 28 pinned snapshots, the 2.17 floor, and library migrations.
- [x] Add README artwork and CI configuration.

## 2. Cross-release consolidation

- [ ] Port upstream's historical migration corrections and inclusion model.
- [ ] Specify a versioned data contract for cargo-libc.
- [ ] Implement bounded Zig abilists binary reading/writing and compare semantic
  output against the pinned Zig implementation; do not assume order is canonical.
- [ ] Establish authoritative binding, visibility, default-version, alias, and TLS metadata.
- [ ] Automate upstream refresh with checksums, provenance, and reviewed diffs.

## 3. Generate and validate link environments

- [ ] Emit x86_64 assembly and version scripts, then assemble/link stub DSOs.
- [ ] Verify exported versions, SONAMEs, object sizes, and symbol resolution.
- [ ] Source/build compatible CRT, libc_nonshared.a, headers, and compiler runtimes.
- [ ] Integrate with cargo-libc behind an explicit provider selection.
- [ ] Test real binaries on glibc 2.17 and SLES 12 SP5/glibc 2.22, including
  threads/TLS, unwinding, native dependencies, and rejected newer symbols.
- [ ] Establish redistribution requirements for generated artifacts and their inputs.

## 4. Expand deliberately

- [ ] Add architectures with their own mappings, sizes, and runtime validation.
- [ ] Add other libc families when their compatibility model and use case are specified.
- [ ] Consider direct ELF emission after the assembler/linker route is validated.

## Repository presentation and release work

| Field | Prepared value |
| --- | --- |
| Tagline | Know the ABI. Preserve the details. |
| Description | Rust tools for collecting and inspecting libc ABI metadata, starting with glibc. |
| Topics | rust, libc, glibc, abi, elf, linux, toolchain, cross-compilation, zig |
| README image | docs/assets/banner.svg; original editable gear and ABI diagram |
| Badges | CI, Rust 1.90+, MIT code, separate upstream data license, prototype status |

- [ ] Apply description/topics to GitHub settings; describe generation as planned.
- [ ] Observe hosted CI and the MSRV job after the first push.
- [ ] Decide packaging and crate publication policy; verify crate-name availability.
- [ ] Publish only after the advertised milestone passes its validation gates.

Live repository settings, pushes, and releases are not part of this local bootstrap.

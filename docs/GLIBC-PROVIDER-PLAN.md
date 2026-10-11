# Establishing additional glibc providers

This is an implementation order, not a list of newly supported runtimes.
2.17, 2.18, 2.19, 2.20 and 2.22 have locally and hosted-qualified provider integration. All 28
vendored labels have snapshot tests; that does not establish real providers.
The complete per-label status remains in [ROADMAP.md](ROADMAP.md).

## Delivery order

| Stage | Versions | Purpose and completion gate |
| --- | --- | --- |
| 1 | 2.17, 2.22 | Establish the shared exhaustive relocation suite and measured default-binding checks before expanding providers. Preserve both 2.17 input routes. |
| 2 | 2.18, 2.19, 2.20, 2.21 | Fill the gap between established providers, one version at a time. Qualify the common pre-merger layout and historical builder for each source tree. |
| 3 | 2.23, 2.24, 2.25, 2.26, 2.27 | Continue in order. Use matrix-derived new-interface boundaries rather than a fixed negative symbol: memfd_create becomes accepted at 2.27. |
| 4 | 2.28, 2.29, 2.30, 2.31, 2.32, 2.33 | Complete the pre-merger series. Review each release's startup, nonshared, header, compiler-runtime, and build requirements individually. |
| 5 | 2.34 | Implement and qualify merged-library handling before accepting later layouts. Test direct per-library exports separately from whole-program resolution. |
| 6 | 2.35, 2.36, 2.37, 2.38, 2.39, 2.40, 2.41, 2.42 | Extend the qualified merged-library implementation sequentially; retain a dedicated job for every established version. |
| 7 | 2.43, 2.44 | Verify actual upstream release/source availability first. A vendored label alone cannot authorize source or runtime claims. Then apply the same gates. |

Versions within a stage are separate deliverables. Advance only after recording
the preceding version's evidence; failures must not be hidden by allowing CI
jobs to fail. Source-building and distribution qualification are separate routes.

## Required steps for each version

1. **Pin real inputs.** Record official source location, immutable archive hash,
   release identity, license, builder image digest, packages, configuration,
   kernel headers, and every patch with its reason. Choose a compiler that can
   build that historical glibc; Rust 1.99 and Clang 21/24 remain application tools.
   Do not assume the 2.17/2.22 builder works for every release.
2. **Produce a complete target tree.** Collect matching headers, loader, runtime
   DSOs, CRT objects, nonshared archives where applicable, GCC startup objects,
   and a real compatible compiler runtime. Discover and document layout changes
   rather than copying the current required-file list blindly.
3. **Reconcile ABI evidence.** Compare all covered real exports with the pinned
   matrix using the native reader and independent GNU readelf normalization.
   Investigate discrepancies; never edit an expectation merely to make CI pass.
   Record native defaults, compatibility definitions, types, binding, visibility,
   sizes, alignment, and shared ranges with input hashes. Regenerate user tables.
4. **Generate and audit stubs.** Preserve exact library membership and measured
   defaults. Run strong function/object/TLS relocation probes for every union
   interface against its isolated library: present must link, absent must fail
   for that interface. Check every measured ordinary default separately. Keep
   unknown evidence distinct from rejection. Never execute generated stubs.
5. **Extend the provider and consumer together.** Add explicit release selection
   in both repositories, release-aware layout validation, descriptor round trips,
   and mismatched-selection rejection. For 2.34+, revise the artifact contract if
   its eight-library/nonshared assumptions cannot describe the real inputs;
   document any schema migration before enabling the version.
6. **Qualify complete Cargo links.** Use Rust 1.99 with Clang 21 and 24, explicit
   target flags, host build scripts/proc macros, and target CRT/nonshared/runtime
   inputs. Test allocation, threads, TLS destructors, unwinding, historical data
   views, native C dependencies, and version boundaries selected from the matrix.
   Audit actual imports, dependency closure, link maps, and runtime paths for
   host leakage. Isolated-library rejection is not whole-program rejection.
7. **Run real implementations.** Execute with the matching real loader and DSOs,
   retain runtime dependency listings, and separately qualify intended distro
   packages and kernels. A source runtime on a modern host does not establish
   support for an old distribution or kernel. Keep semantic tests limited to
   interfaces with correct prototypes and explicit behavior expectations.
8. **Add permanent hosted gates.** Add that version to GitHub Actions under both
   Clang jobs, with immutable source inputs and the tested consumer revision.
   Retain matrix results, tool revisions, input/artifact hashes, diagnostics,
   descriptors, and runtime evidence. Mark the roadmap complete only after the
   hosted jobs pass. Authenticated relocatable SDK publication is a later gate.

## Historical research runs alongside provider expansion

Start pre-2.17 normalization with 2.16 and move backward through applicable
x86_64 evidence. Keep the provider support floor at 2.17. For earlier releases,
classify source tags versus releases, pin archive hashes, reconstruct patch
chains where possible, and record missing material. Establish architecture
applicability before importing symbols: x86_64 cannot represent glibc's entire
early history. Add separate target and unversioned-interface adapters where
necessary. Unknown or incomplete collections cannot produce rejection verdicts.

Symbol matrices document exported interfaces; they do not capture every header,
feature, behavioral, or security change. Maintain source-backed change ledgers
and focused semantic tests alongside the ABI tables as that evidence is collected.

## Immediate deliverables

- Qualify the new exhaustive suite locally and in the four current provider jobs.
- Establish 2.21 after the qualified [2.20 provider](GLIBC-2.20.md).
- Collect and normalize 2.16 history without declaring a 2.16 Rust provider.
- Keep distribution qualification for current 2.17/2.22 inputs tracked separately.

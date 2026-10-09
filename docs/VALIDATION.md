# Validation — bootstrap milestone

Validated 2026-10-09 with Rust 1.99.0 on Windows and Rust 1.98.1 in Ubuntu WSL.

| Check | Result |
| --- | --- |
| `cargo test --locked --offline` | 10 tests passed on Windows and Linux |
| `cargo fmt --all -- --check` | Passed |
| `cargo clippy --all-targets --locked --offline -- -D warnings` | Passed on Windows and Linux |
| Pinned source-data integrity | All 224 SHA-256 checksums match |
| Release coverage | Every snapshot from 2.17 through 2.44 parses all eight selected libraries |
| Historical versus modern format | Equivalent source rows produce identical records |
| 2.17 baseline | Preserves both memcpy versions and GLIBC_2.2.5 startup symbols |
| Library migration | pthread_sigmask membership differs correctly across 2.31/2.32 |
| Startup version change | GLIBC_2.34 __libc_start_main absent on 2.22, present on 2.34 |
| Collector | Fixture Git checkout verifies exact tags, clean tree, byte preservation, and overwrite refusal |
| CLI | Real 2.22 symbol query succeeds; ambiguous/unknown options fail |

Collector tests use a temporary local Git fixture. Collection from a full official
glibc checkout has not been independently exercised. The vendored data comes
from the pinned Zig tree, not a new glibc build.

Rust 1.90 is the declared minimum; an MSRV check is configured in CI but was not
run locally. Hosted CI has not run before the first push. There are no generated
DSOs or runtime qualification results. Passing metadata tests does not establish
that an executable runs on SLES or glibc 2.17.

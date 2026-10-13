//! Release-specific libc ABI metadata. Currently supports x86_64 Linux glibc.
//!
//! Adapted from Zig's libc ABI tools; see NOTICE.md and LICENSES/Zig-MIT.txt.
//! Collects metadata and emits experimental link-only assembly sources.
//! It supplies no libc runtime implementation; see the qualified example scope.
//!
//! # What it does
//!
//! - **ABI metadata** ([`glibc`], [`matrix`]): every versioned symbol that each
//!   glibc release from 2.17 through 2.44 exports from its eight public libraries
//!   (`libc`, `libm`, `ld`, ...), read from glibc's own ABI lists. A [`Matrix`]
//!   answers whether a link against a release can use `name@GLIBC_x.y`.
//! - **Link-only stubs** ([`stub`], [`elf`], [`libgcc`]): assembly and version
//!   scripts for shared libraries that define exactly a release's symbols and
//!   versions, with no code. Linking against them fails on anything newer than
//!   the release.
//! - **Providers** ([`provider`]): everything a linker needs to build for one
//!   release (headers, startup objects, nonshared archives, stubs), prepared and
//!   cached locally in `source` or `container` mode, and described by a
//!   descriptor that cargo-libc reads.
//!
//! The `libc-abi` binary exposes these as commands (`libc-abi glibc ...`,
//! `libc-abi libgcc stub`, `libc-abi provider ...`); run `libc-abi --help`.
//!
//! # Features
//!
//! Both are on by default.
//!
//! - **`bundled-glibc-data`** compiles in the GNU C Library's ABI lists from the
//!   `glibc-abilists` crate (LGPL-2.1-or-later), taken from glibc's signed release
//!   tags. Without it, the data is read from a directory instead
//!   ([`glibc::Snapshot::load`], `--data DIR`).
//! - **`provider-sources`** compiles in the recorded glibc sources of the
//!   `glibc-provider-sources` crate (LGPL-2.1-or-later, and the Linux UAPI
//!   headers' licenses), which `source`-mode providers compile with the host's
//!   GCC. Without it, only `container` mode prepares providers.
//!
//! This crate's own code is MIT licensed.
//!
//! # Example
//!
//! Whether a program linked for glibc 2.17 or 2.28 may import `memfd_create`,
//! which glibc added in 2.27:
//!
//! ```
//! # #[cfg(feature = "bundled-glibc-data")]
//! # fn main() -> Result<(), String> {
//! use libc_abi_tools::glibc::{Kind, Snapshot};
//! use libc_abi_tools::matrix::{Expectation, Matrix};
//! use libc_abi_tools::version::Version;
//!
//! let v = |text: &str| text.parse::<Version>();
//! let matrix = Matrix::from_snapshots([
//!     Snapshot::bundled(v("2.17")?)?,
//!     Snapshot::bundled(v("2.28")?)?,
//! ])?;
//! let memfd = |release| matrix.expect(release, "c", "memfd_create", "2.27".parse().unwrap());
//! assert_eq!(memfd(v("2.17")?), Expectation::Reject);
//! assert_eq!(memfd(v("2.28")?), Expectation::Accept(Kind::Function));
//! // A release outside the matrix is unknown, not rejected.
//! assert_eq!(memfd(v("2.22")?), Expectation::Unknown);
//! # Ok(())
//! # }
//! # #[cfg(not(feature = "bundled-glibc-data"))]
//! # fn main() {}
//! ```
//!
//! [`Matrix`]: matrix::Matrix
#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod elf;
pub mod glibc;
pub mod libgcc;
pub mod matrix;
pub mod provider;
mod sha256;
pub mod stub;
pub mod version;

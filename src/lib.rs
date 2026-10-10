//! Release-specific libc ABI metadata. Currently supports x86_64 Linux glibc.
//!
//! Adapted from Zig's libc ABI tools; see NOTICE.md and LICENSES/Zig-MIT.txt.
//! Collects metadata and emits experimental link-only assembly sources.
//! It supplies no libc runtime implementation; see the qualified example scope.
#![forbid(unsafe_code)]

pub mod elf;
pub mod glibc;
pub mod provider;
pub mod stub;
pub mod version;

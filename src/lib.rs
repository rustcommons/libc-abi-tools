//! Release-specific libc ABI metadata. Currently supports x86_64 Linux glibc.
//!
//! Adapted from Zig's libc ABI tools; see NOTICE.md and LICENSES/Zig-MIT.txt.
//! This library reads metadata; it does not generate runtime or linker libraries.
#![forbid(unsafe_code)]

pub mod glibc;
pub mod version;

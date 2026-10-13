//! One dependency for the rustcommons Linux ABI tools.
//!
//! | Module | Crate | Purpose |
//! | --- | --- | --- |
//! | [`targets`] | `linux-targets` | Distribution presets and `--distro`/`--glibc`/`--kernel`/`--target` selection |
//! | [`libc`] | `libc-abi-tools` | glibc ABI metadata, stub generation, and providers |
//! | [`kernel`] | `kernel-abi-tools` | Kernel syscall data and seccomp simulation (local filter or container) |
//! | [`scan`] | `linux-abi-scan` | Static inspection of built executables against the floors (tier 1) |
//!
//! Each crate can also be used on its own; this crate only re-exports them
//! so that their versions move together.
//!
//! ```
//! use linux_abi_tools::{kernel, targets};
//!
//! // Resolve `--distro sles-12-sp5`, then list what its kernel lacks.
//! let request = targets::Request { distro: Some("sles-12-sp5"), ..Default::default() };
//! let selection = targets::Selection::resolve(&request)?;
//! let sles = selection.distro.clone().expect("a preset was chosen");
//! let missing = kernel::missing_syscalls(sles.kernel, &sles.kernel_backports);
//! assert!(missing.iter().any(|s| s.name == "io_pgetevents")); // Linux 4.18
//! assert_eq!(selection.describe(), format!("glibc 2.22, kernel 4.12 ({})", sles.name));
//! # Ok::<(), String>(())
//! ```

#![warn(missing_docs)]

pub use kernel_abi_tools as kernel;
pub use libc_abi_tools as libc;
pub use linux_abi_scan as scan;
pub use linux_targets as targets;

#[cfg(test)]
mod tests {
    #[test]
    fn crates_share_one_release_type() {
        // kernel-abi-tools re-exports linux-targets' types; a version skew
        // between the two would make these distinct types.
        let k: crate::kernel::KernelVersion = crate::targets::Release::parse("4.12").unwrap();
        assert_eq!(k.to_string(), "4.12");
    }
}

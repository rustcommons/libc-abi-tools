//! Linux kernel ABI data for x86_64 and seccomp profiles that make a modern
//! kernel answer `ENOSYS` for every syscall a selected older kernel lacks.
//!
//! The profiles are a testing aid, not a security boundary: every syscall the
//! simulated kernel has is allowed, and only syscall availability is simulated. Newer flags on old
//! syscalls and behavior changes are not modelled; see the roadmap.

use std::cmp::Ordering;
use std::fmt;

const SYSCALL_DATA: &str = include_str!("../data/linux/syscalls-x86_64.tsv");
const DISTRO_DATA: &str = include_str!("../data/distros.tsv");

/// `ENOSYS` on Linux (all architectures).
pub const ENOSYS: i32 = 38;

/// Kernel releases shipped as generic profiles. Chosen to cover the kernels
/// of the distributions in `data/distros.tsv` plus common LTS lines.
pub const GENERIC_KERNELS: &[&str] = &[
    "3.10", "3.16", "4.4", "4.9", "4.12", "4.14", "4.15", "4.18", "4.19", "5.3", "5.4", "5.10",
    "5.14", "5.15", "6.1", "6.4", "6.6", "6.8", "6.12",
];

/// A mainline kernel release, compared by `major.minor` only.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct KernelVersion {
    pub major: u32,
    pub minor: u32,
}

impl KernelVersion {
    /// Parses `4.12`, `4.12.14`, or a distribution release string such as
    /// `4.12.14-122.37-default`. Only the leading `major.minor` is used.
    pub fn parse(s: &str) -> Result<Self, String> {
        let s = s.trim().trim_start_matches(['v', 'V']);
        let mut parts = s.split(|c: char| !c.is_ascii_digit());
        let major = parts.next().and_then(|p| p.parse().ok());
        let minor = parts.next().and_then(|p| p.parse().ok());
        match (major, minor) {
            (Some(major), Some(minor)) => Ok(KernelVersion { major, minor }),
            _ => Err(format!("not a kernel version: {s:?} (expected e.g. 4.12)")),
        }
    }
}

impl Ord for KernelVersion {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.major, self.minor).cmp(&(other.major, other.minor))
    }
}

impl PartialOrd for KernelVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for KernelVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

/// One x86_64 syscall and the first mainline release whose table lists it.
#[derive(Clone, Debug)]
pub struct Syscall {
    pub nr: u32,
    pub name: &'static str,
    pub first: KernelVersion,
}

/// All x86_64 (64-bit ABI) syscalls, ordered by number.
pub fn syscalls() -> Vec<Syscall> {
    data_rows(SYSCALL_DATA)
        .map(|cols| {
            let [nr, name, first] = cols[..] else {
                panic!("syscall data row needs 3 columns: {cols:?}");
            };
            Syscall {
                nr: nr.parse().expect("syscall number"),
                name,
                first: KernelVersion::parse(first).expect("syscall first_release"),
            }
        })
        .collect()
}

/// The oldest release in the data. Syscalls reported at this release may be
/// older; kernels below it cannot be distinguished from it.
pub fn table_floor() -> KernelVersion {
    syscalls()
        .iter()
        .map(|s| s.first)
        .min()
        .expect("syscall data is empty")
}

/// The newest release in the data.
pub fn table_ceiling() -> KernelVersion {
    syscalls()
        .iter()
        .map(|s| s.first)
        .max()
        .expect("syscall data is empty")
}

/// Syscalls missing from `kernel`, minus any names in `allow` (backports).
pub fn missing_syscalls(kernel: KernelVersion, allow: &[&str]) -> Vec<Syscall> {
    syscalls()
        .into_iter()
        .filter(|s| s.first > kernel && !allow.contains(&s.name))
        .collect()
}

/// A distribution preset: a kernel floor, a glibc floor, and a test image.
#[derive(Clone, Debug)]
pub struct Distro {
    pub id: &'static str,
    pub name: &'static str,
    pub kernel: KernelVersion,
    pub glibc: &'static str,
    pub image: &'static str,
    /// Syscalls the distribution kernel backports beyond its mainline base.
    pub backports: Vec<&'static str>,
    /// How the kernel and glibc values were established.
    pub status: &'static str,
}

/// All distribution presets from `data/distros.tsv`.
pub fn distros() -> Vec<Distro> {
    data_rows(DISTRO_DATA)
        .map(|cols| {
            let [id, name, kernel, glibc, image, backports, status] = cols[..] else {
                panic!("distro data row needs 7 columns: {cols:?}");
            };
            Distro {
                id,
                name,
                kernel: KernelVersion::parse(kernel).expect("distro kernel"),
                glibc,
                image,
                backports: if backports == "-" {
                    Vec::new()
                } else {
                    backports.split(',').collect()
                },
                status,
            }
        })
        .collect()
}

pub fn distro(id: &str) -> Option<Distro> {
    distros().into_iter().find(|d| d.id == id)
}

/// What a profile simulates.
pub struct Target {
    pub label: String,
    pub kernel: KernelVersion,
    pub allow: Vec<&'static str>,
}

impl Target {
    pub fn kernel(kernel: KernelVersion) -> Self {
        Target {
            label: format!("Linux {kernel}"),
            kernel,
            allow: Vec::new(),
        }
    }

    pub fn distro(d: &Distro) -> Self {
        Target {
            label: format!("{} (Linux {})", d.name, d.kernel),
            kernel: d.kernel,
            allow: d.backports.clone(),
        }
    }
}

/// Syscalls `target` provides: those in its mainline release plus backports.
pub fn present_syscalls(target: &Target) -> Vec<Syscall> {
    syscalls()
        .into_iter()
        .filter(|s| s.first <= target.kernel || target.allow.contains(&s.name))
        .collect()
}

/// An OCI/Docker/Podman seccomp profile that allows only the syscalls
/// `target` provides and answers `ENOSYS` for everything else.
///
/// The profile is an allowlist on purpose. Container runtimes resolve names
/// with their own libseccomp and silently drop names it does not know, so a
/// blocklist fails open for exactly the newest syscalls. With an allowlist an
/// unknown name stays blocked (and `syscall-probe` reports it), and syscalls
/// newer than this crate's data are blocked too.
pub fn seccomp_profile(target: &Target) -> String {
    let present = present_syscalls(target);
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str(&format!(
        "  \"_comment\": {},\n",
        json_str(&format!(
            "kernel-abi-tools: simulate {} on x86_64; {} syscalls allowed, all others \
             return ENOSYS. Testing aid only, not a security boundary.",
            target.label,
            present.len()
        ))
    ));
    out.push_str("  \"defaultAction\": \"SCMP_ACT_ERRNO\",\n");
    out.push_str(&format!("  \"defaultErrnoRet\": {ENOSYS},\n"));
    out.push_str("  \"architectures\": [\"SCMP_ARCH_X86_64\"],\n");
    out.push_str("  \"syscalls\": [\n    {\n      \"names\": [\n");
    for (i, s) in present.iter().enumerate() {
        let sep = if i + 1 < present.len() { "," } else { "" };
        out.push_str(&format!("        {}{sep}\n", json_str(s.name)));
    }
    out.push_str("      ],\n      \"action\": \"SCMP_ACT_ALLOW\"\n    }\n  ]\n}\n");
    out
}

fn data_rows(data: &'static str) -> impl Iterator<Item = Vec<&'static str>> {
    data.lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .map(|l| l.split('\t').map(str::trim).collect())
}

fn json_str(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kv(s: &str) -> KernelVersion {
        KernelVersion::parse(s).unwrap()
    }

    #[test]
    fn parses_distribution_release_strings() {
        assert_eq!(kv("4.12.14-122.37-default"), kv("4.12"));
        assert_eq!(kv("v5.14"), kv("5.14.0"));
        assert!(kv("4.9") < kv("4.12"));
        assert!(KernelVersion::parse("linux").is_err());
    }

    #[test]
    fn known_syscall_introductions() {
        let first = |n: &str| syscalls().into_iter().find(|s| s.name == n).unwrap().first;
        assert_eq!(first("getrandom"), kv("3.17"));
        assert_eq!(first("copy_file_range"), kv("4.5"));
        assert_eq!(first("statx"), kv("4.11"));
        assert_eq!(first("clone3"), kv("5.3"));
        assert_eq!(first("close_range"), kv("5.9"));
    }

    #[test]
    fn linux_4_12_profile_boundaries() {
        let names: Vec<_> = missing_syscalls(kv("4.12"), &[])
            .iter()
            .map(|s| s.name)
            .collect();
        assert!(names.contains(&"rseq"));
        assert!(names.contains(&"io_uring_setup"));
        assert!(names.contains(&"clone3"));
        assert!(!names.contains(&"statx"));
        assert!(!names.contains(&"copy_file_range"));
        assert!(!names.contains(&"read"));
    }

    #[test]
    fn backports_are_not_blocked() {
        let names: Vec<_> = missing_syscalls(kv("3.10"), &["getrandom"])
            .iter()
            .map(|s| s.name)
            .collect();
        assert!(!names.contains(&"getrandom"));
        assert!(names.contains(&"memfd_create"));
    }

    #[test]
    fn profile_is_a_fail_closed_allowlist() {
        let p = seccomp_profile(&Target::kernel(kv("4.12")));
        assert!(p.contains("\"defaultAction\": \"SCMP_ACT_ERRNO\""));
        assert!(p.contains("\"defaultErrnoRet\": 38"));
        assert!(p.contains("\"statx\""));
        assert!(!p.contains("\"rseq\""));
        let all = seccomp_profile(&Target::kernel(table_ceiling()));
        assert_eq!(all.matches("\n        \"").count(), syscalls().len());
    }

    #[test]
    fn every_distro_preset_parses() {
        let all = distros();
        assert!(all
            .iter()
            .any(|d| d.id == "sles-12-sp5" && d.kernel == kv("4.12")));
        for d in &all {
            for b in &d.backports {
                assert!(
                    syscalls().iter().any(|s| s.name == *b),
                    "{}: unknown backport {b}",
                    d.id
                );
            }
        }
    }
}

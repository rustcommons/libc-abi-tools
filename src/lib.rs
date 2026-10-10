//! Linux kernel ABI data for x86_64 and seccomp profiles that make a modern
//! kernel answer like a selected older kernel: `ENOSYS` for every syscall it
//! lacks, and `EINVAL` for `madvise` advice values it does not know.
//!
//! The profiles are a testing aid, not a security boundary: everything the
//! simulated kernel has is allowed. Other newer flags on old syscalls and
//! behavior changes are not modelled yet; see the roadmap.

use std::cmp::Ordering;
use std::fmt;

pub mod bundle;
pub mod container;

const SYSCALL_DATA: &str = include_str!("../data/linux/syscalls-x86_64.tsv");
const MADVISE_DATA: &str = include_str!("../data/linux/madvise-x86_64.tsv");
const DISTRO_DATA: &str = include_str!("../data/distros.tsv");

/// `ENOSYS` on Linux (all architectures).
pub const ENOSYS: i32 = 38;
/// `EINVAL` on Linux (all architectures).
pub const EINVAL: i32 = 22;
/// Version of the profile format and provider descriptor this crate emits.
pub const PROVIDER_FORMAT: &str = "kernel-abi-provider-v1";

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

/// The newest release scanned for the data (from its `# Tags scanned:`
/// header), which can be newer than the last release that added a syscall.
pub fn table_ceiling() -> KernelVersion {
    SYSCALL_DATA
        .lines()
        .find_map(|l| l.strip_prefix("# Tags scanned: "))
        .and_then(|range| range.split(" .. ").nth(1))
        .and_then(|last| KernelVersion::parse(last).ok())
        .expect("syscall data header names the scanned tags")
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

/// One `madvise` advice value and the first mainline release defining it.
#[derive(Clone, Debug)]
pub struct MadviseAdvice {
    pub value: u32,
    pub name: &'static str,
    pub first: KernelVersion,
}

/// All `madvise` advice values known to the data, ordered by value.
pub fn madvise_advice() -> Vec<MadviseAdvice> {
    data_rows(MADVISE_DATA)
        .map(|cols| {
            let [value, name, first] = cols[..] else {
                panic!("madvise data row needs 3 columns: {cols:?}");
            };
            MadviseAdvice {
                value: value.parse().expect("madvise value"),
                name,
                first: KernelVersion::parse(first).expect("madvise first_release"),
            }
        })
        .collect()
}

/// Whether `kernel` accepts `madvise` advice `value`. Unknown values are
/// rejected with `EINVAL` by every kernel.
pub fn madvise_accepts(kernel: KernelVersion, value: u32) -> bool {
    madvise_advice()
        .iter()
        .any(|a| a.value == value && a.first <= kernel)
}

/// An OCI/Docker/Podman seccomp profile that allows only the syscalls
/// `target` provides, answers `ENOSYS` for every other syscall, and answers
/// `EINVAL` for `madvise` advice values `target` does not know.
///
/// The profile is an allowlist on purpose. Container runtimes resolve names
/// with their own libseccomp and silently drop names it does not know, so a
/// blocklist fails open for exactly the newest syscalls. With an allowlist an
/// unknown name stays blocked (and `syscall-probe` reports it), and syscalls
/// newer than this crate's data are blocked too.
///
/// `madvise` gets one rule per advice value from 0 to the highest known
/// value, plus one for anything above it, so no two rules overlap and the
/// result never depends on how a runtime orders conflicting rules. Values are
/// compared on the low 32 bits because the kernel reads an `int`.
pub fn seccomp_profile(target: &Target) -> String {
    let present = present_syscalls(target);
    let names: Vec<&str> = present
        .iter()
        .map(|s| s.name)
        .filter(|n| *n != "madvise")
        .collect();
    let max_advice = madvise_advice().iter().map(|a| a.value).max().unwrap_or(0);
    let mut entries = vec![name_list_entry(&names)];
    for value in 0..=max_advice {
        entries.push(madvise_entry(
            "SCMP_CMP_MASKED_EQ",
            u32::MAX as u64,
            Some(value as u64),
            madvise_accepts(target.kernel, value),
        ));
    }
    entries.push(madvise_entry("SCMP_CMP_GT", max_advice as u64, None, false));

    let comment = format!(
        "kernel-abi-tools {PROVIDER_FORMAT}: simulate {} on x86_64; {} syscalls allowed, \
         all others return ENOSYS; madvise advice it does not know returns EINVAL. \
         Testing aid only, not a security boundary.",
        target.label,
        present.len()
    );
    format!(
        "{{\n  \"_comment\": {},\n  \"defaultAction\": \"SCMP_ACT_ERRNO\",\n  \
         \"defaultErrnoRet\": {ENOSYS},\n  \"architectures\": [\"SCMP_ARCH_X86_64\"],\n  \
         \"syscalls\": [\n{}\n  ]\n}}\n",
        json_str(&comment),
        entries.join(",\n")
    )
}

fn name_list_entry(names: &[&str]) -> String {
    let list: Vec<String> = names
        .iter()
        .map(|n| format!("        {}", json_str(n)))
        .collect();
    format!(
        "    {{\n      \"names\": [\n{}\n      ],\n      \"action\": \"SCMP_ACT_ALLOW\"\n    }}",
        list.join(",\n")
    )
}

fn madvise_entry(op: &str, value: u64, value_two: Option<u64>, allow: bool) -> String {
    let action = if allow {
        "\"action\": \"SCMP_ACT_ALLOW\"".to_string()
    } else {
        format!("\"action\": \"SCMP_ACT_ERRNO\", \"errnoRet\": {EINVAL}")
    };
    let two = value_two.map_or(String::new(), |v| format!(", \"valueTwo\": {v}"));
    format!(
        "    {{ \"names\": [\"madvise\"], {action}, \
         \"args\": [{{ \"index\": 2, \"value\": {value}{two}, \"op\": \"{op}\" }}] }}"
    )
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
        assert_eq!(table_floor(), kv("3.3"));
        // The scan reached 7.2 even though the last new syscall is from 7.0.
        assert_eq!(table_ceiling(), kv("7.2"));
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
        // Every syscall but madvise is in the name list; madvise has rules.
        assert_eq!(all.matches("\n        \"").count(), syscalls().len() - 1);
    }

    #[test]
    fn madvise_advice_introductions() {
        let first = |n: &str| {
            madvise_advice()
                .into_iter()
                .find(|a| a.name == n)
                .unwrap()
                .first
        };
        assert_eq!(first("MADV_FREE"), kv("4.5"));
        assert_eq!(first("MADV_WIPEONFORK"), kv("4.14"));
        assert_eq!(first("MADV_POPULATE_READ"), kv("5.14"));
        assert!(madvise_accepts(kv("4.12"), 4)); // MADV_DONTNEED
        assert!(madvise_accepts(kv("4.12"), 8)); // MADV_FREE
        assert!(!madvise_accepts(kv("4.12"), 22)); // MADV_POPULATE_READ
        assert!(!madvise_accepts(table_ceiling(), 5)); // never defined
    }

    #[test]
    fn madvise_rules_cover_every_value_once() {
        let p = seccomp_profile(&Target::kernel(kv("4.12")));
        let max = madvise_advice().iter().map(|a| a.value).max().unwrap();
        let rules = p.matches("\"names\": [\"madvise\"]").count();
        assert_eq!(rules as u32, max + 2); // 0..=max, plus one for > max
        assert!(p.contains(
            "\"action\": \"SCMP_ACT_ERRNO\", \"errnoRet\": 22, \
             \"args\": [{ \"index\": 2, \"value\": 4294967295, \"valueTwo\": 22,"
        ));
        assert!(p.contains(
            "\"action\": \"SCMP_ACT_ALLOW\", \
             \"args\": [{ \"index\": 2, \"value\": 4294967295, \"valueTwo\": 8,"
        ));
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

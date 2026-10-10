//! Static inspection of a built x86_64 Linux executable or shared object
//! against a target's glibc and kernel floors: **tier 1** of the testing
//! tiers.
//!
//! | Tier | How | Reliability |
//! | --- | --- | --- |
//! | 1. Inspect (this crate) | Read the ELF file; nothing runs | Exact for symbols, versions, and ELF notes; heuristic for syscalls in code |
//! | 2. Simulate | Run tests under a seccomp filter or in a container (`kernel-abi-tools`) | Catches what actually executes; syscall availability and `madvise` only |
//! | 3. Qualify | Run on a real host at the target's minimums (e.g. SLES 12 SP5) | The only qualification |
//!
//! A scan reports what the file *could* do, not what it *does*: it cannot see
//! syscalls made inside the C library, numbers computed at run time,
//! libraries loaded with `dlopen`, new flags on old syscalls, or whether a
//! program falls back when a call fails. Findings carry a [`Confidence`].
//!
//! The kernel data is the same that the seccomp profiles are built from
//! (`kernel_abi_tools::syscalls`, `madvise_advice`, and a preset's
//! backports), so tiers 1 and 2 cannot disagree about what a kernel has.

pub mod elf;

use kernel_abi_tools::{
    madvise_accepts, madvise_advice, present_syscalls, syscalls, KernelVersion, Target,
};
use linux_targets::{GlibcVersion, Selection};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::Path;

/// What to check against. `None` skips that side's checks (host glibc or
/// host kernel), and the report says so.
pub struct ScanTarget {
    pub glibc: Option<GlibcVersion>,
    pub kernel: Option<Target>,
    pub label: String,
}

impl ScanTarget {
    /// The floors of a `linux_targets` selection; a preset's kernel includes
    /// its sourced backports.
    pub fn from_selection(sel: &Selection) -> Self {
        let kernel = match (&sel.distro, sel.kernel) {
            (Some(d), _) => Some(Target::distro(d)),
            (None, Some(k)) => Some(Target::kernel(k)),
            (None, None) => None,
        };
        ScanTarget {
            glibc: sel.glibc,
            kernel,
            label: sel.describe(),
        }
    }
}

/// How a finding was established.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Confidence {
    /// Read from ELF metadata the loader itself uses.
    Exact,
    /// Recovered from machine code by pattern matching.
    Heuristic,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// The target does not provide something the file requires.
    Error,
    /// Possibly a problem: optional use, or a heuristic finding.
    Warning,
    /// Compatible, or context.
    Info,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    pub severity: Severity,
    pub confidence: Confidence,
    /// `glibc-version`, `glibc-wrapper`, `syscall-call`, `syscall-insn`,
    /// `madvise`, `abi-tag`, `relr`, or `scope`.
    pub check: &'static str,
    pub subject: String,
    pub detail: String,
    /// Code address of the call site, for code findings.
    pub address: Option<u64>,
}

#[derive(Debug)]
pub struct Report {
    pub target: String,
    pub findings: Vec<Finding>,
}

impl Report {
    pub fn count(&self, severity: Severity) -> usize {
        self.findings
            .iter()
            .filter(|f| f.severity == severity)
            .count()
    }

    /// 0 when nothing fails; 1 when an error (or, with `strict`, a warning)
    /// was found.
    pub fn exit_code(&self, strict: bool) -> i32 {
        let failing = self.count(Severity::Error)
            + if strict {
                self.count(Severity::Warning)
            } else {
                0
            };
        i32::from(failing > 0)
    }

    /// Human-readable report; `verbose` includes informational findings.
    pub fn to_text(&self, verbose: bool) -> String {
        let mut out = format!("target: {}\n", self.target);
        for f in &self.findings {
            if f.severity == Severity::Info && !verbose {
                continue;
            }
            let sev = match f.severity {
                Severity::Error => "ERROR",
                Severity::Warning => "WARN",
                Severity::Info => "info",
            };
            let conf = f.confidence.as_str();
            let at = f.address.map_or(String::new(), |a| format!(" at {a:#x}"));
            writeln!(
                out,
                "{sev:5} {conf:9} {:13} {}{at}: {}",
                f.check, f.subject, f.detail
            )
            .unwrap();
        }
        writeln!(
            out,
            "{} errors, {} warnings, {} info. Static inspection is tier 1: confirm with the \
             seccomp/container simulation (tier 2) and on a real target host (tier 3).",
            self.count(Severity::Error),
            self.count(Severity::Warning),
            self.count(Severity::Info)
        )
        .unwrap();
        out
    }

    /// Stable TSV with a header, for tools.
    pub fn to_tsv(&self) -> String {
        let mut out = String::from("severity\tconfidence\tcheck\tsubject\taddress\tdetail\n");
        for f in &self.findings {
            writeln!(
                out,
                "{}\t{}\t{}\t{}\t{}\t{}",
                f.severity.as_str(),
                f.confidence.as_str(),
                f.check,
                f.subject,
                f.address.map_or(String::new(), |a| format!("{a:#x}")),
                f.detail.replace(['\t', '\n'], " ")
            )
            .unwrap();
        }
        out
    }
}

impl Severity {
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
        }
    }
}

impl Confidence {
    pub fn as_str(self) -> &'static str {
        match self {
            Confidence::Exact => "exact",
            Confidence::Heuristic => "heuristic",
        }
    }
}

/// glibc functions whose syscall has a different name.
const WRAPPER_ALIASES: &[(&str, &str)] = &[
    ("fstatat", "newfstatat"),
    ("fstatat64", "newfstatat"),
    ("posix_fadvise", "fadvise64"),
    ("posix_fadvise64", "fadvise64"),
    ("prlimit", "prlimit64"),
    ("pread", "pread64"),
    ("pwrite", "pwrite64"),
    ("preadv64", "preadv"),
    ("pwritev64", "pwritev"),
    ("preadv64v2", "preadv2"),
    ("pwritev64v2", "pwritev2"),
    ("fallocate64", "fallocate"),
];

/// Syscalls that Rust's standard library calls with a fallback when the
/// kernel answers `ENOSYS`. A finding for one of these is still reported,
/// because the scan cannot tell whether the call came from std.
const RUST_STD_FALLBACKS: &[&str] = &[
    "statx",
    "getrandom",
    "copy_file_range",
    "pidfd_open",
    "pidfd_send_signal",
    "pidfd_getfd",
    "clone3",
    "futex_waitv",
];

/// How far back from a call site to look for the constant argument.
const LOOKBACK: usize = 64;

/// Scans the ELF file at `path`.
pub fn scan_file(path: &Path, target: &ScanTarget) -> Result<Report, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    scan(&bytes, target).map_err(|e| format!("{}: {e}", path.display()))
}

/// Scans an x86_64 ELF64 executable or shared object.
pub fn scan(bytes: &[u8], target: &ScanTarget) -> Result<Report, String> {
    let elf = elf::Elf::parse(bytes)?;
    let mut findings = Vec::new();
    glibc_checks(&elf, target, &mut findings);
    kernel_checks(&elf, target, &mut findings);
    findings.sort_by(|a, b| {
        (a.severity, a.confidence, a.check, a.address, &a.subject).cmp(&(
            b.severity,
            b.confidence,
            b.check,
            b.address,
            &b.subject,
        ))
    });
    Ok(Report {
        target: target.label.clone(),
        findings,
    })
}

fn glibc_version(v: &str) -> Option<GlibcVersion> {
    GlibcVersion::parse(v.strip_prefix("GLIBC_")?).ok()
}

fn glibc_checks(elf: &elf::Elf, target: &ScanTarget, out: &mut Vec<Finding>) {
    let Some(floor) = target.glibc else {
        out.push(scope(
            "glibc",
            "no glibc floor selected; glibc checks skipped (host glibc)",
        ));
        return;
    };
    let mut seen = BTreeSet::new();
    for need in &elf.needs {
        if !seen.insert((need.file.clone(), need.version.clone())) {
            continue;
        }
        let subject = format!("{} {}", need.file, need.version);
        let (severity, detail) = if need.version == "GLIBC_ABI_DT_RELR" {
            continue; // reported by the DT_RELR check
        } else if need.version == "GLIBC_PRIVATE" {
            (
                Severity::Warning,
                "glibc-internal interface; not stable across releases".to_string(),
            )
        } else if let Some(v) = glibc_version(&need.version) {
            if v <= floor {
                (Severity::Info, format!("within the glibc {floor} floor"))
            } else if need.weak {
                (
                    Severity::Warning,
                    format!("newer than glibc {floor}, but a weak requirement"),
                )
            } else {
                (
                    Severity::Error,
                    format!("needs glibc {v}; the target has {floor}"),
                )
            }
        } else {
            continue; // GCC_*, GLIBCXX_*, and other libraries' versions
        };
        out.push(Finding {
            severity,
            confidence: Confidence::Exact,
            check: "glibc-version",
            subject,
            detail,
            address: None,
        });
    }
    if elf.relr {
        let ok = floor
            >= GlibcVersion {
                major: 2,
                minor: 36,
            };
        out.push(Finding {
            severity: if ok { Severity::Info } else { Severity::Error },
            confidence: Confidence::Exact,
            check: "relr",
            subject: "DT_RELR".into(),
            detail: if ok {
                format!("relative relocations in RELR form; supported from glibc 2.36 (floor {floor})")
            } else {
                format!("relative relocations in RELR form need glibc 2.36; the target has {floor} (link without -z pack-relative-relocs)")
            },
            address: None,
        });
    }
}

fn kernel_checks(elf: &elf::Elf, target: &ScanTarget, out: &mut Vec<Finding>) {
    let Some(kernel) = &target.kernel else {
        out.push(scope(
            "kernel",
            "no kernel floor selected; kernel checks skipped (host kernel, not simulated)",
        ));
        return;
    };
    let all = syscalls();
    let by_name: BTreeMap<&str, &kernel_abi_tools::Syscall> =
        all.iter().map(|s| (s.name, s)).collect();
    let by_nr: BTreeMap<u32, &kernel_abi_tools::Syscall> = all.iter().map(|s| (s.nr, s)).collect();
    let present: BTreeSet<u32> = present_syscalls(kernel).iter().map(|s| s.nr).collect();
    let fallback = |name: &str| {
        if RUST_STD_FALLBACKS.contains(&name) {
            "; Rust's standard library falls back when it returns ENOSYS (other code may not)"
        } else {
            ""
        }
    };

    if let Some([a, b, c]) = elf.abi_tag {
        let tag = KernelVersion { major: a, minor: b };
        let (severity, detail) = if tag <= kernel.kernel {
            (
                Severity::Info,
                format!(
                    "declares Linux {a}.{b}.{c} or later; within the {} floor",
                    kernel.kernel
                ),
            )
        } else {
            (
                Severity::Warning,
                format!(
                    "declares Linux {a}.{b}.{c} or later, above the {} floor; glibc's loader refuses shared objects whose tag exceeds the running kernel",
                    kernel.kernel
                ),
            )
        };
        out.push(Finding {
            severity,
            confidence: Confidence::Exact,
            check: "abi-tag",
            subject: "NT_GNU_ABI_TAG".into(),
            detail,
            address: None,
        });
    }

    // Imported glibc wrappers for syscalls.
    for imp in &elf.imports {
        let syscall_name = WRAPPER_ALIASES
            .iter()
            .find(|(f, _)| *f == imp.name)
            .map_or(imp.name.as_str(), |(_, s)| s);
        let Some(sc) = by_name.get(syscall_name) else {
            continue;
        };
        let version = imp
            .version
            .as_deref()
            .map_or(String::new(), |v| format!("@{v}"));
        let subject = format!("{}{version}", imp.name);
        let (severity, detail) = if present.contains(&sc.nr) {
            (
                Severity::Info,
                format!(
                    "syscall {} (Linux {}) is available on {}",
                    sc.name, sc.first, kernel.label
                ),
            )
        } else if imp.weak {
            (
                Severity::Warning,
                format!(
                    "weak import of a wrapper for {} (Linux {}), which {} lacks; the program must handle its absence{}",
                    sc.name, sc.first, kernel.label, fallback(sc.name)
                ),
            )
        } else {
            (
                Severity::Error,
                format!(
                    "wrapper for {} (Linux {}), which {} lacks: calls fail with ENOSYS",
                    sc.name, sc.first, kernel.label
                ),
            )
        };
        out.push(Finding {
            severity,
            confidence: Confidence::Exact,
            check: "glibc-wrapper",
            subject,
            detail,
            address: None,
        });
    }

    // Machine code: calls to syscall()/madvise() and raw syscall instructions.
    let sites = code_sites(elf);
    let mut unresolved = [0usize; 3];
    for site in &sites {
        match site.kind {
            SiteKind::SyscallFn | SiteKind::SyscallInsn => {
                let (check, via, slot) = if site.kind == SiteKind::SyscallFn {
                    ("syscall-call", "syscall()", 0)
                } else {
                    ("syscall-insn", "syscall instruction", 1)
                };
                let Some(nr) = site.constant else {
                    unresolved[slot] += 1;
                    continue;
                };
                let (severity, subject, detail) = match by_nr.get(&(nr as u32)) {
                    Some(sc) if present.contains(&sc.nr) => (
                        Severity::Info,
                        sc.name.to_string(),
                        format!("{via} with number {nr} (Linux {}); available", sc.first),
                    ),
                    Some(sc) => (
                        Severity::Warning,
                        sc.name.to_string(),
                        format!(
                            "{via} with number {nr}: {} (Linux {}) is missing on {}{}",
                            sc.name,
                            sc.first,
                            kernel.label,
                            fallback(sc.name)
                        ),
                    ),
                    None => (
                        Severity::Warning,
                        format!("#{nr}"),
                        format!("{via} with number {nr}, which no kernel in the data defines"),
                    ),
                };
                out.push(Finding {
                    severity,
                    confidence: Confidence::Heuristic,
                    check,
                    subject,
                    detail,
                    address: Some(site.address),
                });
            }
            SiteKind::Madvise => {
                let Some(advice) = site.constant else {
                    unresolved[2] += 1;
                    continue;
                };
                let name = madvise_advice()
                    .into_iter()
                    .find(|a| a.value as u64 == advice)
                    .map_or(format!("advice {advice}"), |a| {
                        format!("{} ({advice})", a.name)
                    });
                if madvise_accepts(kernel.kernel, advice as u32) {
                    continue; // accepted advice is not worth a line
                }
                out.push(Finding {
                    severity: Severity::Warning,
                    confidence: Confidence::Heuristic,
                    check: "madvise",
                    subject: name,
                    detail: format!(
                        "madvise() with advice that {} rejects with EINVAL",
                        kernel.label
                    ),
                    address: Some(site.address),
                });
            }
        }
    }
    for (count, what) in [
        (unresolved[0], "syscall() calls"),
        (unresolved[1], "syscall instructions"),
        (unresolved[2], "madvise() calls"),
    ] {
        if count > 0 {
            out.push(scope(
                "code",
                &format!("{count} {what} take a value not known statically; only tier 2 or 3 can check them"),
            ));
        }
    }
}

fn scope(subject: &str, detail: &str) -> Finding {
    Finding {
        severity: Severity::Info,
        confidence: Confidence::Exact,
        check: "scope",
        subject: subject.into(),
        detail: detail.into(),
        address: None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SiteKind {
    SyscallFn,
    SyscallInsn,
    Madvise,
}

#[derive(Debug)]
struct Site {
    kind: SiteKind,
    address: u64,
    constant: Option<u64>,
}

fn rel32(b: &[u8], at: usize) -> Option<i64> {
    Some(i32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?) as i64)
}

fn imm32(b: &[u8], at: usize) -> Option<u64> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?) as u64)
}

/// Finds the last write of a constant to a register in `window` (the bytes
/// just before a call site). `mov r32, imm32` is `op imm32`; `mov r64,
/// imm32` is `48 c7 modrm imm32`; `xor r32, r32` is `31/33 modrm`.
///
/// A `syscall` or an indirect `call` clobbers rax and the argument
/// registers, so the search starts after the last one in the window: a
/// constant set before it does not belong to this site.
fn last_constant(window: &[u8], mov_op: u8, modrm: u8) -> Option<u64> {
    let start = (0..window.len().saturating_sub(1))
        .rev()
        .find_map(|k| match (window[k], window[k + 1]) {
            (0x0f, 0x05) => Some(k + 2),
            (0xff, 0x15) => Some((k + 6).min(window.len())),
            _ => None,
        })
        .unwrap_or(0);
    let window = &window[start..];
    let mut best: Option<(usize, u64)> = None;
    let mut consider = |end: usize, value: u64| {
        if best.is_none_or(|(e, _)| end > e) {
            best = Some((end, value));
        }
    };
    for j in 0..window.len() {
        if window[j] == mov_op && j + 5 <= window.len() {
            consider(j + 5, imm32(window, j + 1)?);
        }
        if window[j..].starts_with(&[0x48, 0xc7, modrm]) && j + 7 <= window.len() {
            consider(j + 7, imm32(window, j + 3)?);
        }
        if (window[j] == 0x31 || window[j] == 0x33) && window.get(j + 1) == Some(&modrm) {
            consider(j + 2, 0);
        }
    }
    best.map(|(_, v)| v)
}

/// Every call to `syscall`/`madvise` through the GOT or a PLT stub, and every
/// raw `syscall` instruction, with the constant argument when one is found.
fn code_sites(elf: &elf::Elf) -> Vec<Site> {
    let slots_of = |name: &str| -> BTreeSet<u64> {
        elf.imports
            .iter()
            .filter(|i| i.name == name)
            .flat_map(|i| i.slots.iter().copied())
            .collect()
    };
    let syscall_slots = slots_of("syscall");
    let madvise_slots = slots_of("madvise");

    // PLT stub address -> GOT slot it jumps through.
    let mut stubs: BTreeMap<u64, u64> = BTreeMap::new();
    for c in elf.code.iter().filter(|c| c.name.starts_with(".plt")) {
        let b = c.bytes;
        for i in 0..b.len() {
            if b[i..].starts_with(&[0xff, 0x25]) {
                if let Some(rel) = rel32(b, i + 2) {
                    let slot = (c.addr as i64 + i as i64 + 6 + rel) as u64;
                    let mut start = i;
                    if start >= 1 && b[start - 1] == 0xf2 {
                        start -= 1; // bnd
                    }
                    if start >= 4 && b[start - 4..start] == [0xf3, 0x0f, 0x1e, 0xfa] {
                        start -= 4; // endbr64
                    }
                    stubs.insert(c.addr + start as u64, slot);
                }
            }
        }
    }

    let mut sites = Vec::new();
    let mut seen = BTreeSet::new();
    for c in elf.code.iter().filter(|c| !c.name.starts_with(".plt")) {
        let b = c.bytes;
        for i in 0..b.len().saturating_sub(1) {
            let here = c.addr + i as u64;
            let slot = match (b[i], b[i + 1]) {
                (0xe8 | 0xe9, _) => rel32(b, i + 1)
                    .map(|rel| (here as i64 + 5 + rel) as u64)
                    .and_then(|t| stubs.get(&t).copied()),
                (0xff, 0x15 | 0x25) => rel32(b, i + 2).map(|rel| (here as i64 + 6 + rel) as u64),
                _ => None,
            };
            let window = &b[i.saturating_sub(LOOKBACK)..i];
            let kind = match slot {
                Some(s) if syscall_slots.contains(&s) => {
                    Some((SiteKind::SyscallFn, last_constant(window, 0xbf, 0xff)))
                }
                Some(s) if madvise_slots.contains(&s) => {
                    Some((SiteKind::Madvise, last_constant(window, 0xba, 0xd2)))
                }
                _ if b[i] == 0x0f && b[i + 1] == 0x05 => {
                    // A raw syscall needs its number in eax just before it;
                    // without one the two bytes are likely part of another
                    // instruction, so they are not counted.
                    last_constant(window, 0xb8, 0xc0)
                        .filter(|nr| *nr < 1024)
                        .map(|nr| (SiteKind::SyscallInsn, Some(nr)))
                }
                _ => None,
            };
            if let Some((kind, constant)) = kind {
                if seen.insert(here) {
                    sites.push(Site {
                        kind,
                        address: here,
                        constant,
                    });
                }
            }
        }
    }
    sites
}

#[cfg(test)]
mod tests;

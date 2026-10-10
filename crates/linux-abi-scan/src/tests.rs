//! Tests against small ELF files built here, so they run on any host and
//! pin each pattern exactly. A Linux-only test also scans a real binary.

use super::*;

struct Imp {
    name: &'static str,
    version: Option<&'static str>,
    weak: bool,
    slot: Option<u64>,
}

fn imp(name: &'static str, version: Option<&'static str>, weak: bool, slot: Option<u64>) -> Imp {
    Imp {
        name,
        version,
        weak,
        slot,
    }
}

#[derive(Default)]
struct Spec {
    imports: Vec<Imp>,
    text: Vec<u8>,
    plt_sec: Vec<u8>,
    abi_tag: Option<[u32; 3]>,
    relr: bool,
    /// Mark every version requirement VER_FLG_WEAK.
    weak_versions: bool,
}

/// Virtual address = file offset, so code addresses are easy to compute.
const TEXT: u64 = 0x1000;
const PLT: u64 = 0x2000;
const GOT: u64 = 0x3000;

fn build(spec: &Spec) -> Vec<u8> {
    let mut file = vec![0u8; 0x4000];
    let put = |file: &mut Vec<u8>, at: usize, bytes: &[u8]| {
        if file.len() < at + bytes.len() {
            file.resize(at + bytes.len(), 0);
        }
        file[at..at + bytes.len()].copy_from_slice(bytes);
    };
    put(&mut file, TEXT as usize, &spec.text);
    put(&mut file, PLT as usize, &spec.plt_sec);

    // Dynamic string table, symbols, versions.
    let mut dynstr = vec![0u8];
    let mut add = |s: &str| {
        let off = dynstr.len() as u32;
        dynstr.extend_from_slice(s.as_bytes());
        dynstr.push(0);
        off
    };
    let libc = add("libc.so.6");
    let mut versions: Vec<&str> = spec.imports.iter().filter_map(|i| i.version).collect();
    versions.sort();
    versions.dedup();
    let version_names: Vec<u32> = versions.iter().map(|v| add(v)).collect();
    let mut dynsym = vec![0u8; 24];
    let mut versym = vec![0u8; 2];
    let mut rela = Vec::new();
    for (k, i) in spec.imports.iter().enumerate() {
        let mut e = [0u8; 24];
        e[0..4].copy_from_slice(&add(i.name).to_le_bytes());
        e[4] = (if i.weak { 2 } else { 1 }) << 4 | 2;
        dynsym.extend_from_slice(&e);
        let vi: u16 = match i.version {
            Some(v) => 2 + versions.iter().position(|x| *x == v).unwrap() as u16,
            None => 1,
        };
        versym.extend_from_slice(&vi.to_le_bytes());
        if let Some(slot) = i.slot {
            rela.extend_from_slice(&slot.to_le_bytes());
            rela.extend_from_slice(&((((k + 1) as u64) << 32) | 7).to_le_bytes());
            rela.extend_from_slice(&0u64.to_le_bytes());
        }
    }
    let mut verneed = Vec::new();
    if !versions.is_empty() {
        verneed.extend_from_slice(&1u16.to_le_bytes());
        verneed.extend_from_slice(&(versions.len() as u16).to_le_bytes());
        verneed.extend_from_slice(&libc.to_le_bytes());
        verneed.extend_from_slice(&16u32.to_le_bytes());
        verneed.extend_from_slice(&0u32.to_le_bytes());
        for (k, name) in version_names.iter().enumerate() {
            verneed.extend_from_slice(&0u32.to_le_bytes()); // hash (unused)
            verneed.extend_from_slice(&(if spec.weak_versions { 2u16 } else { 0 }).to_le_bytes());
            verneed.extend_from_slice(&(2 + k as u16).to_le_bytes());
            verneed.extend_from_slice(&name.to_le_bytes());
            let next: u32 = if k + 1 < versions.len() { 16 } else { 0 };
            verneed.extend_from_slice(&next.to_le_bytes());
        }
    }
    let mut note = Vec::new();
    if let Some([a, b, c]) = spec.abi_tag {
        for v in [4u32, 16, 1] {
            note.extend_from_slice(&v.to_le_bytes());
        }
        note.extend_from_slice(b"GNU\0");
        for v in [0u32, a, b, c] {
            note.extend_from_slice(&v.to_le_bytes());
        }
    }
    let mut dynamic = Vec::new();
    if spec.relr {
        dynamic.extend_from_slice(&36i64.to_le_bytes());
        dynamic.extend_from_slice(&0u64.to_le_bytes());
    }
    dynamic.extend_from_slice(&[0u8; 16]);

    // Place the tables after the code.
    let mut cursor = file.len();
    let mut place = |file: &mut Vec<u8>, bytes: &[u8]| {
        let at = cursor;
        put(file, at, bytes);
        cursor += bytes.len().div_ceil(8) * 8;
        at as u64
    };
    let dynstr_off = place(&mut file, &dynstr);
    let dynsym_off = place(&mut file, &dynsym);
    let versym_off = place(&mut file, &versym);
    let verneed_off = place(&mut file, &verneed);
    let rela_off = place(&mut file, &rela);
    let note_off = place(&mut file, &note);
    let dynamic_off = place(&mut file, &dynamic);
    let mut shstr = vec![0u8];
    let mut sname = |s: &str| {
        let off = shstr.len() as u32;
        shstr.extend_from_slice(s.as_bytes());
        shstr.push(0);
        off
    };
    // (name, type, flags, addr/offset, size, link, entsize)
    let sections = [
        (
            sname(".text"),
            1u32,
            6u64,
            TEXT,
            spec.text.len() as u64,
            0u32,
            0u64,
        ),
        (
            sname(".plt.sec"),
            1,
            6,
            PLT,
            spec.plt_sec.len() as u64,
            0,
            0,
        ),
        (
            sname(".dynstr"),
            3,
            2,
            dynstr_off,
            dynstr.len() as u64,
            0,
            0,
        ),
        (
            sname(".dynsym"),
            11,
            2,
            dynsym_off,
            dynsym.len() as u64,
            3,
            24,
        ),
        (
            sname(".gnu.version"),
            0x6fff_ffff,
            2,
            versym_off,
            versym.len() as u64,
            4,
            2,
        ),
        (
            sname(".gnu.version_r"),
            0x6fff_fffe,
            2,
            verneed_off,
            verneed.len() as u64,
            3,
            0,
        ),
        (sname(".rela.plt"), 4, 2, rela_off, rela.len() as u64, 4, 24),
        (
            sname(".note.ABI-tag"),
            7,
            2,
            note_off,
            note.len() as u64,
            0,
            0,
        ),
        (
            sname(".dynamic"),
            6,
            3,
            dynamic_off,
            dynamic.len() as u64,
            3,
            16,
        ),
    ];
    let shstr_name = sname(".shstrtab");
    let shstr_off = place(&mut file, &shstr);
    let phoff = place(&mut file, &[0u8; 2 * 56]);
    let shoff = cursor as u64;
    let mut sh = vec![0u8; 64]; // null section
    for (name, kind, flags, at, size, link, entsize) in
        sections
            .into_iter()
            .chain([(shstr_name, 3, 0, shstr_off, shstr.len() as u64, 0, 0)])
    {
        let mut e = [0u8; 64];
        e[0..4].copy_from_slice(&name.to_le_bytes());
        e[4..8].copy_from_slice(&kind.to_le_bytes());
        e[8..16].copy_from_slice(&flags.to_le_bytes());
        e[16..24].copy_from_slice(&at.to_le_bytes());
        e[24..32].copy_from_slice(&at.to_le_bytes());
        e[32..40].copy_from_slice(&size.to_le_bytes());
        e[40..44].copy_from_slice(&link.to_le_bytes());
        e[56..64].copy_from_slice(&entsize.to_le_bytes());
        sh.extend_from_slice(&e);
    }
    put(&mut file, shoff as usize, &sh);
    // Program headers: PT_NOTE and PT_DYNAMIC.
    let mut ph = Vec::new();
    for (kind, at, size) in [
        (4u32, note_off, note.len() as u64),
        (2, dynamic_off, dynamic.len() as u64),
    ] {
        let mut e = [0u8; 56];
        e[0..4].copy_from_slice(&kind.to_le_bytes());
        e[8..16].copy_from_slice(&at.to_le_bytes());
        e[16..24].copy_from_slice(&at.to_le_bytes());
        e[32..40].copy_from_slice(&size.to_le_bytes());
        e[40..48].copy_from_slice(&size.to_le_bytes());
        ph.extend_from_slice(&e);
    }
    put(&mut file, phoff as usize, &ph);
    // ELF header.
    let mut h = [0u8; 64];
    h[0..4].copy_from_slice(b"\x7fELF");
    h[4] = 2;
    h[5] = 1;
    h[6] = 1;
    h[16..18].copy_from_slice(&3u16.to_le_bytes()); // ET_DYN
    h[18..20].copy_from_slice(&62u16.to_le_bytes());
    h[32..40].copy_from_slice(&phoff.to_le_bytes());
    h[40..48].copy_from_slice(&shoff.to_le_bytes());
    h[52..54].copy_from_slice(&64u16.to_le_bytes());
    h[54..56].copy_from_slice(&56u16.to_le_bytes());
    h[56..58].copy_from_slice(&2u16.to_le_bytes());
    h[58..60].copy_from_slice(&64u16.to_le_bytes());
    h[60..62].copy_from_slice(&11u16.to_le_bytes());
    h[62..64].copy_from_slice(&10u16.to_le_bytes());
    put(&mut file, 0, &h);
    file
}

fn kv(s: &str) -> KernelVersion {
    KernelVersion::parse(s).unwrap()
}

fn target(glibc: Option<&str>, kernel: Option<&str>) -> ScanTarget {
    ScanTarget {
        glibc: glibc.map(|g| GlibcVersion::parse(g).unwrap()),
        kernel: kernel.map(|k| Target::kernel(kv(k))),
        label: "test".into(),
    }
}

fn find<'a>(r: &'a Report, check: &str, subject: &str) -> Option<&'a Finding> {
    r.findings
        .iter()
        .find(|f| f.check == check && f.subject.starts_with(subject))
}

/// `call qword [rip + rel]` at `at` (an offset in .text) to `slot`.
fn call_got(at: u64, slot: u64) -> Vec<u8> {
    let rel = slot as i64 - (TEXT + at + 6) as i64;
    let mut v = vec![0xff, 0x15];
    v.extend_from_slice(&(rel as i32).to_le_bytes());
    v
}

#[test]
fn strong_wrapper_for_a_missing_syscall_is_an_error() {
    let spec = Spec {
        imports: vec![
            imp("statx", Some("GLIBC_2.28"), false, None),
            imp("read", Some("GLIBC_2.2.5"), false, None),
        ],
        ..Default::default()
    };
    let r = scan(&build(&spec), &target(Some("2.28"), Some("4.4"))).unwrap();
    let f = find(&r, "glibc-wrapper", "statx").unwrap();
    assert_eq!(
        (f.severity, f.confidence),
        (Severity::Error, Confidence::Exact)
    );
    assert!(f.detail.contains("Linux 4.11"), "{}", f.detail);
    assert_eq!(
        find(&r, "glibc-wrapper", "read").unwrap().severity,
        Severity::Info
    );
    assert_eq!(r.exit_code(false), 1);
    // On 4.11 the same file is fine.
    let r = scan(&build(&spec), &target(Some("2.28"), Some("4.11"))).unwrap();
    assert_eq!(r.exit_code(true), 0, "{}", r.to_text(true));
}

#[test]
fn weak_wrapper_is_a_warning_with_the_std_fallback_noted() {
    let spec = Spec {
        imports: vec![imp("getrandom", Some("GLIBC_2.25"), true, None)],
        ..Default::default()
    };
    let r = scan(&build(&spec), &target(Some("2.17"), Some("3.10"))).unwrap();
    let f = find(&r, "glibc-wrapper", "getrandom").unwrap();
    assert_eq!(f.severity, Severity::Warning);
    assert!(f.detail.contains("falls back"), "{}", f.detail);
    // A weak *symbol* does not make its *version requirement* weak: glibc
    // 2.17's loader refuses the file ("version GLIBC_2.25 not found").
    let need = find(&r, "glibc-version", "libc.so.6 GLIBC_2.25").unwrap();
    assert_eq!(need.severity, Severity::Error);
    assert_eq!(r.exit_code(false), 1);

    // Only a weak version requirement (VER_FLG_WEAK) is optional.
    let spec = Spec {
        weak_versions: true,
        ..spec
    };
    let r = scan(&build(&spec), &target(Some("2.17"), Some("3.10"))).unwrap();
    let need = find(&r, "glibc-version", "libc.so.6 GLIBC_2.25").unwrap();
    assert_eq!(need.severity, Severity::Warning);
    assert_eq!(r.exit_code(false), 0);
    assert_eq!(r.exit_code(true), 1);
}

#[test]
fn glibc_versions_above_the_floor_are_errors() {
    let spec = Spec {
        imports: vec![
            imp("memcpy", Some("GLIBC_2.14"), false, None),
            imp("_Fork", Some("GLIBC_2.34"), false, None),
        ],
        ..Default::default()
    };
    let r = scan(&build(&spec), &target(Some("2.17"), None)).unwrap();
    assert_eq!(
        find(&r, "glibc-version", "libc.so.6 GLIBC_2.14")
            .unwrap()
            .severity,
        Severity::Info
    );
    assert_eq!(
        find(&r, "glibc-version", "libc.so.6 GLIBC_2.34")
            .unwrap()
            .severity,
        Severity::Error
    );
    assert!(
        find(&r, "scope", "kernel").is_some(),
        "host kernel must be reported"
    );
}

#[test]
fn syscall_function_with_a_constant_number() {
    let slot = GOT;
    let mut text = vec![0xbf, 0x3e, 0x01, 0x00, 0x00]; // mov edi, 318 (getrandom)
    text.extend(call_got(5, slot));
    let spec = Spec {
        imports: vec![imp("syscall", Some("GLIBC_2.2.5"), false, Some(slot))],
        text,
        ..Default::default()
    };
    let r = scan(&build(&spec), &target(None, Some("3.10"))).unwrap();
    let f = find(&r, "syscall-call", "getrandom").unwrap();
    assert_eq!(
        (f.severity, f.confidence),
        (Severity::Warning, Confidence::Heuristic)
    );
    assert_eq!(f.address, Some(TEXT + 5));
    let r = scan(&build(&spec), &target(None, Some("3.17"))).unwrap();
    assert_eq!(
        find(&r, "syscall-call", "getrandom").unwrap().severity,
        Severity::Info
    );
}

#[test]
fn syscall_function_without_a_constant_is_counted_not_guessed() {
    let slot = GOT;
    let mut text = vec![0x48, 0x89, 0xc7]; // mov rdi, rax
    text.extend(call_got(3, slot));
    let spec = Spec {
        imports: vec![imp("syscall", Some("GLIBC_2.2.5"), false, Some(slot))],
        text,
        ..Default::default()
    };
    let r = scan(&build(&spec), &target(None, Some("3.10"))).unwrap();
    assert!(r.findings.iter().all(|f| f.check != "syscall-call"));
    let f = find(&r, "scope", "code").unwrap();
    assert!(f.detail.starts_with("1 syscall() calls"), "{}", f.detail);
}

#[test]
fn raw_syscall_instruction_needs_its_number_in_eax() {
    let text = vec![
        0xb8, 0x4c, 0x01, 0x00, 0x00, 0x0f, 0x05, // mov eax, 332 (statx); syscall
        0x90, 0x0f, 0x05, // syscall with no visible number: not counted
    ];
    let spec = Spec {
        text,
        ..Default::default()
    };
    let r = scan(&build(&spec), &target(None, Some("4.4"))).unwrap();
    let hits: Vec<_> = r
        .findings
        .iter()
        .filter(|f| f.check == "syscall-insn")
        .collect();
    assert_eq!(hits.len(), 1, "{}", r.to_text(true));
    assert_eq!(hits[0].subject, "statx");
    assert_eq!(hits[0].severity, Severity::Warning);
}

#[test]
fn madvise_through_a_plt_stub() {
    let slot = GOT + 8;
    // .plt.sec stub: endbr64; bnd jmp [rip + rel] -> slot
    let rel = slot as i64 - (PLT + 4 + 7) as i64;
    let mut plt_sec = vec![0xf3, 0x0f, 0x1e, 0xfa, 0xf2, 0xff, 0x25];
    plt_sec.extend_from_slice(&(rel as i32).to_le_bytes());
    let call = |at: u64| {
        let rel = PLT as i64 - (TEXT + at + 5) as i64;
        let mut v = vec![0xe8];
        v.extend_from_slice(&(rel as i32).to_le_bytes());
        v
    };
    let mut text = vec![0xba, 0x16, 0x00, 0x00, 0x00]; // mov edx, 22 (MADV_POPULATE_READ)
    text.extend(call(5));
    text.extend([0xba, 0x04, 0x00, 0x00, 0x00]); // mov edx, 4 (MADV_DONTNEED)
    text.extend(call(15));
    let spec = Spec {
        imports: vec![imp("madvise", Some("GLIBC_2.2.5"), false, Some(slot))],
        text,
        plt_sec,
        ..Default::default()
    };
    let r = scan(&build(&spec), &target(None, Some("4.12"))).unwrap();
    let hits: Vec<_> = r.findings.iter().filter(|f| f.check == "madvise").collect();
    assert_eq!(hits.len(), 1, "{}", r.to_text(true));
    assert!(hits[0].subject.starts_with("MADV_POPULATE_READ"));
    assert_eq!(hits[0].address, Some(TEXT + 5));
}

#[test]
fn relr_and_abi_tag() {
    let spec = Spec {
        relr: true,
        abi_tag: Some([4, 4, 0]),
        ..Default::default()
    };
    let r = scan(&build(&spec), &target(Some("2.22"), Some("3.10"))).unwrap();
    assert_eq!(
        find(&r, "relr", "DT_RELR").unwrap().severity,
        Severity::Error
    );
    assert_eq!(
        find(&r, "abi-tag", "NT_GNU_ABI_TAG").unwrap().severity,
        Severity::Warning
    );
    let r = scan(&build(&spec), &target(Some("2.36"), Some("4.4"))).unwrap();
    assert_eq!(r.exit_code(true), 0, "{}", r.to_text(true));
}

#[test]
fn distro_target_includes_backports() {
    let sel = linux_targets::Selection::resolve(&linux_targets::Request {
        distro: Some("sles-12-sp5"),
        ..Default::default()
    })
    .unwrap();
    let t = ScanTarget::from_selection(&sel);
    assert_eq!(
        t.glibc,
        Some(GlibcVersion {
            major: 2,
            minor: 22
        })
    );
    assert_eq!(t.kernel.as_ref().unwrap().kernel, kv("4.12"));
    assert!(t.label.contains("SUSE"));
}

#[test]
fn malformed_input_fails_without_panicking() {
    assert!(scan(b"not an elf", &target(None, None)).is_err());
    let spec = Spec {
        imports: vec![imp("syscall", Some("GLIBC_2.2.5"), false, Some(GOT))],
        text: vec![0xbf, 1, 0, 0, 0, 0xff, 0x15, 0, 0, 0, 0],
        abi_tag: Some([3, 2, 0]),
        relr: true,
        ..Default::default()
    };
    let good = build(&spec);
    assert!(scan(&good, &target(Some("2.17"), Some("3.10"))).is_ok());
    for len in (0..good.len()).step_by(7) {
        let _ = scan(&good[..len], &target(Some("2.17"), Some("3.10")));
    }
    let mut corrupt = good.clone();
    for i in (64..corrupt.len()).step_by(97) {
        corrupt[i] ^= 0xa5;
        let _ = scan(&corrupt, &target(Some("2.17"), Some("3.10")));
    }
}

#[test]
fn text_and_tsv_reports() {
    let spec = Spec {
        imports: vec![imp("statx", Some("GLIBC_2.28"), false, None)],
        ..Default::default()
    };
    let r = scan(&build(&spec), &target(Some("2.17"), Some("4.4"))).unwrap();
    let text = r.to_text(false);
    assert!(text.contains("ERROR exact"), "{text}");
    assert!(text.contains("tier 1"), "{text}");
    let tsv = r.to_tsv();
    assert!(tsv.starts_with("severity\tconfidence\tcheck\tsubject\taddress\tdetail\n"));
    assert!(
        tsv.contains("error\texact\tglibc-wrapper\tstatx@GLIBC_2.28"),
        "{tsv}"
    );
}

/// The test binary itself is a real Rust program.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[test]
fn scans_a_real_rust_binary() {
    let exe = std::env::current_exe().unwrap();
    let r = scan_file(&exe, &target(Some("2.17"), Some("3.10"))).unwrap();
    assert!(
        r.findings.iter().any(|f| f.check == "glibc-version"),
        "{}",
        r.to_text(true)
    );
}

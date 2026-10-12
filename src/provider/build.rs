//! Builds a release's startup objects and nonshared archives from recorded
//! sources with the host's GCC, and verifies every object against the interface
//! recorded from the container build. The same procedure as glibc-provider's
//! `glibc-provider-replay`, which qualifies GCC versions in CI.
//!
//! Needs GCC and GNU binutils (`ar`, `readelf`).
use crate::sha256;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::Command,
};

/// GCC major versions qualified in CI for every release (glibc-provider's Replay
/// workflow). Other versions are verified locally before use.
pub const QUALIFIED_GCC: std::ops::RangeInclusive<u32> = 11..=16;

/// One object behind a target, as recorded in `objects.tsv`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Object {
    pub target: String,
    pub member: u32,
    pub object: String,
    pub kind: String,
    pub directory: String,
    pub command: String,
}

/// Runs PROGRAM in DIR with search-path overrides removed and a fixed locale.
pub(crate) fn run(program: &Path, args: &[String], dir: &Path) -> Result<String, String> {
    let mut command = Command::new(program);
    command.args(args).current_dir(dir).env("LC_ALL", "C");
    for key in [
        "CPATH",
        "C_INCLUDE_PATH",
        "CPLUS_INCLUDE_PATH",
        "OBJC_INCLUDE_PATH",
        "LIBRARY_PATH",
        "GCC_EXEC_PREFIX",
        "COMPILER_PATH",
        "DEPENDENCIES_OUTPUT",
        "SUNPRO_DEPENDENCIES",
    ] {
        command.env_remove(key);
    }
    let output = command
        .output()
        .map_err(|e| format!("cannot run {}: {e}", program.display()))?;
    if !output.status.success() {
        return Err(format!(
            "{} failed in {}: {}",
            program.display(),
            dir.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// The host tools a local build uses.
#[derive(Clone, Debug)]
pub struct Tools {
    pub gcc: PathBuf,
    pub ar: PathBuf,
    pub readelf: PathBuf,
    pub assembler: PathBuf,
    pub linker: PathBuf,
}

impl Default for Tools {
    fn default() -> Self {
        Self {
            gcc: "gcc".into(),
            ar: "ar".into(),
            readelf: "readelf".into(),
            assembler: "as".into(),
            linker: "ld".into(),
        }
    }
}

impl Tools {
    /// The first line of each tool's `--version`, which identifies the toolchain.
    pub fn versions(&self) -> Result<Vec<(&'static str, String)>, String> {
        let mut versions = Vec::new();
        for (name, path) in [
            ("gcc", &self.gcc),
            ("as", &self.assembler),
            ("ld", &self.linker),
            ("ar", &self.ar),
            ("readelf", &self.readelf),
        ] {
            let text = run(path, &["--version".into()], Path::new("."))?;
            versions.push((name, text.lines().next().unwrap_or("").trim().to_string()));
        }
        if versions[0].1.to_ascii_lowercase().contains("clang") {
            return Err(format!(
                "{} is Clang; glibc's sources need GCC (Clang ignores attributes glibc \
                 applies after a definition and would export hidden symbols)",
                self.gcc.display()
            ));
        }
        Ok(versions)
    }

    /// GCC's major version.
    pub fn gcc_major(&self) -> Result<u32, String> {
        let text = run(&self.gcc, &["-dumpversion".into()], Path::new("."))?;
        text.trim()
            .split('.')
            .next()
            .and_then(|m| m.parse().ok())
            .ok_or_else(|| format!("cannot read GCC's version from {text:?}"))
    }

    /// The directory of the host GCC's `crtbegin.o` and `crtend.o`.
    pub fn gcc_startup(&self) -> Result<PathBuf, String> {
        let text = run(
            &self.gcc,
            &["-print-file-name=crtbegin.o".into()],
            Path::new("."),
        )?;
        let path = PathBuf::from(text.trim());
        if !path.is_absolute() || !path.is_file() {
            return Err(format!("{} has no crtbegin.o", self.gcc.display()));
        }
        let dir = path.parent().ok_or("crtbegin.o has no directory")?;
        if !dir.join("crtend.o").is_file() {
            return Err(format!("{} lacks crtend.o", dir.display()));
        }
        fs::canonicalize(dir).map_err(|e| e.to_string())
    }
}

/// Whether GCC accepts FLAG when assembling an empty file.
fn accepts(tools: &Tools, flag: &str, scratch: &Path) -> bool {
    let (source, object) = (scratch.join("probe.s"), scratch.join("probe.o"));
    if fs::write(&source, "").is_err() {
        return false;
    }
    let args = [
        "-c".to_string(),
        source.to_string_lossy().into_owned(),
        "-o".into(),
        object.to_string_lossy().into_owned(),
        flag.into(),
    ];
    let accepted = run(&tools.gcc, &args, scratch).is_ok();
    let _ = fs::remove_file(&source);
    let _ = fs::remove_file(&object);
    accepted
}

/// Flags that make the builder's implicit code-generation defaults explicit
/// and switch off the host compiler's own (distribution) defaults, for whatever
/// a recorded command does not already set. `builder` holds `release.tsv`'s
/// `builder.*` values.
pub fn defaults_flags(command: &str, builder: &BTreeMap<String, String>) -> Vec<String> {
    let tokens: Vec<&str> = command.split_whitespace().collect();
    let has = |prefixes: &[&str]| {
        tokens
            .iter()
            .any(|t| prefixes.iter().any(|p| t.starts_with(p)))
    };
    let on = |key: &str| {
        builder
            .get(&format!("builder.{key}"))
            .is_some_and(|v| v == "yes")
    };
    let mut flags = Vec::new();
    if !has(&[
        "-fPIC", "-fpic", "-fPIE", "-fpie", "-fno-pic", "-fno-PIC", "-fno-pie", "-fno-PIE",
    ]) {
        flags.push(if on("pie") { "-fPIE" } else { "-fno-pie" }.to_string());
    }
    if !has(&["-fstack-protector", "-fno-stack-protector"]) && !on("stack_protector") {
        flags.push("-fno-stack-protector".into());
    }
    if !has(&["-fcf-protection"]) && !on("cet") {
        flags.push("-fcf-protection=none".into());
    }
    if !on("fortify") {
        flags.push("-U_FORTIFY_SOURCE".into());
    }
    if !has(&["-fstack-clash-protection", "-fno-stack-clash-protection"]) {
        flags.push("-fno-stack-clash-protection".into());
    }
    flags
}

/// A recorded command for the host's GCC: `@SRC@`/`@BUILD@` replaced, dependency
/// files dropped, EXTRA appended.
pub fn translate(
    command: &str,
    gcc: &str,
    src: &str,
    build: &str,
    extra: &[String],
) -> Result<Vec<String>, String> {
    let mut tokens: Vec<String> = command
        .split_whitespace()
        .map(|t| t.replace("@SRC@", src).replace("@BUILD@", build))
        .collect();
    if tokens.first().map(String::as_str) != Some("gcc") {
        return Err(format!("recorded command does not run gcc: {command:?}"));
    }
    tokens[0] = gcc.into();
    let mut out = Vec::new();
    let mut skip = false;
    for token in tokens {
        if skip {
            skip = false;
            continue;
        }
        match token.as_str() {
            "-MD" | "-MP" => {}
            "-MF" | "-MT" => skip = true,
            _ => out.push(token),
        }
    }
    out.extend(extra.iter().cloned());
    Ok(out)
}

/// What must match between two relocatable objects.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Interface {
    /// Defined global/weak symbols: name, type, binding, visibility, section.
    pub defined: BTreeSet<String>,
    /// Undefined global/weak symbols: name, binding.
    pub undefined: BTreeSet<String>,
    /// Non-empty allocated sections (name, flags), except the stack note.
    pub sections: BTreeSet<String>,
    /// References outside debug sections: (section, form, symbol).
    pub references: BTreeSet<(String, String, String)>,
}

/// The form of a relocation, merging encodings the linker resolves alike:
/// newer assemblers emit `PLT32` for calls where older ones emitted `PC32`
/// (binutils 2.31), and relaxable `GOTPCRELX` forms where older ones emitted
/// `GOTPCREL` (binutils 2.26).
fn relocation_form(kind: &str) -> String {
    match kind {
        "R_X86_64_PC32" | "R_X86_64_PLT32" => "pc-relative".into(),
        "R_X86_64_GOTPCREL" | "R_X86_64_GOTPCRELX" | "R_X86_64_REX_GOTPCRELX" => "got".into(),
        other => other.into(),
    }
}

/// An object's interface and the SHA-256 of its allocated sections' names and
/// bytes, read with `readelf`.
pub fn interface(readelf: &Path, object: &Path) -> Result<(Interface, String), String> {
    let path = object.to_string_lossy().to_string();
    let here = Path::new(".");
    let read = |flags: &[&str]| {
        let mut args: Vec<String> = flags.iter().map(|f| f.to_string()).collect();
        args.push(path.clone());
        run(readelf, &args, here)
    };
    let mut names: BTreeMap<String, String> = BTreeMap::new();
    let mut result = Interface::default();
    let mut allocated = Vec::new();
    for line in read(&["-W", "-S"])?.lines() {
        let Some(rest) = line.trim_start().strip_prefix('[') else {
            continue;
        };
        let Some((index, rest)) = rest.split_once(']') else {
            continue;
        };
        let fields: Vec<&str> = rest.split_whitespace().collect();
        if fields.len() < 7 || fields[0] == "Name" {
            continue;
        }
        names.insert(index.trim().to_string(), fields[0].to_string());
        // Name Type Address Off Size ES Flg Lk Inf Al
        let flags = if fields.len() >= 10 { fields[6] } else { "" };
        let empty = u64::from_str_radix(fields[4], 16).is_ok_and(|size| size == 0);
        if flags.contains('A') && fields[0] != ".note.GNU-stack" && !empty {
            result.sections.insert(format!("{} {}", fields[0], flags));
            allocated.push(fields[0].to_string());
        }
    }
    for line in read(&["-W", "-s"])?.lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        if f.len() < 8 || !f[0].ends_with(':') || f[0] == "Num:" {
            continue;
        }
        let (kind, bind, visibility, index, name) = (f[3], f[4], f[5], f[6], f[7]);
        if bind == "LOCAL" {
            continue;
        }
        if index == "UND" {
            // Older GNU as references the linker-defined GOT symbol.
            if name != "_GLOBAL_OFFSET_TABLE_" {
                result.undefined.insert(format!("{name} {bind}"));
            }
        } else {
            let section = names.get(index).map_or(index, String::as_str);
            result
                .defined
                .insert(format!("{name} {kind} {bind} {visibility} {section}"));
        }
    }
    let mut section = String::new();
    for line in read(&["-W", "-r"])?.lines() {
        if let Some(rest) = line.strip_prefix("Relocation section '") {
            section = rest.split('\'').next().unwrap_or("").to_string();
            continue;
        }
        if section.contains(".debug") || section.is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split_whitespace().collect();
        if f.len() < 3 || !f[2].starts_with("R_X86_64") {
            continue;
        }
        let symbol = f.get(4).map_or("", |s| s.split('+').next().unwrap_or(s));
        result
            .references
            .insert((section.clone(), relocation_form(f[2]), symbol.to_string()));
    }
    let mut bytes = Vec::new();
    for name in allocated {
        bytes.extend(name.as_bytes());
        bytes.push(0);
        bytes.extend(hex_dump_bytes(&read(&["-W", "-x", &name])?));
    }
    Ok((result, sha256::hex(&sha256::sha256(&bytes))))
}

/// The bytes of a `readelf -x` dump: each data line is `  0xADDRESS ` followed
/// by up to four groups of eight hex digits in a fixed 35-column field, then
/// the ASCII rendering.
fn hex_dump_bytes(dump: &str) -> Vec<u8> {
    let mut bytes = Vec::new();
    for line in dump.lines() {
        let Some(rest) = line.trim_start().strip_prefix("0x") else {
            continue;
        };
        let Some((_, data)) = rest.split_once(' ') else {
            continue;
        };
        let field: String = data
            .chars()
            .take(35)
            .filter(|c| !c.is_whitespace())
            .collect();
        bytes.extend(
            (0..field.len() / 2)
                .filter_map(|i| u8::from_str_radix(field.get(2 * i..2 * i + 2)?, 16).ok()),
        );
    }
    bytes
}

/// Reads `interfaces.tsv`: object -> (interface, allocated-bytes SHA-256).
pub fn read_interfaces(text: &str) -> Result<BTreeMap<String, (Interface, String)>, String> {
    let mut lines = text.lines();
    if lines.next() != Some("object\tkind\tvalue") {
        return Err("interfaces.tsv: unexpected header".into());
    }
    let mut map: BTreeMap<String, (Interface, String)> = BTreeMap::new();
    for line in lines {
        let [object, kind, value] = line.split('\t').collect::<Vec<_>>()[..] else {
            return Err(format!("interfaces.tsv: malformed row {line:?}"));
        };
        let (interface, digest) = map.entry(object.to_string()).or_default();
        let value = value.to_string();
        match kind {
            "defined" => drop(interface.defined.insert(value)),
            "undefined" => drop(interface.undefined.insert(value)),
            "section" => drop(interface.sections.insert(value)),
            "reference" => {
                let mut parts = value.splitn(3, ' ').map(String::from);
                let (Some(s), Some(f), Some(n)) = (parts.next(), parts.next(), parts.next()) else {
                    return Err(format!("interfaces.tsv: malformed reference {value}"));
                };
                interface.references.insert((s, f, n));
            }
            "bytes" => *digest = value,
            _ => return Err(format!("interfaces.tsv: unknown kind {kind}")),
        }
    }
    Ok(map)
}

/// How RECORDED (the container build) and BUILT differ; empty if they match.
pub fn describe(recorded: &Interface, built: &Interface) -> String {
    let diff = |name: &str, x: &BTreeSet<String>, y: &BTreeSet<String>| -> Option<String> {
        let only_x: Vec<_> = x.difference(y).cloned().collect();
        let only_y: Vec<_> = y.difference(x).cloned().collect();
        (!only_x.is_empty() || !only_y.is_empty())
            .then(|| format!("{name}: container only {only_x:?}, replay only {only_y:?}"))
    };
    // A reference the container made directly and the build makes through the
    // GOT resolves to the same address; the GOT form is the more general one.
    let only_a: BTreeSet<_> = recorded
        .references
        .difference(&built.references)
        .cloned()
        .collect();
    let only_b: BTreeSet<_> = built
        .references
        .difference(&recorded.references)
        .cloned()
        .collect();
    let container: BTreeSet<String> = only_a
        .iter()
        .filter(|r| {
            !(r.1 == "pc-relative" && only_b.contains(&(r.0.clone(), "got".into(), r.2.clone())))
        })
        .map(|r| format!("{} {} {}", r.0, r.1, r.2))
        .collect();
    let replay: BTreeSet<String> = only_b
        .iter()
        .filter(|r| {
            !(r.1 == "got" && only_a.contains(&(r.0.clone(), "pc-relative".into(), r.2.clone())))
        })
        .map(|r| format!("{} {} {}", r.0, r.1, r.2))
        .collect();
    [
        diff("defined", &recorded.defined, &built.defined),
        diff("undefined", &recorded.undefined, &built.undefined),
        diff("sections", &recorded.sections, &built.sections),
        diff("relocations", &container, &replay),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join("; ")
}

/// What a release's recorded data supplies to a build.
pub struct Recorded<'a> {
    pub release: &'a str,
    /// `release.tsv` as key/value pairs.
    pub values: BTreeMap<String, String>,
    pub objects: Vec<Object>,
    /// `interfaces.tsv`.
    pub interfaces: &'a str,
    /// glibc-provider's `ACCEPTED_DIFFERENCES` table.
    pub accepted: &'a str,
}

/// The verification of one built object.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Verdict {
    pub object: String,
    /// `identical-bytes`, `equivalent`, `accepted` or `differs`.
    pub status: &'static str,
    pub detail: String,
}

/// The result of [`build`].
pub struct Built {
    /// Each target (`crt1.o`, `libc_nonshared.a`, ...) and its path in the build tree.
    pub targets: BTreeMap<String, PathBuf>,
    pub verdicts: Vec<Verdict>,
}

impl Built {
    /// Whether every object matches the container build or differs only as reviewed.
    pub fn verified(&self) -> bool {
        self.verdicts.iter().all(|v| v.status != "differs")
    }

    /// Count per status, such as `identical-bytes: 12, equivalent: 3`.
    pub fn summary(&self) -> String {
        let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
        for verdict in &self.verdicts {
            *counts.entry(verdict.status).or_default() += 1;
        }
        counts
            .iter()
            .map(|(status, count)| format!("{status}: {count}"))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// Builds every object and target of RECORDED in INPUTS (written by
/// glibc-provider-sources' `write_sources`: `src/`, `build/`, `uapi/`) and verifies
/// each object against its recorded interface.
pub fn build(recorded: &Recorded<'_>, inputs: &Path, tools: &Tools) -> Result<Built, String> {
    let (src, build, uapi) = (
        inputs.join("src"),
        inputs.join("build"),
        inputs.join("uapi"),
    );
    let interfaces = read_interfaces(recorded.interfaces)?;
    if !recorded.values.contains_key("builder.pie") {
        return Err("release.tsv lacks the builder defaults".into());
    }
    // Only the compiler's own headers and the recorded UAPI headers are visible.
    let mut extra = vec!["-nostdinc".to_string()];
    for dir in ["include", "include-fixed"] {
        let path = run(
            &tools.gcc,
            &[format!("-print-file-name={dir}")],
            Path::new("."),
        )?;
        extra.extend(["-isystem".to_string(), path.trim().to_string()]);
    }
    // Some distributions (Fedora) build GNU as to add an x86 "ISA used" note to
    // every object; the builders' assemblers did not.
    let used_note = "-Wa,-mx86-used-note=no".to_string();
    if accepts(tools, &used_note, inputs) {
        extra.push(used_note);
    }
    extra.extend(["-isystem".to_string(), uapi.to_string_lossy().into_owned()]);
    // A relocatable link adds nothing of its own: no PIE, no build ID.
    let link_extra = vec!["-no-pie".to_string(), "-Wl,--build-id=none".into()];
    let (s, b) = (
        src.to_string_lossy().into_owned(),
        build.to_string_lossy().into_owned(),
    );
    let gcc = tools.gcc.to_string_lossy().into_owned();
    let mut built = BTreeSet::new();
    // Compile first, then relocatable links (their inputs must exist).
    for pass in ["compile", "relocatable"] {
        for row in recorded.objects.iter().filter(|r| r.kind == pass) {
            if !built.insert(row.object.clone()) {
                continue;
            }
            let cwd = if row.directory == "." {
                src.clone()
            } else {
                src.join(&row.directory)
            };
            fs::create_dir_all(&cwd).map_err(|e| e.to_string())?;
            if let Some(parent) = build.join(&row.object).parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let mut flags = if pass == "compile" {
                extra.clone()
            } else {
                link_extra.clone()
            };
            if pass == "compile" {
                flags.extend(defaults_flags(&row.command, &recorded.values));
            }
            let args = translate(&row.command, &gcc, &s, &b, &flags)?;
            run(&tools.gcc, &args[1..], &cwd)
                .map_err(|e| format!("glibc {}: {}: {e}", recorded.release, row.object))?;
        }
    }
    let mut targets = BTreeMap::new();
    let mut archives: BTreeMap<&str, Vec<(u32, &str)>> = BTreeMap::new();
    for row in &recorded.objects {
        if row.target.ends_with(".a") {
            archives
                .entry(&row.target)
                .or_default()
                .push((row.member, &row.object));
        } else if Path::new(&row.object).file_name() == Some(row.target.as_ref()) {
            targets.insert(row.target.clone(), build.join(&row.object));
        }
    }
    for (archive, mut members) in archives {
        members.sort();
        members.dedup();
        let path = build.join(archive);
        let mut args = vec!["rcD".to_string(), path.to_string_lossy().into_owned()];
        args.extend(
            members
                .iter()
                .map(|(_, m)| build.join(m).to_string_lossy().into_owned()),
        );
        run(&tools.ar, &args, Path::new("."))?;
        targets.insert(archive.to_string(), path);
    }
    for name in recorded
        .objects
        .iter()
        .map(|o| o.target.as_str())
        .collect::<BTreeSet<_>>()
    {
        if !targets.contains_key(name) {
            return Err(format!(
                "glibc {}: no object builds {name}",
                recorded.release
            ));
        }
    }
    let mut verdicts = Vec::new();
    for object in &built {
        let (reference, reference_bytes) = interfaces
            .get(object)
            .ok_or_else(|| format!("glibc {}: interfaces.tsv lacks {object}", recorded.release))?;
        let (actual, actual_bytes) = interface(&tools.readelf, &build.join(object))?;
        let difference = describe(reference, &actual);
        let reviewed = recorded.accepted.lines().skip(1).any(|line| {
            let row: Vec<&str> = line.split('\t').collect();
            row.len() >= 5
                && row[0] == recorded.release
                && row[1] == object
                && matches!(row[2], "gcc" | "any")
                && row[3] == difference
        });
        let status = if difference.is_empty() {
            if *reference_bytes == actual_bytes {
                "identical-bytes"
            } else {
                "equivalent"
            }
        } else if reviewed {
            "accepted"
        } else {
            "differs"
        };
        verdicts.push(Verdict {
            object: object.clone(),
            status,
            detail: difference,
        });
    }
    Ok(Built { targets, verdicts })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_references_not_encodings() {
        let refs = |items: &[(&str, &str)]| Interface {
            references: items
                .iter()
                .map(|(kind, sym)| (".rela.text".into(), relocation_form(kind), sym.to_string()))
                .collect(),
            ..Interface::default()
        };
        let old = refs(&[("R_X86_64_GOTPCREL", "g"), ("R_X86_64_PC32", "f")]);
        let new = refs(&[("R_X86_64_REX_GOTPCRELX", "g"), ("R_X86_64_PLT32", "f")]);
        assert_eq!(describe(&old, &new), "");
        // A direct reference made through the GOT instead is the general form.
        let direct = refs(&[("R_X86_64_PC32", "h")]);
        let through_got = refs(&[("R_X86_64_GOTPCRELX", "h")]);
        assert_eq!(describe(&direct, &through_got), "");
        // The reverse is not accepted, nor is a missing reference.
        assert_ne!(describe(&through_got, &direct), "");
        assert_ne!(describe(&direct, &refs(&[])), "");
    }

    #[test]
    fn reads_hex_dumps_without_the_ascii_column() {
        let dump = "\nHex dump of section '.rodata.cst4':\n  0x00000000 01000200 deadbeef 31ed4989 ab       ........1.I..\n  0x00000010 cafe                                ..\n";
        assert_eq!(
            hex_dump_bytes(dump),
            [1, 0, 2, 0, 0xde, 0xad, 0xbe, 0xef, 0x31, 0xed, 0x49, 0x89, 0xab, 0xca, 0xfe]
        );
    }

    #[test]
    fn reads_recorded_interfaces() {
        let text = "object\tkind\tvalue\ncsu/start.o\tdefined\t_start FUNC GLOBAL DEFAULT .text\ncsu/start.o\treference\t.rela.text got __libc_start_main\ncsu/start.o\tbytes\tab\n";
        let read = read_interfaces(text).unwrap();
        let (interface, digest) = &read["csu/start.o"];
        assert_eq!(digest, "ab");
        assert!(interface.references.contains(&(
            ".rela.text".into(),
            "got".into(),
            "__libc_start_main".into()
        )));
        assert!(read_interfaces("object\tkind\tvalue\nx\tother\ty\n").is_err());
        assert!(read_interfaces("bad header\n").is_err());
    }

    #[test]
    fn makes_builder_defaults_explicit_only_where_unset() {
        let pie: BTreeMap<String, String> = [("builder.pie".to_string(), "yes".to_string())].into();
        let none = BTreeMap::new();
        assert!(defaults_flags("gcc x.c -c", &pie).contains(&"-fPIE".to_string()));
        assert!(defaults_flags("gcc x.c -c", &none).contains(&"-fno-pie".to_string()));
        let pic = defaults_flags("gcc x.c -c -fPIC -fno-stack-protector", &none);
        assert!(!pic
            .iter()
            .any(|f| f.contains("pie") || f.contains("stack-protector")));
        assert!(pic.contains(&"-U_FORTIFY_SOURCE".to_string()));
    }

    #[test]
    fn translates_recorded_commands() {
        let args = translate(
            "gcc ../x.S -c -I@BUILD@/csu -o @BUILD@/csu/x.o -MD -MP -MF @BUILD@/csu/x.o.dt -MT @BUILD@/csu/x.o",
            "/usr/bin/gcc-14",
            "/o/src",
            "/o/build",
            &["-nostdinc".into()],
        )
        .unwrap();
        assert_eq!(
            args,
            [
                "/usr/bin/gcc-14",
                "../x.S",
                "-c",
                "-I/o/build/csu",
                "-o",
                "/o/build/csu/x.o",
                "-nostdinc"
            ]
        );
        assert!(translate("cc x.c", "gcc", "/s", "/b", &[]).is_err());
    }
}

//! glibc abilist parsing and collection, derived from Zig's glibc tools.
//! Copyright (c) Zig contributors; Copyright (c) 2026 rustcommons contributors.
//! SPDX-License-Identifier: MIT

use crate::version::Version;
use std::{
    collections::BTreeMap,
    fmt::Write as _,
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::Command,
};

pub const TARGET: &str = "x86_64-unknown-linux-gnu";
/// Same library selection as the pinned upstream glibc consolidator.
pub const LIBRARIES: [&str; 8] = ["m", "c", "ld", "resolv", "pthread", "dl", "rt", "util"];
const ABI_PATH: &str = "sysdeps/unix/sysv/linux/x86_64/64";
const MAX_FILE_BYTES: u64 = 10 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Kind {
    Function,
    Object(u16),
    Tls(u16),
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Symbol {
    pub name: String,
    pub version: Version,
    pub kind: Kind,
}

/// Read both pre-2.23 grouped lists and modern per-line versioned lists.
/// Absolute version-node markers (`A`) and GCC_* entries are not glibc symbols.
pub fn parse_abilist(text: &str) -> Result<Vec<Symbol>, String> {
    let mut group: Option<&str> = None;
    let mut records = BTreeMap::new();
    for (index, line) in text.lines().enumerate() {
        let fail = |message: &str| format!("line {}: {message}", index + 1);
        let fields: Vec<_> = line.split_ascii_whitespace().collect();
        if fields.is_empty() {
            continue;
        }
        let indented = line.starts_with([' ', '\t']);
        if !indented && fields.len() == 1 {
            validate_namespace(fields[0]).map_err(|e| fail(&e))?;
            group = Some(fields[0]);
            continue;
        }
        let (namespace, values) = if indented {
            (
                group.ok_or_else(|| fail("symbol before a version group"))?,
                &fields[..],
            )
        } else {
            // A modern row cannot silently inherit a previous grouped version.
            group = None;
            (fields[0], &fields[1..])
        };
        validate_namespace(namespace).map_err(|e| fail(&e))?;
        if values.len() < 2 {
            return Err(fail("expected a symbol name and kind"));
        }
        let name = values[0];
        if !name.bytes().all(|b| b.is_ascii_graphic()) {
            return Err(fail(
                "symbol names must contain printable ASCII without spaces",
            ));
        }
        let size = || -> Result<u16, String> {
            if values.len() != 3 {
                return Err(fail("data/TLS symbols require exactly one size"));
            }
            let raw = values[2];
            let (digits, radix) = raw.strip_prefix("0x").map_or((raw, 10), |s| (s, 16));
            if digits.is_empty()
                || !digits.bytes().all(|b| {
                    if radix == 16 {
                        b.is_ascii_hexdigit()
                    } else {
                        b.is_ascii_digit()
                    }
                })
            {
                return Err(fail("invalid symbol size"));
            }
            u16::from_str_radix(digits, radix).map_err(|_| fail("symbol size exceeds u16"))
        };
        let kind = match values[1] {
            "F" if values.len() == 2 => Kind::Function,
            "D" => Kind::Object(size()?),
            "T" => Kind::Tls(size()?),
            "A" if values.len() == 2 => continue,
            _ => return Err(fail("unsupported symbol kind or extra fields")),
        };
        if namespace.starts_with("GCC_") {
            continue;
        }
        let version: Version = namespace[6..].parse().map_err(|e: String| fail(&e))?;
        let key = (name.to_owned(), version);
        if let Some(previous) = records.insert(key, kind) {
            if previous != kind {
                return Err(fail("conflicting duplicate symbol/version"));
            }
        }
    }
    Ok(records
        .into_iter()
        .map(|((name, version), kind)| Symbol {
            name,
            version,
            kind,
        })
        .collect())
}

fn validate_namespace(text: &str) -> Result<(), String> {
    let numeric = text
        .strip_prefix("GLIBC_")
        .or_else(|| text.strip_prefix("GCC_"))
        .ok_or_else(|| format!("unsupported version namespace {text:?}"))?;
    numeric.parse::<Version>().map(|_| ())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Snapshot {
    pub release: Version,
    pub libraries: BTreeMap<String, Vec<Symbol>>,
}

impl Snapshot {
    /// Load one exact release from a data directory laid out like
    /// glibc-abilists' `data/`: `<root>/<release>/<path in glibc's tree>`.
    /// Missing libraries fail instead of silently producing an incomplete ABI.
    pub fn load(root: &Path, release: Version) -> Result<Self, String> {
        release.validate_release()?;
        let release_dir = root.join(release.to_string());
        let (snapshot, _) = load_source(&release_dir, release)?;
        Ok(snapshot)
    }

    /// Load one exact release from the glibc ABI lists compiled into the
    /// `glibc-abilists` crate (the default `bundled-glibc-data` feature).
    #[cfg(feature = "bundled-glibc-data")]
    pub fn bundled(release: Version) -> Result<Self, String> {
        release.validate_release()?;
        let (snapshot, _) = load_with(release, |relative| bundled_file(release, relative))?;
        Ok(snapshot)
    }

    /// Stable TSV ordered by library, symbol name, and numeric symbol version.
    /// A query with no matches returns the header alone.
    pub fn to_tsv(&self, symbol: Option<&str>) -> String {
        let mut out = String::from("release\ttarget\tlibrary\tsymbol\tversion\tkind\tsize\n");
        for (library, symbols) in &self.libraries {
            for entry in symbols {
                if symbol.is_some_and(|name| name != entry.name) {
                    continue;
                }
                let (kind, size) = match entry.kind {
                    Kind::Function => ("F", String::from("-")),
                    Kind::Object(size) => ("D", size.to_string()),
                    Kind::Tls(size) => ("T", size.to_string()),
                };
                writeln!(
                    out,
                    "{}\t{TARGET}\t{library}\t{}\tGLIBC_{}\t{kind}\t{size}",
                    self.release, entry.name, entry.version
                )
                .unwrap();
            }
        }
        out
    }
}

fn read_inside(root: &Path, relative: &Path) -> Result<Vec<u8>, String> {
    let path = root.join(relative);
    let actual = path
        .canonicalize()
        .map_err(|e| format!("{}: {e}", path.display()))?;
    if !actual.starts_with(root) {
        return Err(format!(
            "{} resolves outside the source tree",
            path.display()
        ));
    }
    if !actual.is_file() {
        return Err(format!("{} is not a regular file", path.display()));
    }
    let mut bytes = Vec::new();
    fs::File::open(&actual)
        .map_err(|e| e.to_string())?
        .take(MAX_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err(format!("{} exceeds the 10 MiB input limit", path.display()));
    }
    Ok(bytes)
}

type SourceFiles = Vec<(PathBuf, Vec<u8>)>;

fn load_source(source: &Path, release: Version) -> Result<(Snapshot, SourceFiles), String> {
    let source = source
        .canonicalize()
        .map_err(|e| format!("{}: {e}", source.display()))?;
    load_with(release, |relative| read_inside(&source, relative))
}

/// The path of a library's ABI list in glibc's tree (and in a data
/// directory's release subdirectory).
pub fn abilist_path(release: Version, library: &str) -> PathBuf {
    let mut path = PathBuf::from(ABI_PATH);
    if release.minor <= 19 {
        path.push("nptl");
    }
    path.join(if library == "ld" {
        "ld.abilist".to_owned()
    } else {
        format!("lib{library}.abilist")
    })
}

fn load_with(
    release: Version,
    read: impl Fn(&Path) -> Result<Vec<u8>, String>,
) -> Result<(Snapshot, SourceFiles), String> {
    let mut libraries = BTreeMap::new();
    let mut files = Vec::new();
    for library in LIBRARIES {
        let relative = abilist_path(release, library);
        let bytes = read(&relative)?;
        let text =
            std::str::from_utf8(&bytes).map_err(|e| format!("{}: {e}", relative.display()))?;
        let records = parse_abilist(text).map_err(|e| format!("{}: {e}", relative.display()))?;
        if records.iter().any(|record| record.version > release) {
            return Err(format!(
                "{} contains a symbol newer than release {release}",
                relative.display()
            ));
        }
        libraries.insert(library.to_owned(), records);
        files.push((relative, bytes));
    }
    Ok((Snapshot { release, libraries }, files))
}

fn git(source: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(source)
        .args(args)
        .output()
        .map_err(|e| format!("cannot run git: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    String::from_utf8(output.stdout)
        .map(|s| s.trim().to_owned())
        .map_err(|e| e.to_string())
}

/// The bundled file at `relative` for `release`.
#[cfg(feature = "bundled-glibc-data")]
fn bundled_file(release: Version, relative: &Path) -> Result<Vec<u8>, String> {
    let version = release.to_string();
    let path = relative.to_string_lossy().replace('\\', "/");
    glibc_abilists::abilists()
        .iter()
        .find(|a| a.release == version && a.path == path)
        .map(|a| a.text.as_bytes().to_vec())
        .ok_or_else(|| format!("glibc-abilists has no {path} for {release}"))
}

/// Releases available in the bundled glibc-abilists data.
#[cfg(feature = "bundled-glibc-data")]
pub fn bundled_releases() -> Result<Vec<Version>, String> {
    glibc_abilists::releases()
        .iter()
        .map(|r| r.version.parse())
        .collect()
}

/// Writes the bundled ABI lists into `output` in the data-directory layout
/// that [`Snapshot::load`] reads, for tools that need files on disk. The
/// directory must not exist yet. Returns the number of files written.
#[cfg(feature = "bundled-glibc-data")]
pub fn export_bundled(output: &Path) -> Result<usize, String> {
    fs::create_dir(output)
        .map_err(|e| format!("cannot create new directory {}: {e}", output.display()))?;
    let mut count = 0;
    for a in glibc_abilists::abilists() {
        let path = output.join(a.release).join(a.path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(&path, a.text).map_err(|e| format!("{}: {e}", path.display()))?;
        count += 1;
    }
    let mut provenance = String::from(
        "# ABI lists exported from the glibc-abilists crate; see its data/releases.tsv\n\
         # and data/files.tsv for each tag's commit, signature, and file hashes.\n\
         # License: LGPL-2.1-or-later (the GNU C Library's).\n\
         release\ttag\tcommit\n",
    );
    for r in glibc_abilists::releases() {
        writeln!(provenance, "{}\t{}\t{}", r.version, r.tag, r.commit).unwrap();
    }
    fs::write(output.join("SOURCE.tsv"), provenance).map_err(|e| e.to_string())?;
    // The LGPL travels with every redistributed copy of the lists.
    fs::write(output.join("COPYING.LIB"), glibc_abilists::LICENSE_TEXT)
        .map_err(|e| e.to_string())?;
    Ok(count)
}

/// Collect the eight x86_64 ABI lists from a clean, exactly tagged glibc checkout.
/// The new release directory must not already exist. All inputs are read and
/// parsed before creating output; no download or build is performed.
pub fn collect(source: &Path, output: &Path) -> Result<PathBuf, String> {
    let tag = git(
        source,
        &["describe", "--tags", "--exact-match", "--match", "glibc-*"],
    )?;
    let release: Version = tag
        .strip_prefix("glibc-")
        .ok_or("expected an official glibc release tag")?
        .parse()?;
    release.validate_release()?;
    if tag != format!("glibc-{release}") {
        return Err("expected an exact glibc-MAJOR.MINOR release tag".into());
    }
    if !git(source, &["status", "--porcelain", "--untracked-files=all"])?.is_empty() {
        return Err("source checkout must be clean before collection".into());
    }
    let commit = git(source, &["rev-parse", "HEAD"])?;
    let (_, mut files) = load_source(source, release)?;
    let source = source.canonicalize().map_err(|e| e.to_string())?;
    // Preserve the source tree's own license text alongside the collected data.
    for name in ["COPYING", "COPYING.LIB"] {
        files.push((PathBuf::from(name), read_inside(&source, Path::new(name))?));
    }
    let destination = output.join(release.to_string());
    fs::create_dir_all(output).map_err(|e| e.to_string())?;
    fs::create_dir(&destination)
        .map_err(|e| format!("cannot create new release {}: {e}", destination.display()))?;
    for (relative, bytes) in files {
        let path = destination.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(&path, bytes).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    fs::write(
        destination.join("SOURCE.txt"),
        format!("tag={tag}\ncommit={commit}\ntarget={TARGET}\n"),
    )
    .map_err(|e| e.to_string())?;
    Ok(destination)
}

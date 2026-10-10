//! Cross-release public ABI evidence. Snapshot absence is not historical absence.
use crate::{
    glibc::{self, Kind, Snapshot},
    version::Version,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write,
    fs,
    path::Path,
};

const MAX_RELEASES: usize = 64;
const MAX_ROWS: usize = 100_000;
const MAX_CELLS: usize = 2_000_000;
const MAX_NAME_BYTES: usize = 16 * 1024 * 1024;
type Key = (String, String, Version);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Expectation {
    Accept(Kind),
    Reject,
    Unknown,
}
impl Expectation {
    pub fn label(self) -> &'static str {
        match self {
            Self::Accept(_) => "accept",
            Self::Reject => "reject",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Default)]
pub struct Matrix {
    releases: BTreeSet<Version>,
    rows: BTreeMap<Key, BTreeMap<Version, Kind>>,
    name_bytes: usize,
}

fn cell(value: Option<Kind>) -> String {
    match value {
        Some(Kind::Function) => "F".into(),
        Some(Kind::Object(n)) => format!("D:{n}"),
        Some(Kind::Tls(n)) => format!("T:{n}"),
        None => "absent".into(),
    }
}

impl Matrix {
    pub fn load(root: &Path) -> Result<Self, String> {
        let mut releases = BTreeSet::new();
        for entry in fs::read_dir(root).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            if !entry.file_type().map_err(|e| e.to_string())?.is_dir() {
                continue;
            }
            let name = entry.file_name();
            let text = name.to_str().ok_or("release directory must be UTF-8")?;
            if !text.starts_with(|c: char| c.is_ascii_digit()) {
                continue;
            }
            let release: Version = text.parse()?;
            release.validate_release()?;
            if text != release.to_string() {
                return Err(format!("noncanonical release directory {text:?}"));
            }
            releases.insert(release);
            if releases.len() > MAX_RELEASES {
                return Err("matrix exceeds 64 snapshot releases".into());
            }
        }
        if releases.is_empty() {
            return Err("matrix has no release snapshots".into());
        }
        let mut matrix = Self::default();
        for release in releases {
            matrix.insert(Snapshot::load(root, release)?)?;
        }
        Ok(matrix)
    }

    pub fn from_snapshots(snapshots: impl IntoIterator<Item = Snapshot>) -> Result<Self, String> {
        let mut matrix = Self::default();
        for snapshot in snapshots {
            matrix.insert(snapshot)?;
        }
        if matrix.releases.is_empty() {
            return Err("matrix has no release snapshots".into());
        }
        Ok(matrix)
    }

    fn insert(&mut self, snapshot: Snapshot) -> Result<(), String> {
        let release = snapshot.release.validate_release()?;
        if self.releases.len() >= MAX_RELEASES || !self.releases.insert(release) {
            return Err("too many or duplicate snapshot releases".into());
        }
        if snapshot.libraries.len() != glibc::LIBRARIES.len()
            || glibc::LIBRARIES
                .iter()
                .any(|lib| !snapshot.libraries.contains_key(*lib))
        {
            return Err("matrix requires complete snapshots of the eight tracked libraries".into());
        }
        for (library, symbols) in snapshot.libraries {
            for symbol in symbols {
                if symbol.name.is_empty()
                    || symbol.name.len() > 1024
                    || !symbol.name.bytes().all(|b| b.is_ascii_graphic())
                    || symbol.version > release
                {
                    return Err("invalid matrix symbol name or future ABI version".into());
                }
                let key = (library.clone(), symbol.name, symbol.version);
                if !self.rows.contains_key(&key) {
                    self.name_bytes += key.0.len() + key.1.len();
                    if self.rows.len() >= MAX_ROWS || self.name_bytes > MAX_NAME_BYTES {
                        return Err("matrix exceeds row/name limits".into());
                    }
                }
                if self
                    .rows
                    .entry(key)
                    .or_default()
                    .insert(release, symbol.kind)
                    .is_some()
                {
                    return Err("duplicate library/symbol/version in a snapshot".into());
                }
            }
        }
        if self.rows.len() * self.releases.len() > MAX_CELLS {
            return Err("matrix exceeds two million cells".into());
        }
        Ok(())
    }

    /// Exact versioned export membership; not default binding or whole-link resolution.
    pub fn expect(
        &self,
        release: Version,
        library: &str,
        name: &str,
        version: Version,
    ) -> Expectation {
        if !self.releases.contains(&release) || !glibc::LIBRARIES.contains(&library) {
            return Expectation::Unknown;
        }
        self.rows
            .get(&(library.into(), name.into(), version))
            .and_then(|row| row.get(&release))
            .copied()
            .map_or(Expectation::Reject, Expectation::Accept)
    }

    pub fn to_tsv(&self, symbol: Option<&str>) -> String {
        let mut text = String::from("target\tlibrary\tsymbol\tversion\tfirst_observed_snapshot");
        for release in &self.releases {
            write!(text, "\t{release}").unwrap();
        }
        text.push('\n');
        for ((library, name, version), row) in &self.rows {
            if symbol.is_some_and(|s| s != name) {
                continue;
            }
            write!(
                text,
                "{}\t{library}\t{name}\tGLIBC_{version}\t{}",
                glibc::TARGET,
                row.keys().next().unwrap()
            )
            .unwrap();
            for release in &self.releases {
                write!(text, "\t{}", cell(row.get(release).copied())).unwrap();
            }
            text.push('\n');
        }
        text
    }

    pub fn diff_tsv(
        &self,
        from: Version,
        to: Version,
        symbol: Option<&str>,
    ) -> Result<String, String> {
        if !self.releases.contains(&from) || !self.releases.contains(&to) {
            return Err("diff requires two known snapshots; missing history is unknown".into());
        }
        let mut text =
            String::from("target\tfrom\tto\tchange\tlibrary\tsymbol\tversion\tbefore\tafter\n");
        for ((library, name, version), row) in &self.rows {
            if symbol.is_some_and(|s| s != name) {
                continue;
            }
            let old = row.get(&from).copied();
            let new = row.get(&to).copied();
            if old == new {
                continue;
            }
            let change = match (old, new) {
                (None, Some(_)) => "added",
                (Some(_), None) => "removed",
                _ => "changed",
            };
            writeln!(
                text,
                "{}\t{from}\t{to}\t{change}\t{library}\t{name}\tGLIBC_{version}\t{}\t{}",
                glibc::TARGET,
                cell(old),
                cell(new)
            )
            .unwrap();
        }
        Ok(text)
    }

    pub fn changes_tsv(&self) -> String {
        let releases: Vec<_> = self.releases.iter().copied().collect();
        let mut text =
            String::from("target\tfrom\tto\tchange\tlibrary\tsymbol\tversion\tbefore\tafter\n");
        for pair in releases.windows(2) {
            let diff = self.diff_tsv(pair[0], pair[1], None).unwrap();
            text.push_str(diff.split_once('\n').unwrap().1);
        }
        text
    }
}

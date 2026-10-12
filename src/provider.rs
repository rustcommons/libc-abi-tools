//! Providers: the versioned local descriptor (a trusted build configuration,
//! not a signed or redistributable SDK), and their preparation and cache in the
//! delivery modes of docs/PROVIDER-DELIVERY.md.
pub mod build;
pub mod cache;
pub mod json;
pub mod pins;
pub mod prepare;

use crate::version::Version;
use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    path::{Path, PathBuf},
};

pub const FORMAT: &str = "libc-abi-provider-v1";
/// glibc releases with a qualified provider; selection is exact, without fallback.
pub const RELEASES: &[&str] = &[
    "2.17", "2.18", "2.19", "2.20", "2.21", "2.22", "2.23", "2.24", "2.25", "2.26", "2.27", "2.28",
    "2.29", "2.30", "2.31", "2.32", "2.33", "2.34", "2.35", "2.36", "2.37", "2.38", "2.39", "2.40",
    "2.41", "2.42", "2.43", "2.44",
];
pub const SONAMES: [&str; 8] = [
    "libc.so.6",
    "libm.so.6",
    "ld-linux-x86-64.so.2",
    "libresolv.so.2",
    "libpthread.so.0",
    "libdl.so.2",
    "librt.so.1",
    "libutil.so.1",
];

#[derive(Clone, Debug)]
pub struct Provider {
    pub release: Version,
    pub sysroot: PathBuf,
    pub stubs: PathBuf,
    pub gcc_startup: PathBuf,
    pub compiler_runtime: PathBuf,
    pub descriptor: String,
}

impl Provider {
    pub fn load(path: &Path) -> Result<Self, String> {
        let mut text = String::new();
        fs::File::open(path)
            .and_then(|f| f.take(16385).read_to_string(&mut text))
            .map_err(|e| format!("cannot read provider {}: {e}", path.display()))?;
        Self::parse(&text)
    }

    pub fn parse(text: &str) -> Result<Self, String> {
        if text.len() > 16384 || text.contains('\r') || !text.ends_with('\n') {
            return Err("provider must be LF-terminated UTF-8, at most 16 KiB".into());
        }
        let mut lines = text.lines();
        if lines.next() != Some(FORMAT) {
            return Err("unsupported provider format".into());
        }
        let mut fields = BTreeMap::new();
        for line in lines {
            let (key, value) = line.split_once('\t').ok_or("invalid provider field")?;
            if ![
                "target",
                "glibc",
                "sysroot",
                "stubs",
                "gcc-startup",
                "compiler-runtime",
            ]
            .contains(&key)
                || value.is_empty()
                || value.chars().any(char::is_control)
                || fields.insert(key, value).is_some()
            {
                return Err("unknown, duplicate, empty, or invalid provider field".into());
            }
        }
        if fields.len() != 6
            || fields.get("target") != Some(&"x86_64-unknown-linux-gnu")
            || !fields.get("glibc").is_some_and(|v| RELEASES.contains(v))
        {
            return Err(format!(
                "provider v1 requires exactly six fields, x86_64 Linux and glibc {}",
                RELEASES.join(", ")
            ));
        }
        let path = |key| {
            let path = PathBuf::from(fields[key]);
            if !path.is_absolute()
                || path
                    .components()
                    .any(|c| matches!(c, std::path::Component::ParentDir))
            {
                return Err(format!(
                    "provider {key} must be an absolute path without parent components"
                ));
            }
            Ok(path)
        };
        Ok(Self {
            release: fields["glibc"].parse()?,
            sysroot: path("sysroot")?,
            stubs: path("stubs")?,
            gcc_startup: path("gcc-startup")?,
            compiler_runtime: path("compiler-runtime")?,
            descriptor: text.into(),
        })
    }

    /// Static archives linked into every executable, relative to the sysroot.
    /// glibc 2.28 merged libpthread_nonshared.a into libc_nonshared.a.
    pub fn nonshared_archives(&self) -> &'static [&'static str] {
        let merged = Version {
            major: 2,
            minor: 28,
            patch: 0,
        };
        if self.release < merged {
            &[
                "usr/lib64/libc_nonshared.a",
                "usr/lib64/libpthread_nonshared.a",
            ]
        } else {
            &["usr/lib64/libc_nonshared.a"]
        }
    }

    pub fn validate_selection(&self, requested: Version) -> Result<(), String> {
        if requested != self.release {
            return Err(format!(
                "provider targets glibc {}, but --glibc requested {requested}",
                self.release
            ));
        }
        Ok(())
    }

    pub fn validate_files(&self) -> Result<(), String> {
        for root in [
            &self.sysroot,
            &self.stubs,
            &self.gcc_startup,
            &self.compiler_runtime,
        ] {
            if !root.is_dir() {
                return Err(format!("provider directory missing: {}", root.display()));
            }
        }
        for (root, names) in [
            (
                &self.sysroot,
                &[
                    "usr/include/features.h",
                    "usr/lib64/crt1.o",
                    "usr/lib64/crti.o",
                    "usr/lib64/crtn.o",
                    "lib64/ld-linux-x86-64.so.2",
                ][..],
            ),
            (&self.sysroot, self.nonshared_archives()),
            (&self.gcc_startup, &["crtbegin.o", "crtend.o"][..]),
            (
                &self.compiler_runtime,
                &["libgcc_s.so", "libgcc_s.so.1"][..],
            ),
            (&self.stubs, &SONAMES[..]),
            (
                &self.stubs,
                &[
                    "libc.so",
                    "libm.so",
                    "libld.so",
                    "libresolv.so",
                    "libpthread.so",
                    "libdl.so",
                    "librt.so",
                    "libutil.so",
                ][..],
            ),
        ] {
            let canonical = fs::canonicalize(root).map_err(|e| e.to_string())?;
            for name in names {
                let file = fs::canonicalize(root.join(name))
                    .map_err(|e| format!("missing provider input {name}: {e}"))?;
                if !file.starts_with(&canonical) || !file.is_file() {
                    return Err(format!(
                        "provider input {name} must resolve to a file inside {}",
                        root.display()
                    ));
                }
            }
        }
        for (linker_name, soname) in [
            "libc.so",
            "libm.so",
            "libld.so",
            "libresolv.so",
            "libpthread.so",
            "libdl.so",
            "librt.so",
            "libutil.so",
        ]
        .iter()
        .zip(SONAMES)
        {
            if fs::canonicalize(self.stubs.join(linker_name)).map_err(|e| e.to_string())?
                != fs::canonicalize(self.stubs.join(soname)).map_err(|e| e.to_string())?
            {
                return Err(format!(
                    "provider linker name {linker_name} must resolve to {soname}"
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn valid() -> String {
        let root = if cfg!(windows) { "C:/sdk" } else { "/sdk" };
        format!("{FORMAT}\ntarget\tx86_64-unknown-linux-gnu\nglibc\t2.17\nsysroot\t{root}\nstubs\t{root}/stubs\ngcc-startup\t{root}/gcc\ncompiler-runtime\t{root}/runtime\n")
    }
    #[test]
    fn rejects_wrong_versions_and_ambiguous_descriptors() {
        let text = valid();
        assert!(Provider::parse(&text).is_ok());
        for bad in [
            text.replace("2.17", "2.16"),
            text.replace(FORMAT, "libc-abi-provider-v2"),
            format!("{text}glibc\t2.17\n"),
            text.replace("stubs\t", "unknown\t"),
            text.replace("/stubs", "/../stubs"),
            text.replace('\n', "\r\n"),
            text.trim_end().into(),
            "x".repeat(16385),
        ] {
            assert!(Provider::parse(&bad).is_err());
        }
        assert!(Provider::parse(&text).unwrap().validate_files().is_err());
    }

    #[test]
    fn nonshared_archives_follow_the_release() {
        for (release, count) in [("2.17", 2), ("2.27", 2), ("2.28", 1), ("2.44", 1)] {
            let provider = Provider::parse(&valid().replace("2.17", release)).unwrap();
            let archives = provider.nonshared_archives();
            assert_eq!(archives.len(), count);
            assert_eq!(archives[0], "usr/lib64/libc_nonshared.a");
        }
    }

    #[test]
    fn selects_each_supported_release_without_fallback() {
        for &release in RELEASES {
            let provider = Provider::parse(&valid().replace("2.17", release)).unwrap();
            assert_eq!(provider.release, release.parse().unwrap());
            assert!(provider
                .validate_selection(release.parse().unwrap())
                .is_ok());
            for other in RELEASES.iter().copied().chain(["2.16", "2.22.1", "2.45"]) {
                if other != release {
                    assert!(provider.validate_selection(other.parse().unwrap()).is_err());
                }
            }
        }
        assert!(Provider::parse(&valid())
            .unwrap()
            .validate_selection("2.22".parse().unwrap())
            .is_err());
        assert!(Provider::parse(&valid().replace("2.17", "2.22"))
            .unwrap()
            .validate_selection("2.17".parse().unwrap())
            .is_err());
        assert!(Provider::parse(&valid().replace("2.17", "2.22.0")).is_err());
    }
}

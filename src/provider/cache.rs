//! Prepared providers on disk:
//!
//! ```text
//! ROOT/x86_64-unknown-linux-gnu/glibc-2.28/<mode>-<content hash>/
//!     provider.txt  sysroot/  stubs/  compiler-runtime/  manifest.json
//! ```
//!
//! ROOT is `$CARGO_LIBC_PROVIDER_DIR`, else `$CARGO_HOME/libc-providers`, else
//! `~/.cargo/libc-providers`. The content hash covers every input of the
//! preparation, so changed inputs never reuse a stale provider. `manifest.json`
//! records the mode, origin, tools, and every file's SHA-256 and size.
use super::{json::Value, Provider, FORMAT, RELEASES, SONAMES};
use crate::{elf, glibc::Snapshot, libgcc, sha256, stub, version::Version};
use std::{
    collections::BTreeSet,
    env, fmt, fs,
    path::{Path, PathBuf},
};

/// The target triple every provider is for.
pub const TARGET: &str = "x86_64-unknown-linux-gnu";
/// The `format` value of a version-1 `manifest.json`.
pub const MANIFEST_FORMAT: &str = "libc-abi-provider-manifest-1";

/// How a provider's glibc objects are obtained.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Mode {
    /// Compiled locally with the host's GCC from glibc-provider-sources (default).
    Source,
    /// The CI-built objects of glibc-provider-binaries (after legal review).
    Prebuilt,
    /// Built locally by the pinned container build.
    Container,
}

impl Mode {
    /// Every mode.
    pub const ALL: [Mode; 3] = [Mode::Source, Mode::Prebuilt, Mode::Container];

    /// The mode's name as used on command lines and in directory names:
    /// `source`, `prebuilt` or `container`.
    pub fn name(self) -> &'static str {
        match self {
            Mode::Source => "source",
            Mode::Prebuilt => "prebuilt",
            Mode::Container => "container",
        }
    }
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl std::str::FromStr for Mode {
    type Err = String;
    fn from_str(text: &str) -> Result<Self, String> {
        Mode::ALL
            .into_iter()
            .find(|m| m.name() == text)
            .ok_or_else(|| {
                format!("unknown provider mode {text:?}; use source, prebuilt or container")
            })
    }
}

/// The default cache root (see the module documentation).
pub fn default_root() -> Result<PathBuf, String> {
    let root = if let Some(dir) = env::var_os("CARGO_LIBC_PROVIDER_DIR") {
        PathBuf::from(dir)
    } else if let Some(home) = env::var_os("CARGO_HOME") {
        PathBuf::from(home).join("libc-providers")
    } else if let Some(home) = env::var_os("HOME") {
        PathBuf::from(home).join(".cargo/libc-providers")
    } else {
        return Err(
            "set CARGO_LIBC_PROVIDER_DIR, CARGO_HOME or HOME for the provider cache".into(),
        );
    };
    if !root.is_absolute() {
        return Err(format!(
            "the provider cache {} must be an absolute path",
            root.display()
        ));
    }
    Ok(root)
}

/// Checks a release name against the supported list.
pub fn release(name: &str) -> Result<&'static str, String> {
    RELEASES
        .iter()
        .copied()
        .find(|r| *r == name)
        .ok_or_else(|| {
            format!(
                "glibc {name} has no provider; supported: {}",
                RELEASES.join(", ")
            )
        })
}

/// One prepared provider.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// The glibc release, such as `2.28`.
    pub release: String,
    /// How its glibc objects were obtained.
    pub mode: Mode,
    /// The content hash of its preparation inputs, in hexadecimal.
    pub hash: String,
    /// Its directory: `ROOT/x86_64-unknown-linux-gnu/glibc-RELEASE/MODE-HASH`.
    pub dir: PathBuf,
}

impl Entry {
    /// The path of its descriptor, `provider.txt` (see [`super::Provider`]). The
    /// file need not exist.
    pub fn descriptor(&self) -> PathBuf {
        self.dir.join("provider.txt")
    }

    /// Reads and parses its `manifest.json`.
    ///
    /// # Errors
    ///
    /// When the file cannot be read or is not JSON this module's parser accepts.
    pub fn manifest(&self) -> Result<Value, String> {
        let path = self.dir.join("manifest.json");
        let text = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        super::json::parse(&text).map_err(|e| format!("{}: {e}", path.display()))
    }
}

/// The directory of RELEASE's providers.
pub fn release_dir(root: &Path, release: &str) -> PathBuf {
    root.join(TARGET).join(format!("glibc-{release}"))
}

/// Where a provider with this mode and content hash lives.
pub fn entry(root: &Path, release: &str, mode: Mode, hash: &str) -> Entry {
    Entry {
        release: release.into(),
        mode,
        hash: hash.into(),
        dir: release_dir(root, release).join(format!("{mode}-{hash}")),
    }
}

/// Every prepared provider under ROOT, by release, mode and hash.
pub fn list(root: &Path) -> Result<Vec<Entry>, String> {
    let mut entries = Vec::new();
    let target = root.join(TARGET);
    if !target.is_dir() {
        return Ok(entries);
    }
    for &release in RELEASES {
        let dir = release_dir(root, release);
        let Ok(read) = fs::read_dir(&dir) else {
            continue;
        };
        for item in read {
            let item = item.map_err(|e| e.to_string())?;
            let name = item.file_name().to_string_lossy().into_owned();
            let Some((mode, hash)) = name.split_once('-') else {
                continue;
            };
            let (Ok(mode), true) = (mode.parse::<Mode>(), hex(hash)) else {
                continue;
            };
            if item.path().join("manifest.json").is_file() {
                entries.push(Entry {
                    release: release.into(),
                    mode,
                    hash: hash.into(),
                    dir: item.path(),
                });
            }
        }
    }
    entries.sort_by(|a, b| {
        let key = |e: &Entry| (e.release.parse::<Version>().ok(), e.mode, e.hash.clone());
        key(a).cmp(&key(b))
    });
    Ok(entries)
}

fn hex(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// Text that identifies a preparation's inputs; its hash names the entry.
pub fn content_hash(inputs: &[(&str, &[u8])]) -> String {
    let mut text = Vec::new();
    for (name, bytes) in inputs {
        text.extend(format!("{name}\t{}\n", bytes.len()).as_bytes());
        text.extend(*bytes);
        text.push(b'\n');
    }
    sha256::hex(&sha256::sha256(&text))[..16].to_string()
}

/// Every file and symbolic link under DIR except `provider.txt` and
/// `manifest.json`, which hold machine-specific paths and the inventory itself.
pub fn inventory(dir: &Path) -> Result<Value, String> {
    let mut items = Vec::new();
    let mut stack = vec![PathBuf::new()];
    let mut seen = BTreeSet::new();
    while let Some(relative) = stack.pop() {
        let mut names: Vec<_> = fs::read_dir(dir.join(&relative))
            .map_err(|e| format!("{}: {e}", dir.join(&relative).display()))?
            .map(|e| e.map(|e| e.file_name()))
            .collect::<Result<_, _>>()
            .map_err(|e| e.to_string())?;
        names.sort();
        for name in names {
            let path = relative.join(&name);
            let full = dir.join(&path);
            let text = path
                .to_str()
                .ok_or("provider paths must be UTF-8")?
                .replace('\\', "/");
            if relative.as_os_str().is_empty()
                && matches!(text.as_str(), "provider.txt" | "manifest.json")
            {
                continue;
            }
            let meta = fs::symlink_metadata(&full).map_err(|e| e.to_string())?;
            if meta.file_type().is_symlink() {
                let link = fs::read_link(&full).map_err(|e| e.to_string())?;
                let link = link.to_str().ok_or("link targets must be UTF-8")?;
                if Path::new(link).is_absolute() || link.contains("..") {
                    return Err(format!("{text}: links must stay in their directory"));
                }
                seen.insert(text.clone());
                items.push(Value::Object(vec![
                    ("path".into(), Value::String(text)),
                    ("link".into(), Value::String(link.into())),
                ]));
            } else if meta.is_dir() {
                stack.push(path);
            } else if meta.is_file() {
                let bytes = fs::read(&full).map_err(|e| format!("{}: {e}", full.display()))?;
                seen.insert(text.clone());
                items.push(Value::Object(vec![
                    ("path".into(), Value::String(text)),
                    (
                        "sha256".into(),
                        Value::String(sha256::hex(&sha256::sha256(&bytes))),
                    ),
                    ("size".into(), Value::Integer(bytes.len() as u64)),
                ]));
            } else {
                return Err(format!("{text}: not a file, directory or link"));
            }
        }
    }
    items.sort_by(|a, b| a.str("path").ok().cmp(&b.str("path").ok()));
    Ok(Value::Array(items))
}

/// Writes `BASE.S` and `BASE.map` into DIR and links `SONAME` from them with the
/// host's GNU as and ld, as the qualified pipeline does.
pub fn link_stub(
    tools: &super::build::Tools,
    dir: &Path,
    base: &str,
    soname: &str,
    sources: &stub::Sources,
) -> Result<(), String> {
    let (assembly, map, object) = (
        dir.join(format!("{base}.S")),
        dir.join(format!("{base}.map")),
        dir.join(format!("{base}.o")),
    );
    fs::write(&assembly, &sources.assembly).map_err(|e| e.to_string())?;
    fs::write(&map, &sources.versions).map_err(|e| e.to_string())?;
    let text = |p: &Path| p.to_string_lossy().into_owned();
    super::build::run(
        &tools.assembler,
        &["--64".into(), text(&assembly), "-o".into(), text(&object)],
        dir,
    )?;
    super::build::run(
        &tools.linker,
        &[
            "-shared".into(),
            "--build-id=none".into(),
            "-soname".into(),
            soname.into(),
            format!("--version-script={}", text(&map)),
            text(&object),
            "-o".into(),
            text(&dir.join(soname)),
        ],
        dir,
    )?;
    fs::remove_file(&object).map_err(|e| e.to_string())
}

/// A relative symbolic link NAME -> TARGET inside DIR.
pub fn symlink(dir: &Path, target: &str, name: &str) -> Result<(), String> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, dir.join(name))
            .map_err(|e| format!("{}: {e}", dir.join(name).display()))
    }
    #[cfg(not(unix))]
    {
        let _ = (dir, target, name);
        Err("providers are prepared on Linux only".into())
    }
}

/// Writes the eight glibc stubs (and their `lib*.so` names) into DIR.
pub fn write_glibc_stubs(
    tools: &super::build::Tools,
    dir: &Path,
    stubs: &[(&str, stub::Sources)],
) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    for (library, sources) in stubs {
        let soname = stub::glibc_soname(library).ok_or("unknown library")?;
        link_stub(tools, dir, library, soname, sources)?;
        symlink(dir, soname, &format!("lib{library}.so"))?;
    }
    Ok(())
}

/// Writes the `libgcc_s` stub (GCC 4.8.5's interface) into DIR.
pub fn write_libgcc_stub(tools: &super::build::Tools, dir: &Path) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    link_stub(tools, dir, "libgcc_s", libgcc::SONAME, &libgcc::stub()?)?;
    symlink(dir, libgcc::SONAME, "libgcc_s.so")
}

/// Rechecks ENTRY: every file against the manifest, the descriptor, the eight
/// stubs against the ABI lists, and the `libgcc_s` stub against GCC 4.8.5's
/// interface. Returns the descriptor.
pub fn verify(entry: &Entry, snapshot: &Snapshot) -> Result<Provider, String> {
    let manifest = entry.manifest()?;
    if manifest.str("format")? != MANIFEST_FORMAT
        || manifest.str("glibc")? != entry.release
        || manifest.str("mode")? != entry.mode.name()
        || manifest.str("hash")? != entry.hash
    {
        return Err(format!(
            "{}: manifest does not describe this entry",
            entry.dir.display()
        ));
    }
    let recorded = manifest.get("files").ok_or("manifest lacks files")?;
    let actual = inventory(&entry.dir)?;
    if *recorded != actual {
        let names = |v: &Value| -> BTreeSet<String> {
            match v {
                Value::Array(items) => items.iter().map(|i| format!("{i:?}")).collect(),
                _ => BTreeSet::new(),
            }
        };
        let changed: Vec<String> = names(recorded)
            .symmetric_difference(&names(&actual))
            .take(3)
            .cloned()
            .collect();
        return Err(format!(
            "{}: files differ from manifest.json: {changed:?}",
            entry.dir.display()
        ));
    }
    let provider = Provider::load(&entry.descriptor())?;
    provider.validate_selection(entry.release.parse()?)?;
    provider.validate_files()?;
    for path in [
        &provider.sysroot,
        &provider.stubs,
        &provider.compiler_runtime,
    ] {
        if !path.starts_with(&entry.dir) {
            return Err(format!(
                "{} must point inside {}",
                path.display(),
                entry.dir.display()
            ));
        }
    }
    if snapshot.release.to_string() != entry.release {
        return Err("the ABI snapshot is for another release".into());
    }
    for (library, soname) in crate::glibc::LIBRARIES
        .iter()
        .map(|l| (*l, stub::glibc_soname(l).expect("known")))
    {
        debug_assert!(SONAMES.contains(&soname));
        let metadata = elf::Metadata::load(&provider.stubs.join(soname))?;
        stub::render_glibc(snapshot, library, &metadata)
            .map_err(|e| format!("stub {soname}: {e}"))?;
    }
    let runtime = elf::Metadata::load(&provider.compiler_runtime.join(libgcc::SONAME))?;
    let key = |s: &elf::Symbol| {
        (
            s.name.clone(),
            s.version.clone(),
            s.default_version,
            s.symbol_type,
            s.size * u64::from(s.symbol_type != 2),
        )
    };
    let exported: BTreeSet<_> = runtime
        .symbols
        .iter()
        .filter(|s| s.section != 0 && s.version.as_deref().is_some_and(|v| v.starts_with("GCC_")))
        .map(key)
        .collect();
    let expected: BTreeSet<_> = libgcc::stub()?.selected.symbols.iter().map(key).collect();
    if exported != expected || runtime.soname != libgcc::SONAME {
        return Err(format!(
            "{}: the libgcc_s stub does not have GCC {}'s interface",
            provider.compiler_runtime.display(),
            libgcc::GCC_RELEASE
        ));
    }
    Ok(provider)
}

/// Copies DIR to a new TARGET, keeping symbolic links as links.
pub fn copy_tree(dir: &Path, target: &Path) -> Result<(), String> {
    fs::create_dir(target).map_err(|e| format!("cannot create new {}: {e}", target.display()))?;
    for item in fs::read_dir(dir).map_err(|e| e.to_string())? {
        let item = item.map_err(|e| e.to_string())?;
        let (from, to) = (item.path(), target.join(item.file_name()));
        let kind = fs::symlink_metadata(&from)
            .map_err(|e| e.to_string())?
            .file_type();
        if kind.is_symlink() {
            let link = fs::read_link(&from).map_err(|e| e.to_string())?;
            let parent = to.parent().expect("inside target");
            let name = to
                .file_name()
                .expect("named")
                .to_string_lossy()
                .into_owned();
            symlink(
                parent,
                link.to_str().ok_or("link targets must be UTF-8")?,
                &name,
            )?;
        } else if kind.is_dir() {
            copy_tree(&from, &to)?;
        } else {
            fs::copy(&from, &to).map_err(|e| format!("{}: {e}", from.display()))?;
        }
    }
    Ok(())
}

/// Removes a directory tree, making read-only directories writable first.
pub fn remove_tree(dir: &Path) -> Result<(), String> {
    if !dir.exists() {
        return Ok(());
    }
    if fs::remove_dir_all(dir).is_ok() {
        return Ok(());
    }
    let mut stack = vec![dir.to_path_buf()];
    while let Some(path) = stack.pop() {
        if let Ok(meta) = fs::symlink_metadata(&path) {
            if meta.is_dir() {
                let mut permissions = meta.permissions();
                #[allow(clippy::permissions_set_readonly_false)]
                permissions.set_readonly(false);
                let _ = fs::set_permissions(&path, permissions);
                if let Ok(read) = fs::read_dir(&path) {
                    stack.extend(read.flatten().map(|e| e.path()));
                }
            }
        }
    }
    fs::remove_dir_all(dir).map_err(|e| format!("cannot remove {}: {e}", dir.display()))
}

/// Copies ENTRY into TO (`TO/glibc-X.Y/<mode>-<hash>`) for another machine.
pub fn export(entry: &Entry, to: &Path) -> Result<PathBuf, String> {
    let target = to
        .join(format!("glibc-{}", entry.release))
        .join(entry.dir.file_name().expect("named"));
    fs::create_dir_all(target.parent().expect("has parent")).map_err(|e| e.to_string())?;
    copy_tree(&entry.dir, &target)?;
    // The descriptor's paths are this machine's; import writes new ones.
    fs::remove_file(target.join("provider.txt")).map_err(|e| e.to_string())?;
    Ok(target)
}

/// Imports every exported provider under FROM into ROOT, after checking each
/// file against its manifest. GCC_STARTUP is this machine's GCC directory.
pub fn import(from: &Path, root: &Path, gcc_startup: &Path) -> Result<Vec<Entry>, String> {
    let mut imported = Vec::new();
    for release_item in fs::read_dir(from).map_err(|e| format!("{}: {e}", from.display()))? {
        let release_path = release_item.map_err(|e| e.to_string())?.path();
        let Some(release_name) = release_path
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(|n| n.strip_prefix("glibc-"))
            .map(String::from)
        else {
            continue;
        };
        let release_name = release(&release_name)?;
        for item in fs::read_dir(&release_path).map_err(|e| e.to_string())? {
            let source = item.map_err(|e| e.to_string())?.path();
            let name = source
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or("entry names must be UTF-8")?
                .to_string();
            let Some((mode, hash)) = name.split_once('-') else {
                continue;
            };
            let mode: Mode = mode.parse()?;
            if !hex(hash) {
                return Err(format!("{}: not a provider entry", source.display()));
            }
            let staged = Entry {
                release: release_name.into(),
                mode,
                hash: hash.into(),
                dir: source.clone(),
            };
            let manifest = staged.manifest()?;
            if manifest.get("files") != Some(&inventory(&source)?) {
                return Err(format!(
                    "{}: files differ from manifest.json",
                    source.display()
                ));
            }
            let target = entry(root, release_name, mode, hash);
            if target.dir.exists() {
                imported.push(target);
                continue;
            }
            let parent = target.dir.parent().expect("has parent");
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            let temporary = parent.join(format!(".tmp-import-{}-{name}", std::process::id()));
            remove_tree(&temporary)?;
            copy_tree(&source, &temporary)?;
            // The descriptor names the final directory.
            let text = descriptor_text(&target.dir, release_name, gcc_startup)?;
            fs::write(temporary.join("provider.txt"), text).map_err(|e| e.to_string())?;
            fs::rename(&temporary, &target.dir).map_err(|e| e.to_string())?;
            imported.push(target);
        }
    }
    Ok(imported)
}

/// The text of a descriptor for an entry directory DIR.
pub fn descriptor_text(dir: &Path, release: &str, gcc_startup: &Path) -> Result<String, String> {
    let mut text = format!("{FORMAT}\ntarget\t{TARGET}\nglibc\t{release}\n");
    for (key, path) in [
        ("sysroot", dir.join("sysroot")),
        ("stubs", dir.join("stubs")),
        ("gcc-startup", gcc_startup.to_path_buf()),
        ("compiler-runtime", dir.join("compiler-runtime")),
    ] {
        let path = path.to_str().ok_or("provider paths must be UTF-8")?;
        text.push_str(&format!("{key}\t{path}\n"));
    }
    Provider::parse(&text)?;
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_entries_by_release_mode_and_hash() {
        let root = Path::new("/cache");
        let e = entry(root, "2.28", Mode::Source, "0123456789abcdef");
        assert_eq!(
            e.dir,
            Path::new("/cache/x86_64-unknown-linux-gnu/glibc-2.28/source-0123456789abcdef")
        );
        assert_eq!(e.descriptor(), e.dir.join("provider.txt"));
        assert_eq!("container".parse::<Mode>().unwrap(), Mode::Container);
        assert!("binary".parse::<Mode>().is_err());
        assert!(release("2.16").is_err());
        assert_eq!(release("2.44").unwrap(), "2.44");
    }

    #[test]
    fn hashes_every_input() {
        let a = content_hash(&[("x", b"1"), ("y", b"2")]);
        assert_eq!(a.len(), 16);
        assert_ne!(a, content_hash(&[("x", b"1"), ("y", b"3")]));
        // Boundaries between inputs are part of the hash.
        assert_ne!(
            content_hash(&[("x", b"12"), ("y", b"")]),
            content_hash(&[("x", b"1"), ("y", b"2")])
        );
    }

    #[test]
    fn inventories_files_and_rejects_changes() {
        let dir = env::temp_dir().join(format!("lat-cache-test-{}", std::process::id()));
        let _ = remove_tree(&dir);
        fs::create_dir_all(dir.join("stubs")).unwrap();
        fs::write(dir.join("stubs/a"), b"abc").unwrap();
        fs::write(dir.join("provider.txt"), b"machine specific").unwrap();
        let first = inventory(&dir).unwrap();
        let text = first.to_text();
        assert!(text.contains("\"stubs/a\""), "{text}");
        assert!(text.contains(&sha256::hex(&sha256::sha256(b"abc"))));
        assert!(!text.contains("provider.txt"));
        fs::write(dir.join("provider.txt"), b"another machine").unwrap();
        assert_eq!(inventory(&dir).unwrap(), first);
        fs::write(dir.join("stubs/a"), b"abd").unwrap();
        assert_ne!(inventory(&dir).unwrap(), first);
        remove_tree(&dir).unwrap();
    }
}

//! Prepares a provider into the cache in one of the delivery modes. Every mode
//! produces the same layout; see docs/PROVIDER-DELIVERY.md.
use super::{
    build::Tools,
    cache::{self, Entry, Mode},
    json::Value,
    pins,
};
use crate::{glibc::Snapshot, libgcc, sha256, stub};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

/// Options shared by every mode.
#[derive(Clone, Debug)]
pub struct Options {
    /// The cache root ([`cache::default_root`]).
    pub root: PathBuf,
    pub tools: Tools,
    /// Prepare again even if the cache has this provider.
    pub force: bool,
    /// `container`: podman or docker; found on PATH when `None`.
    pub engine: Option<PathBuf>,
    /// `container`: a directory of previously fetched `glibc-X.Y.tar.xz`
    /// archives, used before downloading (also `ABI_GLIBC_SOURCE_DIR`).
    pub archives: Option<PathBuf>,
    /// `container`: move the build's work tree (sources, the installed glibc,
    /// logs) here instead of deleting it. The directory must not exist.
    pub keep_work: Option<PathBuf>,
}

impl Options {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            tools: Tools::default(),
            force: false,
            engine: None,
            archives: None,
            keep_work: None,
        }
    }
}

/// A prepared provider.
#[derive(Clone, Debug)]
pub struct Prepared {
    pub entry: Entry,
    /// Whether the cache already had it.
    pub reused: bool,
    /// What the preparation found worth reporting.
    pub notes: Vec<String>,
}

/// Prepares RELEASE in MODE, or returns the cached provider with the same inputs.
/// SNAPSHOT is the release's ABI lists, from which the stubs are generated.
pub fn prepare(
    release: &str,
    mode: Mode,
    snapshot: &Snapshot,
    options: &Options,
) -> Result<Prepared, String> {
    let release = cache::release(release)?;
    if snapshot.release.to_string() != release {
        return Err("the ABI snapshot is for another release".into());
    }
    if !cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        return Err("providers are prepared on x86-64 Linux".into());
    }
    match mode {
        Mode::Source => source(release, snapshot, options),
        Mode::Prebuilt => Err(
            "prebuilt mode is not available yet: shipping CI-built glibc objects awaits \
             a legal review; use source (the default) or container"
                .into(),
        ),
        Mode::Container => container(release, snapshot, options),
    }
}

/// The content hash RELEASE would be prepared under in MODE now, without
/// preparing it; `None` if MODE is unavailable.
pub fn current_hash(
    release: &str,
    mode: Mode,
    snapshot: &Snapshot,
    tools: &Tools,
) -> Result<Option<String>, String> {
    let host = Host::probe(tools)?;
    match mode {
        Mode::Source => source_inputs(release, snapshot, &host),
        Mode::Prebuilt => Ok(None),
        Mode::Container => Ok(Some(container_hash(release, snapshot, &host)?)),
    }
}

/// The host toolchain facts every mode depends on.
struct Host {
    versions: Vec<(&'static str, String)>,
    gcc_major: u32,
    startup: PathBuf,
    startup_hashes: String,
}

impl Host {
    fn probe(tools: &Tools) -> Result<Self, String> {
        let versions = tools.versions()?;
        let startup = tools.gcc_startup()?;
        let mut startup_hashes = String::new();
        for name in ["crtbegin.o", "crtend.o"] {
            let bytes = fs::read(startup.join(name)).map_err(|e| e.to_string())?;
            startup_hashes.push_str(&format!(
                "{name} {}\n",
                sha256::hex(&sha256::sha256(&bytes))
            ));
        }
        Ok(Self {
            versions,
            gcc_major: tools.gcc_major()?,
            startup,
            startup_hashes,
        })
    }

    fn inputs(&self) -> Vec<(String, Vec<u8>)> {
        let tools: String = self
            .versions
            .iter()
            .map(|(name, version)| format!("{name} {version}\n"))
            .collect();
        vec![
            ("tools".into(), tools.into_bytes()),
            (
                "gcc-startup".into(),
                format!("{}\n{}", self.startup.display(), self.startup_hashes).into_bytes(),
            ),
        ]
    }

    fn manifest(&self) -> Vec<(String, Value)> {
        vec![
            (
                "tools".into(),
                Value::Object(
                    self.versions
                        .iter()
                        .map(|(name, version)| (name.to_string(), Value::String(version.clone())))
                        .collect(),
                ),
            ),
            ("gcc_major".into(), Value::Integer(self.gcc_major.into())),
            (
                "gcc_startup".into(),
                Value::String(self.startup.to_string_lossy().into_owned()),
            ),
        ]
    }
}

fn common_inputs(release: &str, mode: Mode, snapshot: &Snapshot) -> Vec<(String, Vec<u8>)> {
    vec![
        ("libc-abi-tools".into(), env!("CARGO_PKG_VERSION").into()),
        ("release".into(), release.into()),
        ("mode".into(), mode.name().into()),
        ("abi-lists".into(), snapshot.to_tsv(None).into_bytes()),
        ("libgcc".into(), libgcc::INTERFACE.into()),
    ]
}

fn hash_of(inputs: &[(String, Vec<u8>)]) -> String {
    let borrowed: Vec<(&str, &[u8])> = inputs
        .iter()
        .map(|(name, bytes)| (name.as_str(), bytes.as_slice()))
        .collect();
    cache::content_hash(&borrowed)
}

/// A new temporary directory next to ENTRY, removed again on failure.
struct Stage {
    dir: PathBuf,
}

impl Stage {
    fn new(entry: &Entry) -> Result<Self, String> {
        let parent = entry.dir.parent().expect("entries have a parent");
        fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        let name = entry.dir.file_name().expect("named").to_string_lossy();
        let dir = parent.join(format!(".tmp-{name}-{}", std::process::id()));
        cache::remove_tree(&dir)?;
        fs::create_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        Ok(Self { dir })
    }

    /// Writes the manifest and descriptor and moves the stage to ENTRY.
    fn finish(
        self,
        entry: &Entry,
        startup: &Path,
        mut members: Vec<(String, Value)>,
        force: bool,
    ) -> Result<(), String> {
        let mut manifest = vec![
            (
                "format".into(),
                Value::String(cache::MANIFEST_FORMAT.into()),
            ),
            ("target".into(), Value::String(cache::TARGET.into())),
            ("glibc".into(), Value::String(entry.release.clone())),
            ("mode".into(), Value::String(entry.mode.name().into())),
            ("hash".into(), Value::String(entry.hash.clone())),
            (
                "libc_abi_tools".into(),
                Value::String(env!("CARGO_PKG_VERSION").into()),
            ),
        ];
        manifest.append(&mut members);
        manifest.push(("files".into(), cache::inventory(&self.dir)?));
        fs::write(
            self.dir.join("manifest.json"),
            Value::Object(manifest).to_text(),
        )
        .map_err(|e| e.to_string())?;
        let text = cache::descriptor_text(&entry.dir, &entry.release, startup)?;
        fs::write(self.dir.join("provider.txt"), text).map_err(|e| e.to_string())?;
        if entry.dir.exists() {
            if !force {
                // Prepared concurrently with the same inputs; keep the first.
                return Ok(());
            }
            cache::remove_tree(&entry.dir)?;
        }
        fs::rename(&self.dir, &entry.dir).map_err(|e| {
            format!(
                "cannot move {} to {}: {e}",
                self.dir.display(),
                entry.dir.display()
            )
        })
    }
}

impl Drop for Stage {
    fn drop(&mut self) {
        let _ = cache::remove_tree(&self.dir);
    }
}

fn reuse(entry: &Entry, force: bool) -> Option<Prepared> {
    (!force && entry.dir.join("manifest.json").is_file()).then(|| Prepared {
        entry: entry.clone(),
        reused: true,
        notes: Vec::new(),
    })
}

/// Writes the stubs, the loader's link stub and the `libgcc_s` stub.
fn write_stubs(stage: &Path, tools: &Tools, stubs: &[(&str, stub::Sources)]) -> Result<(), String> {
    cache::write_glibc_stubs(tools, &stage.join("stubs"), stubs)?;
    cache::write_libgcc_stub(tools, &stage.join("compiler-runtime"))
}

fn copy(from: &Path, to: &Path) -> Result<(), String> {
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::copy(from, to)
        .map(|_| ())
        .map_err(|e| format!("cannot copy {}: {e}", from.display()))
}

#[cfg(feature = "provider-sources")]
fn source_inputs(
    release: &str,
    snapshot: &Snapshot,
    host: &Host,
) -> Result<Option<String>, String> {
    let sources = glibc_provider_sources::Sources::bundled()?;
    let data = sources.release(release)?;
    let mut inputs = common_inputs(release, Mode::Source, snapshot);
    for table in glibc_provider_sources::TABLES {
        inputs.push((table.into(), data.table(table)?.as_bytes().to_vec()));
    }
    inputs.push((
        "accepted-differences".into(),
        glibc_provider_sources::ACCEPTED_DIFFERENCES.into(),
    ));
    inputs.extend(host.inputs());
    Ok(Some(hash_of(&inputs)))
}

#[cfg(not(feature = "provider-sources"))]
fn source_inputs(_: &str, _: &Snapshot, _: &Host) -> Result<Option<String>, String> {
    Err(NO_SOURCES.into())
}

#[cfg(not(feature = "provider-sources"))]
const NO_SOURCES: &str =
    "built without the provider-sources feature, so source mode is unavailable; use container";

#[cfg(not(feature = "provider-sources"))]
fn source(_: &str, _: &Snapshot, _: &Options) -> Result<Prepared, String> {
    Err(NO_SOURCES.into())
}

/// `source` mode: compile the recorded glibc sources with the host's GCC and
/// verify every object against the container build's recorded interface.
#[cfg(feature = "provider-sources")]
fn source(release: &str, snapshot: &Snapshot, options: &Options) -> Result<Prepared, String> {
    let tools = &options.tools;
    let host = Host::probe(tools)?;
    let hash = source_inputs(release, snapshot, &host)?;
    let entry = cache::entry(&options.root, release, Mode::Source, &hash.expect("source"));
    if let Some(prepared) = reuse(&entry, options.force) {
        return Ok(prepared);
    }
    let sources = glibc_provider_sources::Sources::bundled()?;
    let data = sources.release(release)?;
    let stage = Stage::new(&entry)?;
    let work = stage.dir.join("work");
    fs::create_dir(&work).map_err(|e| e.to_string())?;
    let inputs = work.join("inputs");
    data.write_sources(&inputs)?;
    let values = data
        .table("release.tsv")?
        .lines()
        .skip(1)
        .filter_map(|line| line.split_once('\t'))
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    let objects = data
        .objects()?
        .into_iter()
        .map(|o| super::build::Object {
            target: o.target.into(),
            member: o.member,
            object: o.object.into(),
            kind: o.kind.into(),
            directory: o.directory.into(),
            command: o.command.into(),
        })
        .collect();
    let recorded = super::build::Recorded {
        release,
        values,
        objects,
        interfaces: data.table("interfaces.tsv")?,
        accepted: glibc_provider_sources::ACCEPTED_DIFFERENCES,
    };
    let built = super::build::build(&recorded, &inputs, tools)?;
    let mut report = String::from("object\tstatus\tdetail\n");
    for verdict in &built.verdicts {
        report.push_str(&format!(
            "{}\t{}\t{}\n",
            verdict.object,
            verdict.status,
            verdict.detail.replace(['\t', '\n'], " ")
        ));
    }
    if !built.verified() {
        let failed = entry
            .dir
            .with_file_name(format!("source-{}.differences.tsv", entry.hash));
        let _ = fs::write(&failed, &report);
        let differing: Vec<&str> = built
            .verdicts
            .iter()
            .filter(|v| v.status == "differs")
            .map(|v| v.object.as_str())
            .collect();
        return Err(format!(
            "glibc {release}: GCC {} built objects that differ from the qualified container \
             build ({}; details in {}). Not using them: use --mode container, or a GCC in the \
             qualified range {}-{}",
            host.gcc_major,
            differing.join(", "),
            failed.display(),
            super::build::QUALIFIED_GCC.start(),
            super::build::QUALIFIED_GCC.end()
        ));
    }
    let sysroot = stage.dir.join("sysroot");
    fs::create_dir_all(sysroot.join("usr")).map_err(|e| e.to_string())?;
    data.write_headers(&sysroot.join("usr/include"))?;
    for (target, path) in &built.targets {
        copy(path, &sysroot.join("usr/lib64").join(target))?;
    }
    let table = data.table("libraries.tsv")?;
    let stubs = stub::glibc_from_table(snapshot, table)?;
    write_stubs(&stage.dir, tools, &stubs)?;
    fs::write(stage.dir.join("libraries.tsv"), table).map_err(|e| e.to_string())?;
    // No real loader exists in this mode; the descriptor's sysroot holds the
    // loader's link stub, which is never run.
    copy(
        &stage.dir.join("stubs/ld-linux-x86-64.so.2"),
        &sysroot.join("lib64/ld-linux-x86-64.so.2"),
    )?;
    fs::write(stage.dir.join("verification.tsv"), &report).map_err(|e| e.to_string())?;
    cache::remove_tree(&work)?;
    let mut notes = vec![format!(
        "verified every object against the container build ({})",
        built.summary()
    )];
    if !super::build::QUALIFIED_GCC.contains(&host.gcc_major) {
        notes.push(format!(
            "GCC {} is outside the range qualified in CI ({}-{}); the local verification passed",
            host.gcc_major,
            super::build::QUALIFIED_GCC.start(),
            super::build::QUALIFIED_GCC.end()
        ));
    }
    let mut members = vec![(
        "origin".into(),
        Value::Object(vec![
            (
                "data".into(),
                Value::String("glibc-provider-sources".into()),
            ),
            (
                "archive".into(),
                Value::String(data.value("archive").unwrap_or("").into()),
            ),
            (
                "archive_sha256".into(),
                Value::String(data.value("archive_sha256").unwrap_or("").into()),
            ),
            (
                "loader".into(),
                Value::String("sysroot/lib64/ld-linux-x86-64.so.2 is the link stub".into()),
            ),
        ]),
    )];
    members.extend(host.manifest());
    members.push((
        "gcc_qualified".into(),
        Value::Bool(super::build::QUALIFIED_GCC.contains(&host.gcc_major)),
    ));
    members.push(("verification".into(), Value::String(built.summary())));
    stage.finish(&entry, &host.startup, members, options.force)?;
    Ok(Prepared {
        entry,
        reused: false,
        notes,
    })
}

fn container_hash(release: &str, snapshot: &Snapshot, host: &Host) -> Result<String, String> {
    let pins = pins::pins(release)?;
    let mut inputs = common_inputs(release, Mode::Container, snapshot);
    inputs.push(("archive".into(), pins.archive_sha256()?.into()));
    inputs.push(("image".into(), pins.image()?.into()));
    inputs.push(("script".into(), pins.script.into()));
    inputs.extend(host.inputs());
    Ok(hash_of(&inputs))
}

/// podman, else docker, from PATH.
fn find_engine() -> Result<PathBuf, String> {
    for engine in ["podman", "docker"] {
        let found = Command::new(engine)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success());
        if found {
            return Ok(engine.into());
        }
    }
    Err("container mode needs podman or docker on PATH (or --engine)".into())
}

/// Runs a command, sending its output to LOG.
fn logged(command: &mut Command, log: &Path) -> Result<(), String> {
    let file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log)
        .map_err(|e| e.to_string())?;
    let error = file.try_clone().map_err(|e| e.to_string())?;
    let status = command
        .stdout(file)
        .stderr(error)
        .status()
        .map_err(|e| format!("cannot run {:?}: {e}", command.get_program()))?;
    if !status.success() {
        return Err(format!(
            "{:?} failed; see {}",
            command.get_program(),
            log.display()
        ));
    }
    Ok(())
}

/// Places `glibc-X.Y.tar.xz` in WORK, from ARCHIVES or the GNU mirrors, and
/// checks its pinned SHA-256.
fn fetch(
    pins: &pins::Pins,
    work: &Path,
    archives: Option<&Path>,
    log: &Path,
) -> Result<(), String> {
    let name = pins.archive();
    let target = work.join(&name);
    let local = archives
        .map(Path::to_path_buf)
        .or_else(|| std::env::var_os("ABI_GLIBC_SOURCE_DIR").map(PathBuf::from))
        .map(|dir| dir.join(&name))
        .filter(|path| path.is_file());
    if let Some(path) = local {
        copy(&path, &target)?;
    } else {
        let mut fetched = false;
        for base in [
            "https://ftp.gnu.org/gnu/glibc",
            "https://mirrors.kernel.org/gnu/glibc",
        ] {
            let mut curl = Command::new("curl");
            curl.args(["-fL", "--retry", "2", "--connect-timeout", "30", "-o"])
                .arg(&target)
                .arg(format!("{base}/{name}"));
            if logged(&mut curl, log).is_ok() {
                fetched = true;
                break;
            }
            let _ = fs::remove_file(&target);
        }
        if !fetched {
            return Err(format!("cannot download {name}; see {}", log.display()));
        }
    }
    let bytes = fs::read(&target).map_err(|e| e.to_string())?;
    if sha256::hex(&sha256::sha256(&bytes)) != pins.archive_sha256()? {
        return Err(format!("{name} does not match its pinned SHA-256"));
    }
    Ok(())
}

/// `container` mode: run the pinned container build that CI runs, and take the
/// provider's glibc files from its installed tree.
fn container(release: &str, snapshot: &Snapshot, options: &Options) -> Result<Prepared, String> {
    let tools = &options.tools;
    let host = Host::probe(tools)?;
    let pins = pins::pins(release)?;
    let hash = container_hash(release, snapshot, &host)?;
    let entry = cache::entry(&options.root, release, Mode::Container, &hash);
    if let Some(keep) = &options.keep_work {
        if keep.exists() {
            return Err(format!("--keep-work {} already exists", keep.display()));
        }
    }
    if let Some(prepared) = reuse(&entry, options.force) {
        if options.keep_work.is_some() {
            return Err(format!(
                "the glibc {release} container provider is cached, so there is no build to keep; add --force"
            ));
        }
        return Ok(prepared);
    }
    let engine = match &options.engine {
        Some(engine) => engine.clone(),
        None => find_engine()?,
    };
    let stage = Stage::new(&entry)?;
    let work = stage.dir.join("work");
    fs::create_dir(&work).map_err(|e| e.to_string())?;
    let log = entry.dir.with_file_name(format!("container-{hash}.log"));
    let _ = fs::remove_file(&log);
    fetch(pins, &work, options.archives.as_deref(), &log)?;
    fs::write(work.join("build-inputs.sh"), pins.script).map_err(|e| e.to_string())?;
    let image = pins.image()?;
    logged(Command::new(&engine).args(["pull", image]), &log)?;
    let mount = format!("{}:/work", work.to_str().ok_or("paths must be UTF-8")?);
    let built = logged(
        Command::new(&engine).args([
            "run",
            "--rm",
            "-v",
            &mount,
            image,
            "bash",
            "/work/build-inputs.sh",
        ]),
        &log,
    );
    // Docker leaves the build's files owned by root; hand them back.
    let docker = engine
        .file_name()
        .is_some_and(|n| n.to_string_lossy().contains("docker"));
    if docker {
        let id = |flag: &str| -> Result<String, String> {
            let out = Command::new("id")
                .arg(flag)
                .output()
                .map_err(|e| e.to_string())?;
            Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
        };
        let owner = format!("{}:{}", id("-u")?, id("-g")?);
        logged(
            Command::new(&engine).args([
                "run", "--rm", "-v", &mount, image, "chown", "-R", &owner, "/work",
            ]),
            &log,
        )?;
    }
    built.map_err(|e| format!("the glibc {release} container build failed: {e}"))?;
    let installed = work.join("source");
    let sysroot = stage.dir.join("sysroot");
    fs::create_dir_all(sysroot.join("usr")).map_err(|e| e.to_string())?;
    cache::copy_tree(&installed.join("usr/include"), &sysroot.join("usr/include"))?;
    let mut targets = vec!["crt1.o", "crti.o", "crtn.o", "libc_nonshared.a"];
    if installed.join("usr/lib64/libpthread_nonshared.a").is_file() {
        targets.push("libpthread_nonshared.a");
    }
    for target in targets {
        let path = format!("usr/lib64/{target}");
        copy(&installed.join(&path), &sysroot.join(&path))?;
    }
    // The real libraries, with the loader: a runtime for tests.
    cache::copy_tree(&installed.join("lib64"), &sysroot.join("lib64"))?;
    let table = stub::glibc_table(snapshot, &installed.join("lib64"))?;
    let stubs = stub::glibc_from_table(snapshot, &table)?;
    write_stubs(&stage.dir, tools, &stubs)?;
    fs::write(stage.dir.join("libraries.tsv"), &table).map_err(|e| e.to_string())?;
    // The builder's own compiler runtime, to run tests with (never to link).
    for dir in ["prebuilt/usr/lib64", "prebuilt/lib64"] {
        let runtime = work.join(dir).join(libgcc::SONAME);
        if runtime.exists() {
            copy(
                &runtime,
                &stage.dir.join("run-runtime").join(libgcc::SONAME),
            )?;
            break;
        }
    }
    match &options.keep_work {
        Some(keep) => {
            if fs::rename(&work, keep).is_err() {
                cache::copy_tree(&work, keep)?;
                cache::remove_tree(&work)?;
            }
        }
        None => cache::remove_tree(&work)?,
    }
    let mut members = vec![(
        "origin".into(),
        Value::Object(vec![
            ("archive".into(), Value::String(pins.archive())),
            (
                "archive_sha256".into(),
                Value::String(pins.archive_sha256()?.into()),
            ),
            ("image".into(), Value::String(image.into())),
            (
                "engine".into(),
                Value::String(engine.to_string_lossy().into_owned()),
            ),
        ]),
    )];
    members.extend(host.manifest());
    stage.finish(&entry, &host.startup, members, options.force)?;
    let _ = fs::remove_file(&log);
    Ok(Prepared {
        entry,
        reused: false,
        notes: vec![format!("built by the pinned container build in {image}")],
    })
}

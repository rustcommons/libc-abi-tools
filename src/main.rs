use libc_abi_tools::{elf, glibc, libgcc, matrix::Matrix, stub, version::Version};
use std::{
    ffi::OsString,
    io::{self, Write},
    path::PathBuf,
};

const HELP: &str = "libc-abi - Rust libc ABI metadata tools (glibc first)

USAGE
  libc-abi glibc snapshot [--data DIR] --release VERSION [--symbol NAME]
  libc-abi glibc collect --source GLIBC_CHECKOUT --output DIR
  libc-abi glibc export-data --output DIR
  libc-abi glibc elf --input SHARED_OBJECT
  libc-abi glibc libraries [--data DIR] --release VERSION --input LIB64
  libc-abi glibc matrix [--data DIR] [--symbol NAME]
  libc-abi glibc diff [--data DIR] --from VERSION --to VERSION [--symbol NAME]
  libc-abi glibc changes [--data DIR]
  libc-abi glibc expect [--data DIR] --release VERSION --library NAME --symbol NAME --version VERSION
  libc-abi libgcc stub --output DIR
  libc-abi provider prepare (--glibc VERSION | --all) [--mode MODE] [OPTIONS]
  libc-abi provider list
  libc-abi provider path --glibc VERSION [--mode MODE]
  libc-abi provider verify --glibc VERSION [--mode MODE]
  libc-abi provider export --glibc VERSION [--mode MODE] --to DIR
  libc-abi provider import DIR
  libc-abi provider remove (--glibc VERSION | --all) [--mode MODE]
  libc-abi --version

The glibc ABI lists come from the GNU C Library's release tags through the
glibc-abilists crate (LGPL-2.1-or-later), compiled in by default. --data DIR
reads them from a directory instead; DIR contains version directories such as
2.17/ and 2.22/ (the layout of glibc-abilists' data/ and of export-data).
Built without the bundled-glibc-data feature, --data is required.

snapshot writes stable TSV for one exact x86_64 Linux glibc release.
export-data writes the bundled ABI lists into a new directory.
libraries writes, from a release's real installed libraries (LIB64 holds
libc.so.6 and the seven others), the public definitions of all eight that its
link stubs are generated from, each checked against the ABI lists. Providers
built without the real libraries generate their stubs from this table.
collect requires a clean checkout at an exact glibc release tag; no overwrites.
elf writes defined dynamic-symbol evidence from ELF64 x86_64, including SONAME.
An ELF file does not establish its source release or semantic alias relationships.
Target releases start at 2.17; older symbol versions are retained.
matrix compares all available snapshots; first observation is not introduction.
expect prints accept/reject/unknown for exact library/symbol/version membership.
Missing release/library evidence is unknown, not a rejection or fallback.
The glibc commands generate no stub libraries, sysroots, headers, or runtime
implementations.

libgcc stub writes, into a new directory, the assembly (libgcc_s.S) and version
script (libgcc_s.map) of a link-only libgcc_s.so.1 with the interface of GCC
4.8.5's compiler runtime, the one every provider links against:
  as --64 libgcc_s.S -o libgcc_s.o
  ld -shared --build-id=none -soname libgcc_s.so.1 --version-script=libgcc_s.map \\
    libgcc_s.o -o libgcc_s.so.1
Its functions trap: link against it, but run with a real libgcc_s.so.1.

provider prepares and manages glibc link providers for x86_64 Linux, glibc 2.17
through 2.44, in a cache: $CARGO_LIBC_PROVIDER_DIR, else $CARGO_HOME/libc-providers,
else ~/.cargo/libc-providers (or --cache DIR). Each provider has a descriptor
(provider.txt) for cargo-libc's --provider. MODE is one of:
  source     (default) compile the recorded glibc sources of the
             glibc-provider-sources crate with the host's GCC, offline, and verify
             every object against the qualified container build before use
  container  run the pinned container build CI runs (podman or docker, network,
             several minutes per release)
  prebuilt   CI-built objects; not available until its legal review
prepare reuses a cached provider with the same inputs unless --force. Options:
  --cc GCC          the GCC to compile with and take crtbegin.o/crtend.o from
  --engine PATH     container engine (default: podman, else docker)
  --archives DIR    previously fetched glibc-X.Y.tar.xz archives (container)
  --data DIR        ABI lists to generate stubs from instead of the bundled ones
list prints every cached provider. path prints the descriptor prepare would use
(source, else container, without --mode). verify rechecks every file against the
manifest and the stubs against the ABI lists. export copies providers to DIR for
another machine; import adds them, with this machine's GCC startup files.
";

fn run(args: Vec<OsString>) -> Result<(), String> {
    if args.is_empty() || matches!(args[0].to_str(), Some("--help" | "-h" | "help")) {
        print!("{HELP}");
        return Ok(());
    }
    if args.len() == 1 && args[0] == "--version" {
        println!("libc-abi {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if args[0] == "libgcc" {
        return libgcc_command(&args[1..]);
    }
    if args[0] == "provider" {
        return provider_command(&args[1..]);
    }
    if args.len() < 2 || args[0] != "glibc" {
        return Err("expected glibc snapshot, glibc collect, or glibc elf; see --help".into());
    }
    let command = args[1].to_str().ok_or("command must be UTF-8")?;
    let allowed: &[&str] = match command {
        "snapshot" => &["--data", "--release", "--symbol"],
        "collect" => &["--source", "--output"],
        "export-data" => &["--output"],
        "elf" => &["--input"],
        "libraries" => &["--data", "--release", "--input"],
        "matrix" => &["--data", "--symbol"],
        "diff" => &["--data", "--from", "--to", "--symbol"],
        "changes" => &["--data"],
        "expect" => &["--data", "--release", "--library", "--symbol", "--version"],
        _ => return Err(format!("unknown glibc command {command:?}")),
    };
    let mut options = std::collections::BTreeMap::new();
    let mut i = 2;
    while i < args.len() {
        if args[i] == "--help" {
            print!("{HELP}");
            return Ok(());
        }
        let flag = args[i].to_str().ok_or("option must be UTF-8")?;
        if !allowed.contains(&flag) {
            return Err(format!("unknown option {flag:?}"));
        }
        let value = args
            .get(i + 1)
            .ok_or_else(|| format!("{flag} needs a value"))?;
        if value.is_empty() || value.to_str().is_some_and(|s| s.starts_with("--")) {
            return Err(format!("{flag} needs a nonempty value"));
        }
        if options.insert(flag, value).is_some() {
            return Err(format!("duplicate option {flag}"));
        }
        i += 2;
    }
    let required = |flag| {
        options
            .get(flag)
            .copied()
            .ok_or_else(|| format!("{flag} is required"))
    };
    match command {
        "matrix" | "diff" | "changes" | "expect" => {
            let matrix = match options.get("--data") {
                Some(dir) => Matrix::load(&PathBuf::from(dir))?,
                None => bundled_matrix()?,
            };
            let value = |flag| {
                required(flag)?
                    .to_str()
                    .ok_or_else(|| format!("{flag} must be UTF-8"))
            };
            let symbol = options
                .get("--symbol")
                .map(|v| v.to_str().ok_or("symbol must be UTF-8"))
                .transpose()?;
            let text = match command {
                "matrix" => matrix.to_tsv(symbol),
                "diff" => {
                    matrix.diff_tsv(value("--from")?.parse()?, value("--to")?.parse()?, symbol)?
                }
                "changes" => matrix.changes_tsv(),
                _ => format!(
                    "{}\n",
                    matrix
                        .expect(
                            value("--release")?.parse()?,
                            value("--library")?,
                            value("--symbol")?,
                            value("--version")?.parse()?
                        )
                        .label()
                ),
            };
            if let Err(error) = io::stdout().lock().write_all(text.as_bytes()) {
                if error.kind() != io::ErrorKind::BrokenPipe {
                    return Err(error.to_string());
                }
            }
        }
        "elf" => {
            let text = elf::Metadata::load(&PathBuf::from(required("--input")?))?.to_tsv();
            match io::stdout().lock().write_all(text.as_bytes()) {
                Err(error) if error.kind() != io::ErrorKind::BrokenPipe => {
                    return Err(error.to_string())
                }
                _ => {}
            }
        }
        "libraries" => {
            let release: Version = required("--release")?
                .to_str()
                .ok_or("release must be UTF-8")?
                .parse()?;
            let snapshot = match options.get("--data") {
                Some(dir) => glibc::Snapshot::load(&PathBuf::from(dir), release)?,
                None => bundled_snapshot(release)?,
            };
            let text = stub::glibc_table(&snapshot, &PathBuf::from(required("--input")?))?;
            match io::stdout().lock().write_all(text.as_bytes()) {
                Err(error) if error.kind() != io::ErrorKind::BrokenPipe => {
                    return Err(error.to_string())
                }
                _ => {}
            }
        }
        "snapshot" => {
            let release: Version = required("--release")?
                .to_str()
                .ok_or("release must be UTF-8")?
                .parse()?;
            let symbol = options
                .get("--symbol")
                .map(|s| s.to_str().ok_or("symbol must be UTF-8"))
                .transpose()?;
            let snapshot = match options.get("--data") {
                Some(dir) => glibc::Snapshot::load(&PathBuf::from(dir), release)?,
                None => bundled_snapshot(release)?,
            };
            let text = snapshot.to_tsv(symbol);
            match io::stdout().lock().write_all(text.as_bytes()) {
                Err(error) if error.kind() != io::ErrorKind::BrokenPipe => {
                    return Err(error.to_string())
                }
                _ => {}
            }
        }
        "collect" => {
            let destination = glibc::collect(
                &PathBuf::from(required("--source")?),
                &PathBuf::from(required("--output")?),
            )?;
            println!("Collected {}", destination.display());
        }
        "export-data" => export_data(&PathBuf::from(required("--output")?))?,
        _ => unreachable!(),
    }
    Ok(())
}

fn libgcc_command(args: &[OsString]) -> Result<(), String> {
    let [command, flag, output] = args else {
        return Err("expected libgcc stub --output DIR; see --help".into());
    };
    if command != "stub" || flag != "--output" || output.is_empty() {
        return Err("expected libgcc stub --output DIR; see --help".into());
    }
    let output = PathBuf::from(output);
    let sources = libgcc::stub()?;
    std::fs::create_dir(&output)
        .map_err(|e| format!("cannot create new directory {}: {e}", output.display()))?;
    for (name, text) in [
        ("libgcc_s.S", &sources.assembly),
        ("libgcc_s.map", &sources.versions),
    ] {
        std::fs::write(output.join(name), text).map_err(|e| e.to_string())?;
    }
    println!(
        "{}: {} definitions, the interface of GCC {}",
        libgcc::SONAME,
        sources.selected.symbols.len(),
        libgcc::GCC_RELEASE
    );
    Ok(())
}

fn provider_command(args: &[OsString]) -> Result<(), String> {
    use libc_abi_tools::provider::{
        build::Tools,
        cache::{self, Mode},
        prepare,
    };
    let command = args
        .first()
        .and_then(|c| c.to_str())
        .ok_or("expected a provider command; see --help")?;
    let (values, flags): (&[&str], &[&str]) = match command {
        "prepare" => (
            &[
                "--glibc",
                "--mode",
                "--cc",
                "--engine",
                "--archives",
                "--data",
                "--cache",
            ],
            &["--all", "--force"],
        ),
        "list" => (&["--cache"], &[]),
        "path" => (&["--glibc", "--mode", "--cc", "--data", "--cache"], &[]),
        "verify" => (&["--glibc", "--mode", "--data", "--cache"], &[]),
        "export" => (&["--glibc", "--mode", "--to", "--cache"], &[]),
        "import" => (&["--cc", "--cache"], &[]),
        "remove" => (&["--glibc", "--mode", "--cache"], &["--all"]),
        _ => return Err(format!("unknown provider command {command:?}")),
    };
    let mut options = std::collections::BTreeMap::new();
    let mut set = std::collections::BTreeSet::new();
    let mut positional = Vec::new();
    let mut i = 1;
    while i < args.len() {
        let arg = args[i].to_str().ok_or("arguments must be UTF-8")?;
        if values.contains(&arg) {
            let value = args
                .get(i + 1)
                .and_then(|v| v.to_str())
                .filter(|v| !v.is_empty() && !v.starts_with("--"))
                .ok_or_else(|| format!("{arg} needs a value"))?;
            if options.insert(arg, value).is_some() {
                return Err(format!("duplicate option {arg}"));
            }
            i += 2;
        } else if flags.contains(&arg) {
            set.insert(arg);
            i += 1;
        } else if !arg.starts_with("--") && command == "import" && positional.is_empty() {
            positional.push(arg);
            i += 1;
        } else {
            return Err(format!("unknown option {arg:?} for provider {command}"));
        }
    }
    let root = match options.get("--cache") {
        Some(dir) => std::path::absolute(dir).map_err(|e| e.to_string())?,
        None => cache::default_root()?,
    };
    let mut tools = Tools::default();
    if let Some(cc) = options.get("--cc") {
        tools.gcc = cc.into();
    }
    let mode = options
        .get("--mode")
        .map(|m| m.parse::<Mode>())
        .transpose()?;
    let snapshot = |release: &str| -> Result<glibc::Snapshot, String> {
        let release: Version = release.parse()?;
        match options.get("--data") {
            Some(dir) => glibc::Snapshot::load(&PathBuf::from(dir), release),
            None => bundled_snapshot(release),
        }
    };
    let glibc = || -> Result<&str, String> {
        cache::release(options.get("--glibc").ok_or("--glibc is required")?)
    };
    let all = set.contains("--all");
    if all && options.contains_key("--glibc") {
        return Err("--all conflicts with --glibc".into());
    }
    let selected = |release: Option<&str>| -> Result<Vec<cache::Entry>, String> {
        Ok(cache::list(&root)?
            .into_iter()
            .filter(|e| release.is_none_or(|r| e.release == r) && mode.is_none_or(|m| e.mode == m))
            .collect())
    };
    match command {
        "prepare" => {
            let releases: Vec<&str> = if all {
                libc_abi_tools::provider::RELEASES.to_vec()
            } else {
                vec![glibc()?]
            };
            let mut prepare_options = prepare::Options::new(root.clone());
            prepare_options.tools = tools;
            prepare_options.force = set.contains("--force");
            prepare_options.engine = options.get("--engine").map(PathBuf::from);
            prepare_options.archives = options.get("--archives").map(PathBuf::from);
            for release in releases {
                let prepared = prepare::prepare(
                    release,
                    mode.unwrap_or(Mode::Source),
                    &snapshot(release)?,
                    &prepare_options,
                )?;
                let entry = &prepared.entry;
                eprintln!(
                    "glibc {release}: {} provider {} ({})",
                    entry.mode,
                    if prepared.reused {
                        "already prepared"
                    } else {
                        "prepared"
                    },
                    entry.hash
                );
                for note in &prepared.notes {
                    eprintln!("  {note}");
                }
                println!("{}", entry.descriptor().display());
            }
        }
        "list" => {
            println!("glibc\tmode\thash\tdescriptor");
            for entry in cache::list(&root)? {
                println!(
                    "{}\t{}\t{}\t{}",
                    entry.release,
                    entry.mode,
                    entry.hash,
                    entry.descriptor().display()
                );
            }
        }
        "path" => {
            let release = glibc()?;
            let snapshot = snapshot(release)?;
            let modes = match mode {
                Some(mode) => vec![mode],
                None => vec![Mode::Source, Mode::Container],
            };
            for mode in modes {
                if let Some(hash) = prepare::current_hash(release, mode, &snapshot, &tools)? {
                    let entry = cache::entry(&root, release, mode, &hash);
                    if entry.dir.join("manifest.json").is_file() {
                        println!("{}", entry.descriptor().display());
                        return Ok(());
                    }
                }
            }
            return Err(format!(
                "no current provider for glibc {release} in {}; run libc-abi provider prepare --glibc {release}",
                root.display()
            ));
        }
        "verify" => {
            let release = glibc()?;
            let entries = selected(Some(release))?;
            if entries.is_empty() {
                return Err(format!("no cached provider for glibc {release}"));
            }
            let snapshot = snapshot(release)?;
            for entry in entries {
                cache::verify(&entry, &snapshot)
                    .map_err(|e| format!("{} provider {}: {e}", entry.mode, entry.hash))?;
                println!(
                    "glibc {release}: {} provider {} verified",
                    entry.mode, entry.hash
                );
            }
        }
        "export" => {
            let release = glibc()?;
            let to = PathBuf::from(options.get("--to").ok_or("--to is required")?);
            let entries = selected(Some(release))?;
            if entries.is_empty() {
                return Err(format!("no cached provider for glibc {release}"));
            }
            for entry in entries {
                println!("{}", cache::export(&entry, &to)?.display());
            }
        }
        "import" => {
            let from = positional.first().ok_or("import needs a directory")?;
            let startup = tools.gcc_startup()?;
            for entry in cache::import(std::path::Path::new(from), &root, &startup)? {
                println!("{}", entry.descriptor().display());
            }
        }
        "remove" => {
            let release = if all { None } else { Some(glibc()?) };
            for entry in selected(release)? {
                cache::remove_tree(&entry.dir)?;
                println!("removed {}", entry.dir.display());
            }
        }
        _ => unreachable!(),
    }
    Ok(())
}

#[cfg(feature = "bundled-glibc-data")]
fn bundled_matrix() -> Result<Matrix, String> {
    Matrix::bundled()
}

#[cfg(feature = "bundled-glibc-data")]
fn bundled_snapshot(release: Version) -> Result<glibc::Snapshot, String> {
    glibc::Snapshot::bundled(release)
}

#[cfg(feature = "bundled-glibc-data")]
fn export_data(output: &std::path::Path) -> Result<(), String> {
    let count = glibc::export_bundled(output)?;
    println!("Exported {count} glibc ABI lists to {}", output.display());
    Ok(())
}

#[cfg(not(feature = "bundled-glibc-data"))]
const NOT_BUNDLED: &str =
    "built without the bundled-glibc-data feature; pass --data DIR (see --help)";

#[cfg(not(feature = "bundled-glibc-data"))]
fn bundled_matrix() -> Result<Matrix, String> {
    Err(NOT_BUNDLED.into())
}

#[cfg(not(feature = "bundled-glibc-data"))]
fn bundled_snapshot(_: Version) -> Result<glibc::Snapshot, String> {
    Err(NOT_BUNDLED.into())
}

#[cfg(not(feature = "bundled-glibc-data"))]
fn export_data(_: &std::path::Path) -> Result<(), String> {
    Err(NOT_BUNDLED.into())
}

fn main() {
    if let Err(error) = run(std::env::args_os().skip(1).collect()) {
        eprintln!("libc-abi: {error}");
        std::process::exit(1);
    }
}

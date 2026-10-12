//! The `provider` commands shared by `libc-abi provider` and `cargo libc provider`:
//! prepare|list|path|verify|export|import|remove.
use super::{
    build::Tools,
    cache::{self, Mode},
    prepare,
};
use crate::{glibc, version::Version};
use std::{ffi::OsString, path::PathBuf};

/// Help text for the provider commands; PROGRAM is the command prefix.
pub fn help(program: &str) -> String {
    format!(
        "  {program} provider prepare (--glibc VERSION | --all) [--mode MODE] [OPTIONS]
  {program} provider list
  {program} provider path --glibc VERSION [--mode MODE]
  {program} provider verify --glibc VERSION [--mode MODE]
  {program} provider export --glibc VERSION [--mode MODE] --to DIR
  {program} provider import DIR
  {program} provider remove (--glibc VERSION | --all) [--mode MODE]

provider prepares and manages glibc link providers for x86_64 Linux, glibc 2.17
through 2.44, in a cache: $CARGO_LIBC_PROVIDER_DIR, else $CARGO_HOME/libc-providers,
else ~/.cargo/libc-providers (or --cache DIR). Each provider has a descriptor
(provider.txt). MODE is one of:
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
"
    )
}

/// The ABI lists of RELEASE compiled into glibc-abilists.
#[cfg(feature = "bundled-glibc-data")]
pub fn bundled_snapshot(release: Version) -> Result<glibc::Snapshot, String> {
    glibc::Snapshot::bundled(release)
}

/// Without the bundled-glibc-data feature there are no compiled-in ABI lists.
#[cfg(not(feature = "bundled-glibc-data"))]
pub fn bundled_snapshot(_: Version) -> Result<glibc::Snapshot, String> {
    Err("built without the bundled-glibc-data feature; pass --data DIR".into())
}

/// Runs one provider command; ARGS start with the command name.
pub fn run(args: &[OsString], program: &str) -> Result<(), String> {
    let command = args
        .first()
        .and_then(|c| c.to_str())
        .ok_or_else(|| format!("expected a provider command; see {program} --help"))?;
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
                super::RELEASES.to_vec()
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
                "no current provider for glibc {release} in {}; run {program} provider prepare --glibc {release}",
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

/// Prepares RELEASE in MODE in the default cache ([`cache::default_root`]) with
/// the bundled ABI lists, or reuses the cached provider with the same inputs: what
/// a build tool does when no provider is given. GCC overrides the host's `gcc`.
pub fn ensure(
    release: &str,
    mode: Mode,
    gcc: Option<PathBuf>,
) -> Result<prepare::Prepared, String> {
    let release = cache::release(release)?;
    let mut options = prepare::Options::new(cache::default_root()?);
    if let Some(gcc) = gcc {
        options.tools.gcc = gcc;
    }
    prepare::prepare(
        release,
        mode,
        &bundled_snapshot(release.parse()?)?,
        &options,
    )
}

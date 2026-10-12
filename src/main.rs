use libc_abi_tools::{elf, glibc, libgcc, matrix::Matrix, version::Version};
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
  libc-abi glibc matrix [--data DIR] [--symbol NAME]
  libc-abi glibc diff [--data DIR] --from VERSION --to VERSION [--symbol NAME]
  libc-abi glibc changes [--data DIR]
  libc-abi glibc expect [--data DIR] --release VERSION --library NAME --symbol NAME --version VERSION
  libc-abi libgcc stub --output DIR
  libc-abi --version

The glibc ABI lists come from the GNU C Library's release tags through the
glibc-abilists crate (LGPL-2.1-or-later), compiled in by default. --data DIR
reads them from a directory instead; DIR contains version directories such as
2.17/ and 2.22/ (the layout of glibc-abilists' data/ and of export-data).
Built without the bundled-glibc-data feature, --data is required.

snapshot writes stable TSV for one exact x86_64 Linux glibc release.
export-data writes the bundled ABI lists into a new directory.
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
    if args.len() < 2 || args[0] != "glibc" {
        return Err("expected glibc snapshot, glibc collect, or glibc elf; see --help".into());
    }
    let command = args[1].to_str().ok_or("command must be UTF-8")?;
    let allowed: &[&str] = match command {
        "snapshot" => &["--data", "--release", "--symbol"],
        "collect" => &["--source", "--output"],
        "export-data" => &["--output"],
        "elf" => &["--input"],
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

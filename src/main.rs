use libc_abi_tools::{glibc, version::Version};
use std::{
    ffi::OsString,
    io::{self, Write},
    path::PathBuf,
};

const HELP: &str = "libc-abi - Rust libc ABI metadata tools (glibc first)

USAGE
  libc-abi glibc snapshot --data DIR --release VERSION [--symbol NAME]
  libc-abi glibc collect --source GLIBC_CHECKOUT --output DIR
  libc-abi --version

snapshot writes stable TSV for one exact x86_64 Linux glibc release.
DIR contains version directories such as 2.17/ and 2.22/.
collect requires a clean checkout at an exact glibc release tag; no overwrites.
Target releases start at 2.17; older symbol versions are retained.
No stub libraries, sysroots, headers, or runtime implementations are generated.
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
    if args.len() < 2 || args[0] != "glibc" {
        return Err("expected glibc snapshot or glibc collect; see --help".into());
    }
    let command = args[1].to_str().ok_or("command must be UTF-8")?;
    let allowed: &[&str] = match command {
        "snapshot" => &["--data", "--release", "--symbol"],
        "collect" => &["--source", "--output"],
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
        "snapshot" => {
            let data = PathBuf::from(required("--data")?);
            let release: Version = required("--release")?
                .to_str()
                .ok_or("release must be UTF-8")?
                .parse()?;
            let symbol = options
                .get("--symbol")
                .map(|s| s.to_str().ok_or("symbol must be UTF-8"))
                .transpose()?;
            let text = glibc::Snapshot::load(&data, release)?.to_tsv(symbol);
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
        _ => unreachable!(),
    }
    Ok(())
}

fn main() {
    if let Err(error) = run(std::env::args_os().skip(1).collect()) {
        eprintln!("libc-abi: {error}");
        std::process::exit(1);
    }
}

//! linux-abi-scan FILE... (--distro ID | [--glibc X.Y] [--kernel X.Y] [--target TRIPLE[.X.Y]])
//!                [--strict] [--verbose] [--tsv]
//!
//! Tier 1 static inspection; see the crate documentation.

use linux_abi_scan::{scan_file, ScanTarget};
use linux_targets::{Request, Selection};
use std::path::Path;
use std::process::ExitCode;

const USAGE: &str = "\
usage: linux-abi-scan FILE... (--distro ID | [--glibc X.Y] [--kernel X.Y] [--target TRIPLE[.X.Y]])
                      [--strict] [--verbose] [--tsv]

Statically inspects x86_64 ELF executables and shared objects against a
target's glibc and kernel floors. Exit status: 0 clean, 1 errors (or warnings
with --strict), 2 usage or read failure.

Tier 1 of three: exact for symbols, versions, and ELF notes; heuristic for
syscalls in machine code. Confirm with the seccomp/container simulation
(kernel-seccomp run) and on a real host at the target's minimums.";

fn main() -> ExitCode {
    match run(&std::env::args().skip(1).collect::<Vec<_>>()) {
        Ok(code) => ExitCode::from(code),
        Err(e) => {
            eprintln!("linux-abi-scan: {e}");
            ExitCode::from(2)
        }
    }
}

fn run(args: &[String]) -> Result<u8, String> {
    let (mut files, mut req) = (Vec::new(), Request::default());
    let (mut strict, mut verbose, mut tsv) = (false, false, false);
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let mut value = || {
            it.next()
                .map(String::as_str)
                .ok_or(format!("{a} needs a value"))
        };
        match a.as_str() {
            "--distro" => req.distro = Some(value()?),
            "--glibc" => req.glibc = Some(value()?),
            "--kernel" => req.kernel = Some(value()?),
            "--target" => req.target = Some(value()?),
            "--strict" => strict = true,
            "--verbose" | "-v" => verbose = true,
            "--tsv" => tsv = true,
            "-h" | "--help" => {
                println!("{USAGE}");
                return Ok(0);
            }
            s if s.starts_with('-') => return Err(format!("unknown option {s:?}\n{USAGE}")),
            f => files.push(f),
        }
    }
    if files.is_empty() {
        return Err(USAGE.to_string());
    }
    let target = ScanTarget::from_selection(&Selection::resolve(&req)?);
    let mut code = 0;
    for f in files {
        let report = scan_file(Path::new(f), &target)?;
        if tsv {
            print!("{}", report.to_tsv());
        } else {
            println!("== {f}");
            print!("{}", report.to_text(verbose));
        }
        code = code.max(report.exit_code(strict));
    }
    Ok(code as u8)
}

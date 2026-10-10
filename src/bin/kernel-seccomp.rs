//! Generates seccomp profiles that simulate an older Linux kernel.
//!
//! kernel-seccomp profile  (--kernel X.Y | --distro ID) [--output FILE]
//! kernel-seccomp missing  (--kernel X.Y | --distro ID)
//! kernel-seccomp distros
//! kernel-seccomp generate DIR

use kernel_abi_tools::{
    distro, distros, missing_syscalls, seccomp_profile, table_ceiling, table_floor, KernelVersion,
    Target, GENERIC_KERNELS,
};
use std::path::Path;
use std::process::ExitCode;
use std::{env, fs};

const USAGE: &str = "\
usage:
  kernel-seccomp profile (--kernel X.Y | --distro ID) [--output FILE]
  kernel-seccomp missing (--kernel X.Y | --distro ID)
  kernel-seccomp distros
  kernel-seccomp generate DIR";

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("kernel-seccomp: {e}");
            ExitCode::from(2)
        }
    }
}

fn run(args: &[String]) -> Result<(), String> {
    let (cmd, rest) = args.split_first().ok_or(USAGE)?;
    match cmd.as_str() {
        "profile" => {
            let target = select_target(rest)?;
            let json = seccomp_profile(&target);
            match flag(rest, "--output") {
                Some(path) => write(Path::new(&path), &json),
                None => {
                    print!("{json}");
                    Ok(())
                }
            }
        }
        "missing" => {
            let target = select_target(rest)?;
            let missing = missing_syscalls(target.kernel, &target.allow);
            println!("# {}: {} syscalls missing", target.label, missing.len());
            for s in missing {
                println!("{}\t{}\t{}", s.nr, s.name, s.first);
            }
            Ok(())
        }
        "distros" => {
            println!("id\tkernel\tglibc\tstatus\timage");
            for d in distros() {
                println!(
                    "{}\t{}\t{}\t{}\t{}",
                    d.id, d.kernel, d.glibc, d.status, d.image
                );
            }
            Ok(())
        }
        "generate" => {
            let dir = Path::new(rest.first().ok_or(USAGE)?);
            for k in GENERIC_KERNELS {
                let kernel = KernelVersion::parse(k)?;
                let path = dir.join("kernel").join(format!("linux-{kernel}.json"));
                write(&path, &seccomp_profile(&Target::kernel(kernel)))?;
            }
            for d in distros() {
                let path = dir.join("distro").join(format!("{}.json", d.id));
                write(&path, &seccomp_profile(&Target::distro(&d)))?;
            }
            eprintln!(
                "wrote {} kernel and {} distro profiles under {}",
                GENERIC_KERNELS.len(),
                distros().len(),
                dir.display()
            );
            Ok(())
        }
        "-h" | "--help" | "help" => {
            println!("{USAGE}");
            Ok(())
        }
        other => Err(format!("unknown command {other:?}\n{USAGE}")),
    }
}

fn select_target(args: &[String]) -> Result<Target, String> {
    match (flag(args, "--kernel"), flag(args, "--distro")) {
        (Some(k), None) => {
            let kernel = KernelVersion::parse(&k)?;
            let (floor, ceiling) = (table_floor(), table_ceiling());
            if kernel < floor {
                eprintln!(
                    "kernel-seccomp: warning: data starts at {floor}; {kernel} is treated as {floor}"
                );
            }
            if kernel > ceiling {
                eprintln!(
                    "kernel-seccomp: warning: data ends at {ceiling}; syscalls after it are unknown"
                );
            }
            Ok(Target::kernel(kernel))
        }
        (None, Some(id)) => distro(&id)
            .map(|d| Target::distro(&d))
            .ok_or_else(|| format!("unknown distro {id:?}; see `kernel-seccomp distros`")),
        _ => Err(format!("give exactly one of --kernel or --distro\n{USAGE}")),
    }
}

fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

fn write(path: &Path, contents: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    fs::write(path, contents).map_err(|e| format!("{}: {e}", path.display()))
}

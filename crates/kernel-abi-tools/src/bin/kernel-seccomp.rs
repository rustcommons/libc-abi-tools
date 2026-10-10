//! Generates seccomp profiles that simulate an older Linux kernel, and runs
//! programs in a container under them.
//!
//! kernel-seccomp profile  (--kernel X.Y | --distro ID) [--output FILE]
//! kernel-seccomp missing  (--kernel X.Y | --distro ID)
//! kernel-seccomp run      (--kernel X.Y | --distro ID) [--image IMAGE] -- PROGRAM [ARGS...]
//! kernel-seccomp distros
//! kernel-seccomp generate DIR
//! kernel-seccomp descriptor --commit SHA
//! kernel-seccomp verify-bundle DIR
//! kernel-seccomp version

use kernel_abi_tools::bundle::{verify_bundle, Descriptor};
use kernel_abi_tools::container::{self, Engine};
use kernel_abi_tools::{
    distro, distros, madvise_advice, missing_syscalls, seccomp_profile, syscalls, table_ceiling,
    table_floor, KernelVersion, Target, GENERIC_KERNELS, PROVIDER_FORMAT,
};
use std::path::Path;
use std::process::ExitCode;
use std::{env, fs};

const USAGE: &str = "\
usage:
  kernel-seccomp profile (--kernel X.Y | --distro ID) [--output FILE]
  kernel-seccomp missing (--kernel X.Y | --distro ID)
  kernel-seccomp run (--kernel X.Y | --distro ID) [--image IMAGE] -- PROGRAM [ARGS...]
  kernel-seccomp distros
  kernel-seccomp generate DIR
  kernel-seccomp descriptor --commit SHA
  kernel-seccomp verify-bundle DIR
  kernel-seccomp version

run environment:
  CONTAINER_ENGINE        podman (default) or docker
  KERNEL_ABI_ENGINE_ARGS  engine options placed before `run`, e.g. --runtime crun
  KERNEL_ABI_RUN_ARGS     options added to `run`, e.g. --network none";

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    match dispatch(&args) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("kernel-seccomp: {e}");
            ExitCode::from(2)
        }
    }
}

fn dispatch(args: &[String]) -> Result<ExitCode, String> {
    let (cmd, rest) = args.split_first().ok_or(USAGE)?;
    match cmd.as_str() {
        "profile" => {
            let target = select_target(rest)?;
            let json = seccomp_profile(&target);
            match flag(rest, "--output") {
                Some(path) => write(Path::new(&path), &json)?,
                None => print!("{json}"),
            }
        }
        "missing" => {
            let target = select_target(rest)?;
            let missing = missing_syscalls(target.kernel, &target.allow);
            println!("# {}: {} syscalls missing", target.label, missing.len());
            for s in missing {
                println!("{}\t{}\t{}", s.nr, s.name, s.first);
            }
        }
        "run" => return run(rest),
        "distros" => {
            println!("id\tkernel\tglibc\tstatus\timage");
            for d in distros() {
                println!(
                    "{}\t{}\t{}\t{}\t{}",
                    d.id, d.kernel, d.glibc, d.status, d.image
                );
            }
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
        }
        "version" | "--version" | "-V" => {
            println!("kernel-seccomp {}", env!("CARGO_PKG_VERSION"));
            println!("format\t{PROVIDER_FORMAT}");
            println!("arch\tx86_64");
            println!(
                "syscalls\t{} ({}..{})",
                syscalls().len(),
                table_floor(),
                table_ceiling()
            );
            println!("madvise-values\t{}", madvise_advice().len());
            println!("distros\t{}", distros().len());
        }
        "descriptor" => {
            let commit = flag(rest, "--commit").ok_or("descriptor needs --commit SHA")?;
            let descriptor = Descriptor {
                version: env!("CARGO_PKG_VERSION").to_string(),
                arch: "x86_64".to_string(),
                commit,
                kernel_seccomp: "bin/kernel-seccomp".into(),
                syscall_probe: "bin/syscall-probe".into(),
                profiles: "profiles".into(),
                distros: "data/distros.tsv".into(),
                inventory: "SHA256SUMS".into(),
            };
            print!("{}", descriptor.render());
        }
        "verify-bundle" => {
            let dir = Path::new(rest.first().ok_or(USAGE)?);
            let report = verify_bundle(dir)?;
            println!(
                "verified kernel-abi-tools {} ({}, commit {}): {} files match {}",
                report.descriptor.version,
                report.descriptor.arch,
                report.descriptor.commit,
                report.files_verified,
                report.descriptor.inventory.display()
            );
        }
        "-h" | "--help" | "help" => println!("{USAGE}"),
        other => return Err(format!("unknown command {other:?}\n{USAGE}")),
    }
    Ok(ExitCode::SUCCESS)
}

/// `run` subcommand: parses the selection and delegates to
/// [`container::run`], which documents mounts and environment.
fn run(args: &[String]) -> Result<ExitCode, String> {
    let split = args.iter().position(|a| a == "--");
    let (opts, program) = match split {
        Some(i) => (&args[..i], &args[i + 1..]),
        None => return Err(format!("run needs `--` before the program\n{USAGE}")),
    };
    let (program, program_args) = program
        .split_first()
        .ok_or(format!("run needs a program after `--`\n{USAGE}"))?;
    let target = select_target(opts)?;
    let image = match (flag(opts, "--image"), flag(opts, "--distro")) {
        (Some(image), _) => image,
        (None, Some(id)) => distro(&id).map(|d| d.image.to_string()).unwrap_or_default(),
        (None, None) => return Err("--image is required with --kernel".to_string()),
    };
    let code = container::run(&Engine::from_env(), &target, &image, program, program_args)?;
    Ok(ExitCode::from(code.clamp(0, 255) as u8))
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
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
    }
    fs::write(path, contents).map_err(|e| format!("{}: {e}", path.display()))
}

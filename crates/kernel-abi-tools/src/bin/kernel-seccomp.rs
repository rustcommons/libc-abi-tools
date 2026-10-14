//! Generates seccomp profiles that simulate an older Linux kernel, and runs
//! programs under them: on the host with an in-process filter, or in a
//! container.
//!
//! kernel-seccomp profile  (--kernel X.Y | --distro ID) [--output FILE]
//! kernel-seccomp missing  (--kernel X.Y | --distro ID)
//! kernel-seccomp run      (--kernel X.Y | --distro ID) [--runner local|container]
//!                         [--image IMAGE] -- PROGRAM [ARGS...]
//! kernel-seccomp distros
//! kernel-seccomp generate DIR
//! kernel-seccomp version

use kernel_abi_tools::container::{self, Engine};
use kernel_abi_tools::{
    distro, distros, local, madvise_advice, missing_syscalls, seccomp_profile, syscalls,
    table_ceiling, table_floor, KernelVersion, Runner, Target, GENERIC_KERNELS, PROVIDER_FORMAT,
};
use std::fmt::Write as _;
use std::io::{self, Write as _};
use std::path::Path;
use std::process::ExitCode;
use std::{env, fs};

const USAGE: &str = "\
usage:
  kernel-seccomp profile (--kernel X.Y | --distro ID) [--output FILE]
  kernel-seccomp missing (--kernel X.Y | --distro ID)
  kernel-seccomp run (--kernel X.Y | --distro ID) [--runner local|container]
                     [--image IMAGE] -- PROGRAM [ARGS...]
  kernel-seccomp distros
  kernel-seccomp generate DIR
  kernel-seccomp version

run: the default runner is the container when an image is known (--image, or
the --distro preset's image) and the local filter otherwise. The local runner
needs no container runtime and no root, but keeps the host userland.

container runner environment:
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
                None => emit(&json)?,
            }
        }
        "missing" => {
            let target = select_target(rest)?;
            let missing = missing_syscalls(target.kernel, &target.allow);
            let mut out = format!("# {}: {} syscalls missing\n", target.label, missing.len());
            for s in missing {
                let _ = writeln!(out, "{}\t{}\t{}", s.nr, s.name, s.first);
            }
            emit(&out)?;
        }
        "run" => return run(rest),
        "distros" => {
            let mut out = String::from("id\tkernel\tglibc\tstatus\timage\n");
            for d in distros() {
                let _ = writeln!(
                    out,
                    "{}\t{}\t{}\t{}\t{}",
                    d.id, d.kernel, d.glibc, d.status, d.image
                );
            }
            emit(&out)?;
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
            emit(&format!(
                "kernel-seccomp {}\nformat\t{PROVIDER_FORMAT}\narch\tx86_64\n\
                 syscalls\t{} ({}..{})\nmadvise-values\t{}\ndistros\t{}\n",
                env!("CARGO_PKG_VERSION"),
                syscalls().len(),
                table_floor(),
                table_ceiling(),
                madvise_advice().len(),
                distros().len()
            ))?;
        }
        "-h" | "--help" | "help" => emit(&format!("{USAGE}\n"))?,
        other => return Err(format!("unknown command {other:?}\n{USAGE}")),
    }
    Ok(ExitCode::SUCCESS)
}

/// `run` subcommand: parses the selection and delegates to [`local::run`]
/// or [`container::run`], which document what each runner provides.
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
    let requested = flag(opts, "--runner")
        .map(|r| Runner::parse(&r))
        .transpose()?;
    let explicit_image = flag(opts, "--image");
    let preset_image = flag(opts, "--distro")
        .and_then(|id| distro(&id))
        .map(|d| d.image.to_string());
    let image = match requested {
        // A preset's image is a default; the local runner simply ignores it.
        Some(Runner::Local) => explicit_image,
        _ => explicit_image.or(preset_image),
    };
    let code = match Runner::choose(requested, image.as_deref())? {
        Runner::Local => {
            eprintln!(
                "kernel-seccomp: simulating {} with a local filter; userland (libc) is the host's",
                target.label
            );
            local::run(&target, program, program_args)?
        }
        Runner::Container => container::run(
            &Engine::from_env(),
            &target,
            image.as_deref().unwrap_or_default(),
            program,
            program_args,
        )?,
    };
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

/// Writes a command's output to stdout. `print!` would panic when the reader
/// has gone (`kernel-seccomp missing ... | head`): Rust ignores SIGPIPE, so the
/// write fails with a broken pipe instead of ending the process. Nobody is
/// reading any more, so that is not an error: stop quietly.
fn emit(text: &str) -> Result<(), String> {
    let mut out = io::stdout().lock();
    match out.write_all(text.as_bytes()).and_then(|()| out.flush()) {
        Err(e) if e.kind() != io::ErrorKind::BrokenPipe => {
            Err(format!("cannot write to stdout: {e}"))
        }
        _ => Ok(()),
    }
}

fn write(path: &Path, contents: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
    }
    fs::write(path, contents).map_err(|e| format!("{}: {e}", path.display()))
}

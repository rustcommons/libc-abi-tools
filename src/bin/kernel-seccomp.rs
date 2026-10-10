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
use kernel_abi_tools::{
    distro, distros, madvise_advice, missing_syscalls, seccomp_profile, syscalls, table_ceiling,
    table_floor, KernelVersion, Target, GENERIC_KERNELS, PROVIDER_FORMAT,
};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::{SystemTime, UNIX_EPOCH};
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

/// Runs PROGRAM in a container under the selected profile and returns its
/// exit status. The current directory and, if PROGRAM is a host path outside
/// it, the PROGRAM file are mounted at their own absolute paths so relative
/// paths and compile-time `CARGO_MANIFEST_DIR` values keep working; a bare
/// PROGRAM name runs from the image. `CARGO*`/`RUST*` variables are
/// forwarded. Usable as a Cargo `runner`.
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

    let cwd = env::current_dir().map_err(|e| format!("current directory: {e}"))?;
    // A bare name runs from the image's PATH. A path names a host file, which
    // is mounted by itself (never its directory, which could hide the image's
    // own /bin or /usr/bin) unless it is already under the mounted cwd.
    let host_program = program
        .contains('/')
        .then(|| absolute(&cwd, Path::new(program)));
    if let Some(p) = &host_program {
        if !p.is_file() {
            return Err(format!("{} is not a file on this host", p.display()));
        }
    }

    let profile = TempFile::new("kernel-seccomp", ".json")?;
    write(&profile.0, &seccomp_profile(&target))?;

    let engine = env::var("CONTAINER_ENGINE").unwrap_or_else(|_| "podman".to_string());
    let mut cmd = Command::new(&engine);
    cmd.args(env_words("KERNEL_ABI_ENGINE_ARGS"));
    cmd.args(["run", "--rm", "--security-opt"]);
    cmd.arg(format!("seccomp={}", profile.0.display()));
    cmd.arg("-v").arg(format!("{0}:{0}:z", cwd.display()));
    match &host_program {
        Some(p) if !p.starts_with(&cwd) => {
            cmd.arg("-v").arg(format!("{0}:{0}:ro,z", p.display()));
        }
        _ => {}
    }
    cmd.arg("-w").arg(&cwd);
    cmd.args(["-e", "CARGO*", "-e", "RUST*"]);
    cmd.args(env_words("KERNEL_ABI_RUN_ARGS"));
    cmd.arg(&image);
    match &host_program {
        Some(p) => cmd.arg(p),
        None => cmd.arg(program),
    };
    cmd.args(program_args);

    let status = cmd
        .status()
        .map_err(|e| format!("cannot start {engine}: {e}"))?;
    Ok(match status.code() {
        Some(code) => ExitCode::from(code.clamp(0, 255) as u8),
        None => ExitCode::from(128),
    })
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

fn env_words(name: &str) -> Vec<String> {
    env::var(name)
        .unwrap_or_default()
        .split_whitespace()
        .map(str::to_string)
        .collect()
}

fn absolute(cwd: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
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

/// A file in the temporary directory, removed when dropped.
struct TempFile(PathBuf);

impl TempFile {
    fn new(prefix: &str, suffix: &str) -> Result<Self, String> {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let name = format!("{prefix}-{}-{nanos}{suffix}", std::process::id());
        let path = env::temp_dir().join(name);
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(TempFile(path))
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

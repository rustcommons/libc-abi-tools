//! Runs a program on the host under a seccomp filter that simulates a kernel.
//!
//! No container runtime and no root are needed: the child sets
//! `no_new_privs`, installs the filter from [`crate::filter`], and executes
//! the program. Only the kernel is simulated; the program still uses the
//! host's libc and userland. Inside the filtered process tree, setuid
//! programs (such as `sudo`) cannot gain privileges.

use crate::Target;

/// Runs `program` with `args` under the filter for `target` and returns its
/// exit status (128 if it was killed by a signal). The current directory and
/// environment are inherited. Suitable for a Cargo runner.
///
/// # Errors
///
/// Returns a message when the host is not x86_64 or `program` cannot be
/// started, including when the filter cannot be installed.
#[cfg(target_os = "linux")]
pub fn run(target: &Target, program: &str, args: &[String]) -> Result<i32, String> {
    use crate::filter::{install, program as filter_program};
    use std::os::unix::process::CommandExt;
    use std::process::Command;

    if !cfg!(target_arch = "x86_64") {
        return Err("the local runner simulates x86_64 kernels only".to_string());
    }
    let insns = filter_program(target);
    let mut cmd = Command::new(program);
    cmd.args(args);
    // SAFETY: the closure only calls prctl (async-signal-safe) on data built
    // before the fork; it does not allocate.
    unsafe {
        cmd.pre_exec(move || install::apply(&install::Prog::new(&insns)));
    }
    let status = cmd
        .status()
        .map_err(|e| format!("cannot start {program} under the seccomp filter: {e}"))?;
    Ok(status.code().unwrap_or(128))
}

/// The local runner needs Linux; elsewhere it reports why it cannot run.
///
/// # Errors
///
/// Always returns a message saying a Linux host is needed.
#[cfg(not(target_os = "linux"))]
pub fn run(_target: &Target, _program: &str, _args: &[String]) -> Result<i32, String> {
    Err("the local runner needs a Linux host; use the container runner".to_string())
}

#[cfg(all(test, target_os = "linux", target_arch = "x86_64"))]
mod tests {
    use super::*;
    use crate::KernelVersion;

    #[test]
    fn runs_a_program_under_the_filter() {
        let target = Target::kernel(KernelVersion::parse("3.10").unwrap());
        let ok = run(&target, "/bin/sh", &["-c".into(), "exit 7".into()]);
        assert_eq!(ok, Ok(7));
        assert!(run(&target, "/no/such/program", &[]).is_err());
    }
}

//! Verifies that a seccomp profile simulates the intended kernel.
//!
//! syscall-probe dump                              (run without a profile)
//! syscall-probe check (--kernel X.Y | --distro ID) --baseline FILE
//!
//! Only syscalls newer than the data floor are probed, each invoked directly
//! with every argument set to -1 so the kernel rejects it (EINVAL, EBADF,
//! EFAULT, ...) before doing work. Any result other than ENOSYS means the call
//! reached a kernel that implements it.

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn main() -> std::process::ExitCode {
    linux::main()
}

#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
fn main() -> std::process::ExitCode {
    eprintln!("syscall-probe: only x86_64 Linux is supported");
    std::process::ExitCode::from(2)
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod linux {
    use kernel_abi_tools::{
        distro, missing_syscalls, syscalls, table_floor, KernelVersion, Syscall, Target, ENOSYS,
    };
    use std::collections::HashMap;
    use std::process::ExitCode;
    use std::{env, fs};

    /// Never invoked: they signal, load code, or have unreviewed semantics.
    /// Their profile entries are still covered by the profile generator tests.
    const UNSAFE_TO_PROBE: &[&str] = &[
        "finit_module",
        "kexec_file_load",
        "uretprobe",
        "uprobe",
        "rseq_slice_yield",
    ];

    pub fn main() -> ExitCode {
        let args: Vec<String> = env::args().skip(1).collect();
        let result = match args.first().map(String::as_str) {
            Some("dump") => {
                dump();
                Ok(true)
            }
            Some("check") => check(&args[1..]),
            _ => Err(
                "usage: syscall-probe dump | check (--kernel X.Y | --distro ID) --baseline FILE"
                    .to_string(),
            ),
        };
        match result {
            Ok(true) => ExitCode::SUCCESS,
            Ok(false) => ExitCode::from(1),
            Err(e) => {
                eprintln!("syscall-probe: {e}");
                ExitCode::from(2)
            }
        }
    }

    fn probed() -> Vec<Syscall> {
        let floor = table_floor();
        syscalls()
            .into_iter()
            .filter(|s| s.first > floor && !UNSAFE_TO_PROBE.contains(&s.name))
            .collect()
    }

    /// Returns 0 or the positive errno.
    fn invoke(nr: u32) -> i32 {
        let ret: i64;
        let m1 = u64::MAX;
        // SAFETY: every syscall in `probed()` validates its arguments before
        // acting, and all-ones arguments are rejected (see module docs).
        unsafe {
            std::arch::asm!(
                "syscall",
                inlateout("rax") nr as i64 => ret,
                in("rdi") m1, in("rsi") m1, in("rdx") m1,
                in("r10") m1, in("r8") m1, in("r9") m1,
                lateout("rcx") _, lateout("r11") _,
                options(nostack),
            );
        }
        if (-4095..0).contains(&ret) {
            -ret as i32
        } else {
            0
        }
    }

    fn dump() {
        for s in probed() {
            println!("{}\t{}\t{}", s.name, s.first, invoke(s.nr));
        }
    }

    fn check(args: &[String]) -> Result<bool, String> {
        let flag = |name: &str| {
            args.iter()
                .position(|a| a == name)
                .and_then(|i| args.get(i + 1))
                .cloned()
        };
        let target = match (flag("--kernel"), flag("--distro")) {
            (Some(k), None) => Target::kernel(KernelVersion::parse(&k)?),
            (None, Some(id)) => {
                Target::distro(&distro(&id).ok_or_else(|| format!("unknown distro {id:?}"))?)
            }
            _ => return Err("give exactly one of --kernel or --distro".to_string()),
        };
        let baseline_path = flag("--baseline").ok_or("missing --baseline")?;
        let baseline_text =
            fs::read_to_string(&baseline_path).map_err(|e| format!("{baseline_path}: {e}"))?;
        let baseline: HashMap<&str, i32> = baseline_text
            .lines()
            .filter_map(|l| {
                let mut c = l.split('\t');
                Some((c.next()?, c.nth(1)?.parse().ok()?))
            })
            .collect();

        let missing: Vec<&str> = missing_syscalls(target.kernel, &target.allow)
            .iter()
            .map(|s| s.name)
            .collect();
        let (mut pass, mut fail, mut host_lacks) = (0, 0, 0);
        for s in probed() {
            let got = invoke(s.nr);
            let base = baseline.get(s.name).copied();
            let should_block = missing.contains(&s.name);
            let verdict = match (should_block, got == ENOSYS, base == Some(ENOSYS)) {
                (true, true, _) => "PASS blocked",
                (true, false, _) => "FAIL reached the kernel; profile did not block it",
                (false, false, _) => "PASS allowed",
                (false, true, true) => "SKIP host kernel lacks it too",
                (false, true, false) => "FAIL blocked but the simulated kernel has it",
            };
            match verdict.split(' ').next() {
                Some("PASS") => pass += 1,
                Some("FAIL") => fail += 1,
                _ => host_lacks += 1,
            }
            if !verdict.starts_with("PASS") {
                println!("{verdict}: {} (first in {}, errno {got})", s.name, s.first);
            }
        }
        println!(
            "{} simulation: {pass} pass, {fail} fail, {host_lacks} skipped (host lacks)",
            target.label
        );
        Ok(fail == 0)
    }
}

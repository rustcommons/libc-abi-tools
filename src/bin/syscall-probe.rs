//! Verifies that a seccomp profile simulates the intended kernel.
//!
//! syscall-probe dump                              (run without a profile)
//! syscall-probe check (--kernel X.Y | --distro ID) --baseline FILE
//!
//! Syscalls: only those newer than the data floor are probed, each invoked
//! directly with every argument set to -1 so the kernel rejects it (EINVAL,
//! EBADF, EFAULT, ...) before doing work. Any result other than ENOSYS means
//! the call reached a kernel that implements it.
//!
//! madvise: every advice value from 0 to a few past the highest known value is
//! invoked as `madvise(0, 0, advice)`. The kernel validates the advice first and
//! then returns without acting on a zero-length range.

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
        distro, madvise_accepts, madvise_advice, missing_syscalls, syscalls, table_floor,
        KernelVersion, Syscall, Target, EINVAL, ENOSYS,
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

    /// x86_64 `madvise` syscall number.
    const SYS_MADVISE: u32 = 28;

    /// Advice values probed past the highest known one.
    const MADVISE_EXTRA: u32 = 3;

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

    fn probed_advice() -> std::ops::RangeInclusive<u32> {
        let max = madvise_advice().iter().map(|a| a.value).max().unwrap_or(0);
        0..=max + MADVISE_EXTRA
    }

    fn advice_key(value: u32) -> String {
        format!("madvise:{value}")
    }

    /// Returns 0 or the positive errno.
    fn invoke(nr: u32, a: [u64; 6]) -> i32 {
        let ret: i64;
        // SAFETY: callers pass argument sets the kernel rejects or treats as a
        // no-op (see module docs).
        unsafe {
            std::arch::asm!(
                "syscall",
                inlateout("rax") nr as i64 => ret,
                in("rdi") a[0], in("rsi") a[1], in("rdx") a[2],
                in("r10") a[3], in("r8") a[4], in("r9") a[5],
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

    fn invoke_rejected(nr: u32) -> i32 {
        invoke(nr, [u64::MAX; 6])
    }

    fn invoke_madvise(advice: u32) -> i32 {
        invoke(SYS_MADVISE, [0, 0, advice as u64, 0, 0, 0])
    }

    fn dump() {
        for s in probed() {
            println!("{}\t{}\t{}", s.name, s.first, invoke_rejected(s.nr));
        }
        for v in probed_advice() {
            println!("{}\t-\t{}", advice_key(v), invoke_madvise(v));
        }
    }

    #[derive(Default)]
    struct Tally {
        pass: u32,
        fail: u32,
        skip: u32,
    }

    impl Tally {
        fn record(&mut self, verdict: &str, subject: &str, got: i32) {
            match verdict.split(' ').next() {
                Some("PASS") => self.pass += 1,
                Some("FAIL") => self.fail += 1,
                _ => self.skip += 1,
            }
            if !verdict.starts_with("PASS") {
                println!("{verdict}: {subject} (errno {got})");
            }
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
        let mut syscalls = Tally::default();
        for s in probed() {
            let got = invoke_rejected(s.nr);
            let base = baseline.get(s.name).copied();
            let verdict = match (
                missing.contains(&s.name),
                got == ENOSYS,
                base == Some(ENOSYS),
            ) {
                (true, true, _) => "PASS blocked",
                (true, false, _) => "FAIL reached the kernel; profile did not block it",
                (false, false, _) => "PASS allowed",
                (false, true, true) => "SKIP host kernel lacks it too",
                (false, true, false) => "FAIL blocked but the simulated kernel has it",
            };
            syscalls.record(verdict, &format!("{} (first in {})", s.name, s.first), got);
        }

        let mut advice = Tally::default();
        for v in probed_advice() {
            let got = invoke_madvise(v);
            let base = baseline.get(advice_key(v).as_str()).copied();
            let verdict = if !madvise_accepts(target.kernel, v) {
                if got == EINVAL {
                    "PASS rejected"
                } else {
                    "FAIL accepted advice the simulated kernel lacks"
                }
            } else if Some(got) == base {
                "PASS unchanged"
            } else {
                "FAIL changed advice the simulated kernel has"
            };
            advice.record(verdict, &advice_key(v), got);
        }

        println!(
            "{} simulation: syscalls {} pass, {} fail, {} skipped (host lacks); \
             madvise {} pass, {} fail",
            target.label, syscalls.pass, syscalls.fail, syscalls.skip, advice.pass, advice.fail
        );
        Ok(syscalls.fail == 0 && advice.fail == 0)
    }
}

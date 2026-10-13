//! Classic-BPF seccomp filters built directly from the data, for the local
//! runner: no container, no libseccomp, and no root.
//!
//! The program makes the same decisions as [`crate::seccomp_profile`]:
//!
//! 1. Any architecture other than x86_64 (e.g. `int 0x80` i386 calls) and any
//!    x32 syscall number returns `ENOSYS`.
//! 2. `madvise` returns `EINVAL` unless the low 32 bits of the advice (the
//!    kernel reads an `int`) are a value the target kernel knows.
//! 3. Syscalls the target provides are allowed; everything else, including
//!    numbers this crate's data does not know, returns `ENOSYS`.
//!
//! Installing a filter needs no privilege once the process has set
//! `PR_SET_NO_NEW_PRIVS`. Like the profiles, the filter is a testing aid and
//! not a security boundary.

use crate::{madvise_accepts, madvise_advice, present_syscalls, Target, EINVAL, ENOSYS};

/// `AUDIT_ARCH_X86_64`: `EM_X86_64 | __AUDIT_ARCH_64BIT | __AUDIT_ARCH_LE`.
pub const AUDIT_ARCH_X86_64: u32 = 0xc000_003e;
/// Syscall numbers at or above this bit belong to the x32 ABI.
pub const X32_SYSCALL_BIT: u32 = 0x4000_0000;
/// x86_64 `madvise` syscall number.
pub const SYS_MADVISE: u32 = 28;

/// `SECCOMP_RET_ALLOW`.
pub const RET_ALLOW: u32 = 0x7fff_0000;
/// `SECCOMP_RET_ERRNO`; the errno goes in the low 16 bits.
pub const RET_ERRNO: u32 = 0x0005_0000;

// Opcodes used by the program (linux/bpf_common.h).
const LD_W_ABS: u16 = 0x20; // BPF_LD | BPF_W | BPF_ABS
const JEQ_K: u16 = 0x15; // BPF_JMP | BPF_JEQ | BPF_K
const JGT_K: u16 = 0x25; // BPF_JMP | BPF_JGT | BPF_K
const JGE_K: u16 = 0x35; // BPF_JMP | BPF_JGE | BPF_K
const RET_K: u16 = 0x06; // BPF_RET | BPF_K

// Offsets into `struct seccomp_data` (x86_64 is little-endian, so the low
// word of an argument comes first).
const OFF_NR: u32 = 0;
const OFF_ARCH: u32 = 4;
const OFF_ARG2_LO: u32 = 16 + 2 * 8;

/// The kernel's `BPF_MAXINSNS`.
pub const MAX_INSNS: usize = 4096;

/// One `struct sock_filter` instruction.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Insn {
    /// The opcode (`BPF_LD | BPF_W | BPF_ABS` and so on).
    pub code: u16,
    /// Instructions to skip when a conditional jump is taken.
    pub jt: u8,
    /// Instructions to skip when a conditional jump is not taken.
    pub jf: u8,
    /// The constant operand: an offset, a value to compare, or a return value.
    pub k: u32,
}

const fn stmt(code: u16, k: u32) -> Insn {
    Insn {
        code,
        jt: 0,
        jf: 0,
        k,
    }
}

const fn jump(code: u16, k: u32, jt: u8, jf: u8) -> Insn {
    Insn { code, jt, jf, k }
}

/// Builds the filter program for `target`.
///
/// Allowed syscall numbers are merged into contiguous ranges; each range is
/// three instructions with only short forward jumps, so the program needs no
/// jump longer than the 255 instructions BPF allows.
pub fn program(target: &Target) -> Vec<Insn> {
    let enosys = RET_ERRNO | ENOSYS as u32;
    let einval = RET_ERRNO | EINVAL as u32;
    let mut p = vec![
        stmt(LD_W_ABS, OFF_ARCH),
        jump(JEQ_K, AUDIT_ARCH_X86_64, 1, 0),
        stmt(RET_K, enosys),
        stmt(LD_W_ABS, OFF_NR),
        jump(JGE_K, X32_SYSCALL_BIT, 0, 1),
        stmt(RET_K, enosys),
    ];

    let mut nrs: Vec<u32> = present_syscalls(target).iter().map(|s| s.nr).collect();
    nrs.sort_unstable();
    nrs.dedup();
    if nrs.binary_search(&SYS_MADVISE).is_ok() {
        let accepted: Vec<u32> = madvise_advice()
            .iter()
            .map(|a| a.value)
            .filter(|&v| madvise_accepts(target.kernel, v))
            .collect();
        // Not madvise: skip the advice block (the load, one test per value,
        // EINVAL, and ALLOW) and land on the reload of the number.
        let skip = u8::try_from(accepted.len() + 3).expect("madvise block fits a BPF jump");
        p.push(jump(JEQ_K, SYS_MADVISE, 0, skip));
        p.push(stmt(LD_W_ABS, OFF_ARG2_LO));
        let n = accepted.len();
        for (i, v) in accepted.iter().enumerate() {
            // Jump over the remaining tests and the EINVAL to the ALLOW.
            let to_allow = u8::try_from(n - i).expect("madvise block fits a BPF jump");
            p.push(jump(JEQ_K, *v, to_allow, 0));
        }
        p.push(stmt(RET_K, einval));
        p.push(stmt(RET_K, RET_ALLOW));
        // The accumulator holds the advice now; reload the number.
        p.push(stmt(LD_W_ABS, OFF_NR));
    }

    for (lo, hi) in ranges(&nrs) {
        p.push(jump(JGE_K, lo, 0, 2));
        p.push(jump(JGT_K, hi, 1, 0));
        p.push(stmt(RET_K, RET_ALLOW));
    }
    p.push(stmt(RET_K, enosys));
    assert!(p.len() <= MAX_INSNS, "filter exceeds BPF_MAXINSNS");
    p
}

fn ranges(sorted: &[u32]) -> Vec<(u32, u32)> {
    let mut out: Vec<(u32, u32)> = Vec::new();
    for &n in sorted {
        match out.last_mut() {
            Some((_, hi)) if *hi + 1 == n => *hi = n,
            _ => out.push((n, n)),
        }
    }
    out
}

/// Runs `prog` on one syscall the way the kernel does and returns the
/// `SECCOMP_RET_*` value. Supports only the opcodes [`program`] emits.
///
/// `arch` is the `AUDIT_ARCH_*` value, `nr` the syscall number, and `args` its
/// six arguments.
///
/// # Errors
///
/// Returns a message for an opcode or load offset [`program`] never emits, and
/// when the program ends without returning.
pub fn evaluate(prog: &[Insn], arch: u32, nr: u32, args: [u64; 6]) -> Result<u32, String> {
    let word = |off: u32| -> Result<u32, String> {
        match off {
            OFF_NR => Ok(nr),
            OFF_ARCH => Ok(arch),
            16..=63 if off.is_multiple_of(4) => {
                let arg = args[(off as usize - 16) / 8];
                Ok(if off.is_multiple_of(8) {
                    arg as u32
                } else {
                    (arg >> 32) as u32
                })
            }
            _ => Err(format!("load from unsupported offset {off}")),
        }
    };
    let mut acc = 0u32;
    let mut pc = 0usize;
    while let Some(insn) = prog.get(pc) {
        pc += 1;
        let taken = match insn.code {
            LD_W_ABS => {
                acc = word(insn.k)?;
                continue;
            }
            RET_K => return Ok(insn.k),
            JEQ_K => acc == insn.k,
            JGT_K => acc > insn.k,
            JGE_K => acc >= insn.k,
            other => return Err(format!("unsupported opcode {other:#x}")),
        };
        pc += usize::from(if taken { insn.jt } else { insn.jf });
    }
    Err("program fell off the end".to_string())
}

/// Installs filters in the calling thread (and what it executes).
#[cfg(target_os = "linux")]
pub mod install {
    use super::Insn;
    use std::io;

    const PR_SET_NO_NEW_PRIVS: i32 = 38;
    const PR_SET_SECCOMP: i32 = 22;
    const SECCOMP_MODE_FILTER: u64 = 2;

    /// `struct sock_fprog`.
    #[repr(C)]
    pub struct Prog<'a> {
        len: u16,
        filter: *const Insn,
        _insns: std::marker::PhantomData<&'a [Insn]>,
    }

    impl<'a> Prog<'a> {
        /// Wraps `insns` for [`apply`].
        ///
        /// # Panics
        ///
        /// Panics if `insns` has more than 65535 instructions.
        pub fn new(insns: &'a [Insn]) -> Self {
            Prog {
                len: u16::try_from(insns.len()).expect("filter fits sock_fprog"),
                filter: insns.as_ptr(),
                _insns: std::marker::PhantomData,
            }
        }
    }

    extern "C" {
        fn prctl(option: i32, arg2: u64, arg3: u64, arg4: u64, arg5: u64) -> i32;
    }

    /// Sets `no_new_privs` and installs `prog` for the calling thread. It
    /// cannot be removed and is inherited across `fork` and `execve`.
    ///
    /// Only calls `prctl`, so it is async-signal-safe and may run between
    /// `fork` and `exec` (for example from `CommandExt::pre_exec`).
    ///
    /// # Errors
    ///
    /// Returns the OS error when either `prctl` call fails, for example when the
    /// kernel lacks seccomp filters or the program is too long.
    pub fn apply(prog: &Prog<'_>) -> io::Result<()> {
        // SAFETY: plain prctl calls; `prog` outlives them and points at
        // `len` valid instructions.
        unsafe {
            if prctl(PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) != 0 {
                return Err(io::Error::last_os_error());
            }
            let ptr = prog as *const Prog<'_> as u64;
            if prctl(PR_SET_SECCOMP, SECCOMP_MODE_FILTER, ptr, 0, 0) != 0 {
                return Err(io::Error::last_os_error());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{syscalls, table_ceiling, KernelVersion};

    fn kv(s: &str) -> KernelVersion {
        KernelVersion::parse(s).unwrap()
    }

    fn eval(prog: &[Insn], nr: u32, a2: u64) -> u32 {
        evaluate(prog, AUDIT_ARCH_X86_64, nr, [0, 0, a2, 0, 0, 0]).unwrap()
    }

    /// The filter must agree with the data (and so with the JSON profile)
    /// for every known syscall, unknown numbers, and every advice value.
    fn assert_matches_data(target: &Target) {
        let prog = program(target);
        let present: Vec<u32> = present_syscalls(target).iter().map(|s| s.nr).collect();
        let max_nr = syscalls().iter().map(|s| s.nr).max().unwrap();
        for nr in 0..=max_nr + 64 {
            if nr == SYS_MADVISE {
                continue;
            }
            let want = if present.contains(&nr) {
                RET_ALLOW
            } else {
                RET_ERRNO | ENOSYS as u32
            };
            assert_eq!(eval(&prog, nr, 0), want, "{}: nr {nr}", target.label);
        }
        let max_advice = madvise_advice().iter().map(|a| a.value).max().unwrap();
        for v in 0..=max_advice + 8 {
            let want = if madvise_accepts(target.kernel, v) {
                RET_ALLOW
            } else {
                RET_ERRNO | EINVAL as u32
            };
            assert_eq!(eval(&prog, SYS_MADVISE, v as u64), want, "advice {v}");
            // Only the low 32 bits count, as with SCMP_CMP_MASKED_EQ.
            let high = (0xffff_ffffu64 << 32) | v as u64;
            assert_eq!(
                eval(&prog, SYS_MADVISE, high),
                want,
                "advice {v} + high bits"
            );
        }
    }

    #[test]
    fn agrees_with_the_data_for_generic_kernels() {
        for k in crate::GENERIC_KERNELS {
            assert_matches_data(&Target::kernel(kv(k)));
        }
        assert_matches_data(&Target::kernel(table_ceiling()));
    }

    #[test]
    fn agrees_with_the_data_for_distro_backports() {
        for d in crate::distros() {
            assert_matches_data(&Target::distro(&d));
        }
    }

    #[test]
    fn blocks_other_architectures_and_x32() {
        let prog = program(&Target::kernel(table_ceiling()));
        let enosys = RET_ERRNO | ENOSYS as u32;
        const AUDIT_ARCH_I386: u32 = 0x4000_0003;
        assert_eq!(evaluate(&prog, AUDIT_ARCH_I386, 0, [0; 6]), Ok(enosys));
        assert_eq!(eval(&prog, X32_SYSCALL_BIT, 0), enosys);
        assert_eq!(eval(&prog, X32_SYSCALL_BIT | 1, 0), enosys);
        assert_eq!(eval(&prog, u32::MAX, 0), enosys);
        assert_eq!(eval(&prog, 0, 0), RET_ALLOW); // read
    }

    #[test]
    fn stays_well_under_the_instruction_limit() {
        let len = program(&Target::kernel(kv("3.10"))).len();
        assert!(len < 512, "{len} instructions");
    }

    #[test]
    fn instruction_layout_matches_sock_filter() {
        assert_eq!(std::mem::size_of::<Insn>(), 8);
    }
}

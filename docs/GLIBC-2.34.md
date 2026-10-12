# glibc 2.34 provider

The experimental x86_64 Linux provider selects GNU glibc 2.34 explicitly.
Matching headers, CRT objects, nonshared archives, loader, and runtime DSOs come
from a pinned source build. This is a generic GNU environment; distribution and
old-kernel qualification remain separate tasks.

Inputs are pinned in [INPUTS.toml](../examples/glibc-2.34/INPUTS.toml):

- Official GNU `glibc-2.34.tar.xz`, SHA-256
  `44d26a1fe20b8853a48f470ead01e4279e869ac149b195dda4e44a195d981ab2`.
  On 2026-10-11 its detached signature verified with `gpgv` against the GNU
  keyring (RSA key `16792B4EA25340F8`, Carlos O'Donell). CI checks the SHA-256 only.
- Builder `docker.io/library/debian@sha256:16ee3e11da473f6565ef94a715f1a38b3079e0664913be78869c336a71cb6085` with packages from
  http://archive.debian.org/debian stretch main (signed Release; Valid-Until not enforced): GCC 6.3.0-18+deb9u1, binutils 2.28-5, kernel headers linux-libc-dev 4.9.228-1.
- `--prefix=/usr --libdir=/usr/lib64 --disable-werror --enable-kernel=3.2`; no source patches.
- Compiler runtime: Link: generated libgcc_s stub with GCC 4.8.5's interface; tests run with the real Debian 9 GCC 6.3 libgcc_s.so.1 (requires GLIBC_2.14 at most).

| Library | Public numeric GLIBC definitions |
| --- | ---: |
| c | 2682 |
| m | 1089 |
| ld | 4 |
| resolv | 67 |
| pthread | 12 |
| dl | 3 |
| rt | 5 |
| util | 1 |
| **Total** | **3863** |

All eight native DSOs match GNU readelf and the pinned snapshot with zero ABI
differences. The stub audit preserves measured defaults, compatibility versions,
native types, binding/visibility, object sizes, alignment, and shared storage.
Exhaustive isolated-library checks pass **3863 accepted versioned
interfaces, 808 rejected interfaces, 3162 ordinary default bindings,
and 411 compatibility-only names rejected for new unversioned references**.

## Changes from 2.33

The snapshot adds 474 and removes 311 interfaces in this scope:
Added: `_Fork@GLIBC_2.34` (c), `__dn_comp@GLIBC_2.2.5` (c), `__dn_expand@GLIBC_2.2.5` (c), `__dn_skipname@GLIBC_2.2.5` (c), `__isnanf128@GLIBC_2.26` (c), `__isnanf128@GLIBC_2.34` (c), `__libc_start_main@GLIBC_2.34` (c), `__mq_open_2@GLIBC_2.34` (c), `__mq_open_2@GLIBC_2.7` (c), `__pthread_cleanup_routine@GLIBC_2.3.3` (c), `__pthread_cleanup_routine@GLIBC_2.34` (c), `__pthread_getspecific@GLIBC_2.2.5` (c), `__pthread_key_create@GLIBC_2.2.5` (c), `__pthread_key_create@GLIBC_2.34` (c), `__pthread_mutex_destroy@GLIBC_2.2.5` (c), `__pthread_mutex_init@GLIBC_2.2.5` (c), `__pthread_mutex_lock@GLIBC_2.2.5` (c), `__pthread_mutex_trylock@GLIBC_2.2.5` (c), `__pthread_mutex_unlock@GLIBC_2.2.5` (c), `__pthread_mutexattr_destroy@GLIBC_2.2.5` (c), `__pthread_mutexattr_init@GLIBC_2.2.5` (c), `__pthread_mutexattr_settype@GLIBC_2.2.5` (c), `__pthread_once@GLIBC_2.2.5` (c), `__pthread_register_cancel@GLIBC_2.3.3` (c), `__pthread_register_cancel@GLIBC_2.34` (c), `__pthread_register_cancel_defer@GLIBC_2.3.3` (c), `__pthread_register_cancel_defer@GLIBC_2.34` (c), `__pthread_rwlock_destroy@GLIBC_2.2.5` (c), `__pthread_rwlock_init@GLIBC_2.2.5` (c), `__pthread_rwlock_rdlock@GLIBC_2.2.5` (c), `__pthread_rwlock_tryrdlock@GLIBC_2.2.5` (c), `__pthread_rwlock_trywrlock@GLIBC_2.2.5` (c), `__pthread_rwlock_unlock@GLIBC_2.2.5` (c), `__pthread_rwlock_wrlock@GLIBC_2.2.5` (c), `__pthread_setspecific@GLIBC_2.2.5` (c), `__pthread_unregister_cancel@GLIBC_2.3.3` (c), `__pthread_unregister_cancel@GLIBC_2.34` (c), `__pthread_unregister_cancel_restore@GLIBC_2.3.3` (c), `__pthread_unregister_cancel_restore@GLIBC_2.34` (c), `__pthread_unwind_next@GLIBC_2.3.3` (c), and 434 more.
Removed: `malloc_get_state@GLIBC_2.2.5` (c), `malloc_set_state@GLIBC_2.2.5` (c), `dladdr@GLIBC_2.2.5` (dl), `dladdr1@GLIBC_2.3.3` (dl), `dlclose@GLIBC_2.2.5` (dl), `dlerror@GLIBC_2.2.5` (dl), `dlinfo@GLIBC_2.3.3` (dl), `dlmopen@GLIBC_2.3.4` (dl), `dlopen@GLIBC_2.2.5` (dl), `dlsym@GLIBC_2.2.5` (dl), `dlvsym@GLIBC_2.2.5` (dl), `__isnanf128@GLIBC_2.26` (m), `_IO_flockfile@GLIBC_2.2.5` (pthread), `_IO_ftrylockfile@GLIBC_2.2.5` (pthread), `_IO_funlockfile@GLIBC_2.2.5` (pthread), `__close@GLIBC_2.2.5` (pthread), `__connect@GLIBC_2.2.5` (pthread), `__errno_location@GLIBC_2.2.5` (pthread), `__fcntl@GLIBC_2.2.5` (pthread), `__fork@GLIBC_2.2.5` (pthread), `__h_errno_location@GLIBC_2.2.5` (pthread), `__libc_allocate_rtsig@GLIBC_2.2.5` (pthread), `__libc_current_sigrtmax@GLIBC_2.2.5` (pthread), `__libc_current_sigrtmin@GLIBC_2.2.5` (pthread), `__lseek@GLIBC_2.2.5` (pthread), `__open@GLIBC_2.2.5` (pthread), `__open64@GLIBC_2.2.5` (pthread), `__pread64@GLIBC_2.2.5` (pthread), `__pthread_cleanup_routine@GLIBC_2.3.3` (pthread), `__pthread_getspecific@GLIBC_2.2.5` (pthread), `__pthread_key_create@GLIBC_2.2.5` (pthread), `__pthread_mutex_destroy@GLIBC_2.2.5` (pthread), `__pthread_mutex_init@GLIBC_2.2.5` (pthread), `__pthread_mutex_lock@GLIBC_2.2.5` (pthread), `__pthread_mutex_trylock@GLIBC_2.2.5` (pthread), `__pthread_mutex_unlock@GLIBC_2.2.5` (pthread), `__pthread_mutexattr_destroy@GLIBC_2.2.5` (pthread), `__pthread_mutexattr_init@GLIBC_2.2.5` (pthread), `__pthread_mutexattr_settype@GLIBC_2.2.5` (pthread), `__pthread_once@GLIBC_2.2.5` (pthread), and 271 more.

Measured default roles changed for 10 interfaces (`yes` is the default `@@` version, `no` a compatibility `@` version): `__after_morecore_hook@GLIBC_2.2.5` (c) yes to no, `__default_morecore@GLIBC_2.2.5` (c) yes to no, `__free_hook@GLIBC_2.2.5` (c) yes to no, `__libc_start_main@GLIBC_2.2.5` (c) yes to no, `__malloc_hook@GLIBC_2.2.5` (c) yes to no, `__memalign_hook@GLIBC_2.2.5` (c) yes to no, `__morecore@GLIBC_2.2.5` (c) yes to no, `__realloc_hook@GLIBC_2.2.5` (c) yes to no, `mallwatch@GLIBC_2.2.5` (c) yes to no, `tr_break@GLIBC_2.2.5` (c) yes to no.
Measured symbol types changed for `__sched_cpucount@GLIBC_2.6` (c) IFUNC to FUNC, `roundeven@GLIBC_2.25` (m) FUNC to IFUNC, `roundevenf@GLIBC_2.25` (m) FUNC to IFUNC, `roundevenf32@GLIBC_2.27` (m) FUNC to IFUNC, `roundevenf32x@GLIBC_2.27` (m) FUNC to IFUNC, `roundevenf64@GLIBC_2.27` (m) FUNC to IFUNC.
22 interfaces changed between GLOBAL and WEAK binding; the stubs preserve
these, and the exhaustive default checks cover the resulting bindings.

2.34 merged libpthread, libdl, librt, and libutil into libc. The snapshot records 474 added and 311
removed interfaces: most move into libc, usually under a new `GLIBC_2.34` default, while the old
version stays available for existing binaries. The separate libraries remain as small placeholders
(12, 3, 5, and 1 public definitions), so the eight-library provider contract still describes the real
layout and needed no migration. Every library is generated, audited, and exhaustively probed as before.

Because `__libc_start_main` also has a new `GLIBC_2.34` default, referenced by the release's `crt1.o`,
**every executable built with a 2.34 or later provider requires glibc 2.34 or newer**. This is correct
and expected; select an older provider for older targets. Two fixtures were made release-aware: the
pthread default-attribute boundary now expects whichever version the provider measures as default
(`GLIBC_2.34` in libc from 2.34), and the resolver behavior suite accepts the `dn_expand` spelling that
2.34 headers use. The malloc hooks (`__malloc_hook` and similar) became compatibility-only.

## Applications and Cargo

Real-runtime tests pass allocation, threads/TLS destruction, unwinding, historical
data views, memcpy and fmemopen compatibility behavior, the pthread default-attribute
and sigvec boundary fixtures (each following this provider's evidence), and the five
dedicated math/loading/realtime/utility/resolver suites. Cargo preserves host build
scripts and proc macros, audits target link/runtime paths, and rejects mismatched
release selection. The cargo-libc build, tier 1 scan, tier 2 local runner under a
Linux 3.10 filter, and `check` all pass. The too-new link tests use `__memcmpeq`,
chosen from the matrix (`too-new-symbol.py`), and fail at the linker as required.
The Rust test program's highest directly imported GLIBC tag was 2.34; this is an
import measurement, not a general minimum-runtime guarantee.

## Reproduction

```bash
export ABI_GLIBC_RELEASE=2.34
export RUSTUP_TOOLCHAIN=1.99.0
export ABI_BEHAVIOR_CC=/usr/bin/clang-24
bash examples/glibc-2.34/run.sh /tmp/glibc-2.34-work
bash examples/glibc-two-paths/elf-and-check.sh /tmp/glibc-2.34-work
bash examples/glibc-two-paths/prepare-and-check.sh /tmp/glibc-2.34-work
bash examples/glibc-two-paths/application-and-check.sh /tmp/glibc-2.34-work
bash examples/glibc-two-paths/provider-and-check.sh \
  /tmp/glibc-2.34-work ../cargo-libc /usr/bin/clang-24
```

Use a new absolute work directory. Local validation on 2026-10-11 used Rust 1.99.0,
Clang 18.1.3, GNU binutils 2.42, and podman 5.8.2 on WSL Ubuntu 24.04, with
cargo-libc `bfe04693f6685770b47c9346e32de556a8029f4e`. The measured profile `gnu-source-2.34` is
committed to [history provenance](../data/glibc-history/PROVENANCE.json) and
[the interface reference](reference/GLIBC-SYMBOLS.md).

Both Clang 21/24 jobs for 2.34 passed in the [complete 84-job hosted run](https://github.com/rustcommons/linux-abi-tools/actions/runs/38158161339) at
producer `ae68bf63f5ddde7fc7852e76eab83ad2c1beb2c0` with consumer `bfe04693f6685770b47c9346e32de556a8029f4e` on 2026-10-11. Distribution/kernel
qualification, native dependency adapters, and authenticated SDK packaging remain open.

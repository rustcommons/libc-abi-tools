# Generic GNU providers and distribution glibc

The providers in [PROVIDERS.md](PROVIDERS.md) are built from unmodified GNU glibc
release archives with upstream's default configuration. A distribution's glibc is
the same release with that distribution's configuration and patches. The two can
differ in which interfaces new code may link, in their code, and occasionally in
the interfaces they export. This document records the differences we have
measured and what we know they imply. A difference that has not been measured is
listed as a possibility, not as a fact.

## Measured: CentOS 7.9 packages versus GNU source, glibc 2.17

glibc 2.17 is the only release with both a distribution input (CentOS 7.9's
`glibc-2.17-317.el7` packages from the 7.9.2009 vault, the `prebuilt` route) and a GNU source build.
Both profiles are in [elf-evidence.tsv](../data/glibc-history/elf-evidence.tsv),
`centos-7.9-prebuilt-2.17` and `gnu-source-2.17`.

| Property | Result |
| --- | --- |
| Exported versioned interfaces (eight libraries) | Identical: 2915 in both, no type or object-size differences |
| Default (`@@`) versus compatibility-only (`@`) role | 162 differ (below) |
| Symbol binding (GLOBAL/WEAK) | 20 differ, all in libc except libpthread's `pthread_once`; no effect on which version a reference binds to |
| Function sizes | 647 differ: CentOS's patches change the code, not the interface |

The 162 role differences:

- **152 in libc, all Sun RPC:** the XDR (69), `svc*` (26), key/DES (26), `clnt*` (12),
  `pmap*` (5) and `auth*` (4) families plus `callrpc`, `registerrpc`, `get_myaddress`,
  `rtime`, `_rpc_dtablesize`, `_seterr_reply` and the `__rpc_thread_*` accessors.
  CentOS built glibc with obsolete RPC enabled, so new code may link them; the GNU
  default leaves them compatibility-only.
- **10 in libresolv:** `_gethtbyaddr`, `_gethtbyname`, `_gethtbyname2`, `_gethtent`,
  `_sethtent`, `res_gethostbyaddr`, `res_gethostbyname`, `res_gethostbyname2`,
  `res_send_setqhook` and `res_send_setrhook` (all `GLIBC_2.2.5`). CentOS makes them
  compatibility-only; the GNU source build leaves them linkable.

What this means for a program built with the generic 2.17 provider:

- A program that calls Sun RPC fails **at link time** with the generic provider,
  although CentOS 7's own toolchain would link it. The result errs on the safe side;
  use the `prebuilt` route (CentOS packages) for such programs.
- A program that calls one of the ten resolver functions links with the generic
  provider, although CentOS 7's own toolchain would refuse. The executable still
  **loads and runs** on CentOS 7: the loader resolves a versioned reference to a
  compatibility-only definition, and CentOS still exports these versions.
- None of the measured differences makes an executable fail to load on CentOS 7.
  Behavior inside the 647 changed functions (bug and security fixes) follows the
  target system's own glibc at run time, because the provider is used only to link.

## Not measured: other distributions and releases

No other distribution input has been collected, so its differences are unknown.
Distributions commonly differ from the GNU build in these ways:

- **Configuration:** for example obsolete RPC or NSL support, which changes
  default roles as above.
- **Backported fixes:** these change code but rarely interfaces.
- **Backported interfaces:** a distribution may add a newer interface or version node
  to an older release. The generic provider would then reject a program that the
  distribution's toolchain links. The rejection happens at link time, which errs on
  the safe side.
- **Removed or restricted interfaces:** if a distribution stopped exporting a version
  that the GNU release defines, a program linked against the generic provider could
  fail to load there. This is the only direction that breaks at run time, and none
  has been observed.

To check a target, read its real libraries with `libc-abi glibc elf --input PATH`
(for example `/lib64/libc.so.6` on the target or in its container image) and compare
the definitions with the provider's measured profile. Then run the built
executable on the target, or with `cargo libc test --runner container --image IMAGE`.

## Licensing if distribution inputs are ever packaged

Today providers are assembled on the user's or CI machine from downloaded inputs;
this project does not redistribute glibc, kernel-header, or GCC binaries. A future
release that ships provider SDKs, generic or distribution-specific, would
redistribute them and must at least:

- **glibc (LGPL-2.1-or-later):** ship or offer the corresponding source. For a
  distribution input that means the distribution's source package with its patches,
  not only the GNU archive. The startup objects and `libc_nonshared.a` are linked
  into user programs under glibc's linking terms, so keep their notices.
- **Kernel UAPI headers (GPL-2.0 with the Linux-syscall-note):** keep the license and
  note with the headers.
- **`libgcc_s` (GPL-3.0 with the GCC Runtime Library Exception):** keep the license and
  exception text.
- **Distribution terms, which go beyond the code licenses:** Debian and Ubuntu packages
  may be redistributed. Red Hat Enterprise Linux and SUSE Linux Enterprise binaries
  are obtained under subscription terms that restrict redistribution. Red Hat's
  Universal Base Image and SUSE's Base Container Images are offered for
  redistribution under their own terms, and Rocky Linux, AlmaLinux and the CentOS
  vault are freely redistributable rebuilds. Record which applies to each input.
- **Trademarks:** name inputs after their actual source (for example `alma-8` or
  `ubi-8`) rather than implying an official vendor product.
- **Per-package records:** pin each package's version and checksum and record its
  license, as [glibc-abilists](https://github.com/rustcommons/glibc-abilists) does
  for the ABI lists.

Distribution-specific providers are planned for a future release; see the
[roadmap](ROADMAP.md#future-release-distribution-specific-providers).

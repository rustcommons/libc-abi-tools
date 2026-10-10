#!/bin/bash
# Template copied to the Linux experiment directory, never installed globally.
set -euo pipefail
root=${ABI_REAL_ROOT:?}
generated=${ABI_STUB_ROOT:?}
runtime=${ABI_COMPILER_RUNTIME:?}
printf '%q\n' "$@" > "${ABI_LINK_ARGUMENTS:?}"
exec ld --sysroot="$root" -L"$generated" -L"$runtime" \
    "$root/usr/lib64/crt1.o" "$root/usr/lib64/crti.o" "$(gcc -print-file-name=crtbegin.o)" \
    "$@" "$root/usr/lib64/libc_nonshared.a" "$root/usr/lib64/libpthread_nonshared.a" \
    "$(gcc -print-file-name=crtend.o)" "$root/usr/lib64/crtn.o" \
    --dynamic-linker /lib64/ld-linux-x86-64.so.2 -Map="${ABI_LINK_MAP:?}"

#!/bin/bash
# Template copied to the Linux experiment directory, never installed globally.
set -euo pipefail
root=${ABI_REAL_ROOT:?}
generated=${ABI_STUB_ROOT:?}
runtime=${ABI_COMPILER_RUNTIME:?}
printf '%q\n' "$@" > "${ABI_LINK_ARGUMENTS:?}"
# glibc 2.28 merged libpthread_nonshared.a into libc_nonshared.a.
nonshared=("$root/usr/lib64/libc_nonshared.a")
if [ -e "$root/usr/lib64/libpthread_nonshared.a" ]; then
    nonshared+=("$root/usr/lib64/libpthread_nonshared.a")
fi
exec ld --sysroot="$root" -L"$generated" -L"$runtime" \
    "$root/usr/lib64/crt1.o" "$root/usr/lib64/crti.o" "$(gcc -print-file-name=crtbegin.o)" \
    "$@" "${nonshared[@]}" \
    "$(gcc -print-file-name=crtend.o)" "$root/usr/lib64/crtn.o" \
    --dynamic-linker /lib64/ld-linux-x86-64.so.2 -Map="${ABI_LINK_MAP:?}"

#!/bin/bash
set -euo pipefail
cd "${1:?Usage: link-and-check.sh EXPERIMENT_DIRECTORY}"
shift
if [ "$#" -eq 0 ]; then set -- prebuilt source; fi
mkdir -p logs
gcc --version > logs/host-gcc.txt
ld --version > logs/host-ld.txt
uname -a > logs/host-kernel.txt
gcc -O0 -fno-pie -no-pie -fno-stack-protector hello.c -o hello-host
readelf --version-info hello-host > logs/host-versions.txt
for route in "$@"; do
    root="$PWD/$route"
    gcc -O0 -fno-pie -fno-stack-protector -nostdinc \
        -isystem "$root/usr/include" -isystem "$(gcc -print-file-name=include)" \
        -c hello.c -o "hello-$route.o"
    ld --sysroot="$root" -o "hello-$route" \
        --dynamic-linker /lib64/ld-linux-x86-64.so.2 \
        "$root/usr/lib64/crt1.o" "$root/usr/lib64/crti.o" \
        "$(gcc -print-file-name=crtbegin.o)" "hello-$route.o" \
        -L"$root/usr/lib64" -L"$root/lib64" -lc \
        "$(gcc -print-file-name=crtend.o)" "$root/usr/lib64/crtn.o" \
        -Map="logs/$route-link.map"
    readelf --version-info "hello-$route" > "logs/$route-versions.txt"
    readelf -lW "hello-$route" > "logs/$route-program-headers.txt"
    readelf -dW "hello-$route" > "logs/$route-dynamic.txt"
    objdump -T "hello-$route" > "logs/$route-symbols.txt"
    "$root/lib64/ld-linux-x86-64.so.2" \
        --library-path "$root/lib64:$root/usr/lib64" "./hello-$route" \
        > "logs/$route-run.txt"
    cat "logs/$route-run.txt"
    grep -q 'Hello from the glibc compatibility experiment!' "logs/$route-run.txt"
done
# The native control should fail specifically because it requires newer glibc.
if prebuilt/lib64/ld-linux-x86-64.so.2 \
    --library-path prebuilt/lib64:prebuilt/usr/lib64 ./hello-host \
    > logs/host-on-old-runtime.txt 2>&1; then
    echo 'Control unexpectedly ran: this host may not be newer than glibc 2.17.' >&2
    exit 1
fi
grep -q "GLIBC_.*not found" logs/host-on-old-runtime.txt
objdump -T hello-host > logs/host-symbols.txt

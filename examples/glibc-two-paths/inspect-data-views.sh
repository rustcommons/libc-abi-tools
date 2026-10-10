#!/bin/bash
# Real-library control for historical data-view checks.
set -euo pipefail
example_dir=$(cd "$(dirname "$0")" && pwd)
cd "${1:?Usage: inspect-data-views.sh EXPERIMENT_DIRECTORY}"
work=$PWD
for route in prebuilt source; do
    root="$work/$route"
    gcc -O0 -fno-builtin -fno-pie -fno-stack-protector -nostdinc \
        -isystem "$root/usr/include" -isystem "$(gcc -print-file-name=include)" \
        -c "$example_dir/data-views.c" -o "data-views-control-$route.o"
    ld --sysroot="$root" -o "data-views-real-$route" --dynamic-linker /lib64/ld-linux-x86-64.so.2 \
        "$root/usr/lib64/crt1.o" "$root/usr/lib64/crti.o" "$(gcc -print-file-name=crtbegin.o)" \
        "data-views-control-$route.o" -L"$root/usr/lib64" -L"$root/lib64" -lc \
        "$(gcc -print-file-name=crtend.o)" "$root/usr/lib64/crtn.o"
    if "$root/lib64/ld-linux-x86-64.so.2" --library-path "$root/lib64:$root/usr/lib64" \
        "./data-views-real-$route" > "logs/full-$route-data-views-real-control.txt" 2>&1; then
        echo "$route real-library control passed"
    else
        status=$?
        echo "$route real-library control failed with exit $status"
    fi
done

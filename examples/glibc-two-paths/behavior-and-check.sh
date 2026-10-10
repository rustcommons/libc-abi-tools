#!/bin/bash
set -euo pipefail
example_dir=$(cd "$(dirname "$0")" && pwd)
source "$example_dir/experiment.sh"
cd "${1:?Usage: behavior-and-check.sh EXISTING_EXPERIMENT_DIRECTORY}"
work=$PWD
compiler=${ABI_BEHAVIOR_CC:-gcc}
unset LD_PRELOAD LD_AUDIT LD_LIBRARY_PATH
mkdir -p logs behavior
"$compiler" --version > logs/behavior-compiler.txt
for route in "${routes[@]}"; do
    root="$work/$route"
    generated="$work/full-libraries-$route"
    destination="$work/behavior/$route"
    mkdir -p "$destination"
    "$compiler" -Wall -Wextra -Werror -fPIC -fno-stack-protector -c "$example_dir/behavior-plugin.c" -o "$destination/plugin.o"
    ld -shared -z defs --build-id=none --version-script="$example_dir/behavior-plugin.map" \
        "$destination/plugin.o" -o "$destination/plugin.so"
    readelf --wide --dyn-syms --dynamic "$destination/plugin.so" > "$work/logs/behavior-$route-plugin-elf.txt"
    for library in m dl rt util resolv; do
        "$compiler" -O0 -Wall -Wextra -Werror -fno-builtin -fno-pie -fno-stack-protector -nostdinc \
            -isystem "$root/usr/include" -isystem "$(gcc -print-file-name=include)" \
            -D"TEST_${library^^}" -c "$example_dir/library-behavior.c" -o "$destination/$library.o"
        ld -o "$destination/$library" --dynamic-linker /lib64/ld-linux-x86-64.so.2 \
            "$root/usr/lib64/crt1.o" "$root/usr/lib64/crti.o" "$(gcc -print-file-name=crtbegin.o)" \
            "$destination/$library.o" -L"$generated" -l"$library" -lc \
            "$root/usr/lib64/libc_nonshared.a" "$(gcc -print-file-name=crtend.o)" "$root/usr/lib64/crtn.o" \
            -Map="$work/logs/behavior-$route-$library.map"
        readelf --wide --dyn-syms --dynamic --version-info "$destination/$library" \
            > "$work/logs/behavior-$route-$library-elf.txt"
        "$root/lib64/ld-linux-x86-64.so.2" --library-path "$root/lib64:$root/usr/lib64" \
            --list "$destination/$library" > "$work/logs/behavior-$route-$library-libraries.txt"
        timeout 20 "$root/lib64/ld-linux-x86-64.so.2" --library-path "$root/lib64:$root/usr/lib64" \
            "$destination/$library" "$destination/plugin.so" > "$work/logs/behavior-$route-$library-run.txt"
        grep -qx "$library behavior passed" "$work/logs/behavior-$route-$library-run.txt"
    done
done
python3 "$example_dir/check-behavior.py" "$work"

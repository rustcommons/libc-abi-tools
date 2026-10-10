#!/bin/bash
# Reuse the two real input trees. Requires Cargo, GCC, GNU binutils, Python 3.
set -euo pipefail
example_dir=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$example_dir/../.." && pwd)
work=${1:?Usage: stub-and-check.sh ABSOLUTE_EXISTING_EXPERIMENT_DIRECTORY}
cd "$work"
work=$PWD
mkdir -p logs
# Avoid environment overrides during the explicit old-runtime checks.
unset LD_PRELOAD LD_AUDIT LD_LIBRARY_PATH
export CARGO_TARGET_DIR="$work/rust-target"
cargo run --manifest-path "$repo/Cargo.toml" --locked --offline \
    --example minimal_glibc_stub -- "$repo/data/glibc" "$work/prebuilt/lib64/libc.so.6" "$work/generated-stub"
as --64 generated-stub/libc.S -o generated-stub/libc.o
ld -shared --build-id=none -soname libc.so.6 \
    --version-script=generated-stub/libc.map \
    generated-stub/libc.o -o generated-stub/libc.so.6
readelf --dyn-syms --wide generated-stub/libc.so.6 > logs/stub-exports.txt
readelf --version-info generated-stub/libc.so.6 > logs/stub-versions.txt
readelf -dW generated-stub/libc.so.6 > logs/stub-dynamic.txt
# Assemble/link a second copy and compare bytes to check this toolchain's output.
as --64 generated-stub/libc.S -o generated-stub/repeat.o
ld -shared --build-id=none -soname libc.so.6 \
    --version-script=generated-stub/libc.map \
    generated-stub/repeat.o -o generated-stub/repeat.so
cmp generated-stub/libc.so.6 generated-stub/repeat.so

# Demonstrate that memfd_create is newer using the pinned ABI snapshots.
cargo run --manifest-path "$repo/Cargo.toml" --locked --offline -- \
    glibc snapshot --data "$repo/data/glibc" --release 2.17 --symbol memfd_create \
    > logs/memfd-2.17.tsv
cargo run --manifest-path "$repo/Cargo.toml" --locked --offline -- \
    glibc snapshot --data "$repo/data/glibc" --release 2.27 --symbol memfd_create \
    > logs/memfd-2.27.tsv
test "$(wc -l < logs/memfd-2.17.tsv)" -eq 1
grep -q GLIBC_2.27 logs/memfd-2.27.tsv

for route in prebuilt source; do
    root="$work/$route"
    for program in hello too-new; do
        gcc -O0 -fno-pie -fno-stack-protector -nostdinc \
            -isystem "$root/usr/include" -isystem "$(gcc -print-file-name=include)" \
            -c "$example_dir/$program.c" -o "$program-stub-$route.o"
    done
    inputs=("$root/usr/lib64/crt1.o" "$root/usr/lib64/crti.o"
            "$(gcc -print-file-name=crtbegin.o)")
    libraries=(-L"$work/generated-stub" -l:libc.so.6 "$root/usr/lib64/libc_nonshared.a"
               "$(gcc -print-file-name=crtend.o)" "$root/usr/lib64/crtn.o")
    ld -o "hello-stub-$route" --dynamic-linker /lib64/ld-linux-x86-64.so.2 \
        "${inputs[@]}" "hello-stub-$route.o" "${libraries[@]}" \
        -Map="logs/stub-$route-link.map"
    readelf --version-info "hello-stub-$route" > "logs/stub-$route-versions.txt"
    objdump -T "hello-stub-$route" > "logs/stub-$route-symbols.txt"
    "$root/lib64/ld-linux-x86-64.so.2" --library-path "$root/lib64:$root/usr/lib64" \
        --list "./hello-stub-$route" > "logs/stub-$route-runtime-libraries.txt"
    "$root/lib64/ld-linux-x86-64.so.2" --library-path "$root/lib64:$root/usr/lib64" \
        "./hello-stub-$route" > "logs/stub-$route-run.txt"
    grep -qx 'Hello from the glibc compatibility experiment!' "logs/stub-$route-run.txt"
    if ld -o "too-new-stub-$route" --dynamic-linker /lib64/ld-linux-x86-64.so.2 \
        "${inputs[@]}" "too-new-stub-$route.o" "${libraries[@]}" \
        > "logs/stub-$route-negative.txt" 2>&1; then
        echo 'Unexpected success linking memfd_create.' >&2; exit 1
    fi
    grep -q "undefined reference to .*memfd_create" "logs/stub-$route-negative.txt"
done
python3 "$example_dir/check-stub.py" "$work" | tee logs/stub-verification.txt

#!/bin/bash
set -euo pipefail
example_dir=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$example_dir/../.." && pwd)
cd "${1:?Usage: expanded-and-check.sh EXISTING_EXPERIMENT_DIRECTORY}"
work=$PWD
export CARGO_TARGET_DIR="$work/rust-target"
unset LD_PRELOAD LD_AUDIT LD_LIBRARY_PATH
mkdir -p logs
for route in prebuilt source; do
    root="$work/$route"
    generated="$work/expanded-stub-$route"
    cargo run --manifest-path "$repo/Cargo.toml" --locked --offline --example expanded_glibc_stub -- \
        "$repo/data/glibc" "$root/lib64/libc.so.6" "$generated"
    as --64 "$generated/libc.S" -o "$generated/libc.o"
    ld -shared --build-id=none -soname libc.so.6 --version-script="$generated/libc.map" \
        "$generated/libc.o" -o "$generated/libc.so.6"
    for program in expanded too-new; do
        gcc -O0 -fno-builtin -fno-pie -fno-stack-protector -nostdinc \
            -isystem "$root/usr/include" -isystem "$(gcc -print-file-name=include)" \
            -c "$example_dir/$program.c" -o "$program-expanded-$route.o"
    done
    inputs=("$root/usr/lib64/crt1.o" "$root/usr/lib64/crti.o" "$(gcc -print-file-name=crtbegin.o)")
    libraries=(-L"$generated" -l:libc.so.6 "$root/usr/lib64/libc_nonshared.a"
               "$(gcc -print-file-name=crtend.o)" "$root/usr/lib64/crtn.o")
    ld -o "expanded-$route" --dynamic-linker /lib64/ld-linux-x86-64.so.2 \
        "${inputs[@]}" "expanded-expanded-$route.o" "${libraries[@]}" \
        -Map="logs/expanded-$route-link.map"
    readelf --dyn-syms --wide --version-info --relocs --dynamic "expanded-$route" \
        > "logs/expanded-$route-executable.txt"
    "$root/lib64/ld-linux-x86-64.so.2" --library-path "$root/lib64:$root/usr/lib64" \
        --list "./expanded-$route" > "logs/expanded-$route-runtime-libraries.txt"
    "$root/lib64/ld-linux-x86-64.so.2" --library-path "$root/lib64:$root/usr/lib64" \
        "./expanded-$route" > "logs/expanded-$route-run.txt"
    grep -qx 'Both memcpy versions and stdout work!' "logs/expanded-$route-run.txt"
    if ld -o "too-new-expanded-$route" --dynamic-linker /lib64/ld-linux-x86-64.so.2 \
        "${inputs[@]}" "too-new-expanded-$route.o" "${libraries[@]}" \
        > "logs/expanded-$route-negative.txt" 2>&1; then
        echo 'Unexpected success linking memfd_create.' >&2; exit 1
    fi
    grep -q 'undefined reference to .*memfd_create' "logs/expanded-$route-negative.txt"
done
cargo run --manifest-path "$repo/Cargo.toml" --locked --offline --example fixture_stub -- \
    "$repo/tests/fixtures/elf/symbols.so" "$work/expanded-fixture"
as --64 expanded-fixture/fixture.S -o expanded-fixture/fixture.o
ld -shared --build-id=none -soname libfixture.so.1 --version-script=expanded-fixture/fixture.map \
    expanded-fixture/fixture.o -o expanded-fixture/libfixture.so.1
mkdir -p fixture-runtime
cp "$repo/tests/fixtures/elf/symbols.so" fixture-runtime/libfixture.so.1
for route in prebuilt source; do
    root="$work/$route"
    gcc -O0 -fno-builtin -fno-pie -fno-stack-protector -nostdinc \
        -isystem "$root/usr/include" -isystem "$(gcc -print-file-name=include)" \
        -c "$example_dir/fixture-runtime.c" -o "fixture-expanded-$route.o"
    ld --export-dynamic -o "fixture-$route" --dynamic-linker /lib64/ld-linux-x86-64.so.2 \
        "$root/usr/lib64/crt1.o" "$root/usr/lib64/crti.o" "$(gcc -print-file-name=crtbegin.o)" \
        "fixture-expanded-$route.o" -L"$work/expanded-fixture" -l:libfixture.so.1 \
        -L"$work/expanded-stub-$route" -l:libc.so.6 "$root/usr/lib64/libc_nonshared.a" \
        "$(gcc -print-file-name=crtend.o)" "$root/usr/lib64/crtn.o" \
        -Map="logs/expanded-fixture-$route-link.map"
    "$root/lib64/ld-linux-x86-64.so.2" \
        --library-path "$work/fixture-runtime:$root/lib64:$root/usr/lib64" \
        --list "./fixture-$route" > "logs/expanded-fixture-$route-runtime-libraries.txt"
    "$root/lib64/ld-linux-x86-64.so.2" \
        --library-path "$work/fixture-runtime:$root/lib64:$root/usr/lib64" \
        "./fixture-$route" > "logs/expanded-fixture-$route-run.txt"
    grep -qx 'Fixture versions, IFUNC, TLS, and alias storage work!' "logs/expanded-fixture-$route-run.txt"
    readelf --dyn-syms --dynamic --relocs --wide "fixture-$route" \
        > "logs/expanded-fixture-$route-executable.txt"
done
python3 "$example_dir/check-expanded.py" "$work" "$repo" | tee logs/expanded-verification.txt

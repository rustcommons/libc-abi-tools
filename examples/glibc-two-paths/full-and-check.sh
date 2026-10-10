#!/bin/bash
set -euo pipefail
example_dir=$(cd "$(dirname "$0")" && pwd)
source "$example_dir/experiment.sh"
repo=$(cd "$example_dir/../.." && pwd)
cd "${1:?Usage: full-and-check.sh EXISTING_EXPERIMENT_DIRECTORY}"
work=$PWD
export CARGO_TARGET_DIR="$work/rust-target"
unset LD_PRELOAD LD_AUDIT LD_LIBRARY_PATH
mkdir -p logs compiler-runtime
# Explicit compatible real unwinder, not the host's newer libgcc_s.
if [ -e compiler-runtime/libgcc_s.so.1 ] || [ -L compiler-runtime/libgcc_s.so.1 ]; then
    cmp prebuilt/usr/lib64/libgcc_s.so.1 compiler-runtime/libgcc_s.so.1
else
    cp -L prebuilt/usr/lib64/libgcc_s.so.1 compiler-runtime/libgcc_s.so.1
fi
if [ -L compiler-runtime/libgcc_s.so ]; then
    test "$(readlink compiler-runtime/libgcc_s.so)" = libgcc_s.so.1
else
    test ! -e compiler-runtime/libgcc_s.so
    ln -s libgcc_s.so.1 compiler-runtime/libgcc_s.so
fi
cp "$example_dir/rust-linker.sh" full-rust-linker.sh
chmod +x full-rust-linker.sh
rustc --version > logs/full-rustc.txt
libraries=(c m ld resolv pthread dl rt util)
fmemopen_expectation=$(python3 "$example_dir/check-matrix.py" "$work/elf-metadata/matrix.tsv" "$release" c fmemopen GLIBC_2.22)
python3 "$example_dir/check-matrix.py" "$work/elf-metadata/matrix.tsv" "$release" c fmemopen GLIBC_2.2.5 accept
python3 "$example_dir/check-matrix.py" "$work/elf-metadata/matrix.tsv" "$release" c memfd_create GLIBC_2.27 reject
files=(libc.so.6 libm.so.6 ld-linux-x86-64.so.2 libresolv.so.2 libpthread.so.0 libdl.so.2 librt.so.1 libutil.so.1)
for route in "${routes[@]}"; do
    root="$work/$route"
    generated="$work/full-libraries-$route"
    cargo run --manifest-path "$repo/Cargo.toml" --locked --offline --example full_glibc_stubs -- \
        "$repo/data/glibc" "$root/lib64" "$generated" "$release" | tee "logs/full-$route-generation.txt"
    for index in "${!libraries[@]}"; do
        library=${libraries[$index]}
        soname=${files[$index]}
        as --64 "$generated/$library.S" -o "$generated/$library.o"
        ld -shared --build-id=none -soname "$soname" --version-script="$generated/$library.map" \
            "$generated/$library.o" -o "$generated/$soname"
        ln -s "$soname" "$generated/lib$library.so"
        readelf --wide --dyn-syms --dynamic --section-headers "$generated/$soname" \
            > "logs/full-$route-$library-exports.txt"
    done
    export ABI_REAL_ROOT="$root" ABI_STUB_ROOT="$generated" ABI_COMPILER_RUNTIME="$work/compiler-runtime"
    export ABI_LINK_ARGUMENTS="$work/logs/full-$route-rust-link-arguments.txt"
    export ABI_LINK_MAP="$work/logs/full-$route-rust-link.map"
    rustc "$example_dir/rust-hello.rs" --edition=2021 -C linker="$work/full-rust-linker.sh" \
        -C linker-flavor=ld -C relocation-model=static -C panic=unwind -o "rust-full-$route"
    readelf --wide --dyn-syms --version-info --dynamic "rust-full-$route" \
        > "logs/full-$route-rust-executable.txt"
    "$root/lib64/ld-linux-x86-64.so.2" --library-path "$work/compiler-runtime:$root/lib64:$root/usr/lib64" \
        --list "./rust-full-$route" > "logs/full-$route-rust-runtime-libraries.txt"
    "$root/lib64/ld-linux-x86-64.so.2" --library-path "$work/compiler-runtime:$root/lib64:$root/usr/lib64" \
        "./rust-full-$route" > "logs/full-$route-rust-run.txt"
    grep -qx 'Rust allocation, thread/TLS, and unwinding work!' "logs/full-$route-rust-run.txt"
    for program in expanded data-views too-new fmemopen-2.22 fmemopen-default; do
        gcc -O0 -fno-builtin -fno-pie -fno-stack-protector -nostdinc \
            -isystem "$root/usr/include" -isystem "$(gcc -print-file-name=include)" \
            -c "$example_dir/$program.c" -o "$program-full-$route.o"
    done
    inputs=("$root/usr/lib64/crt1.o" "$root/usr/lib64/crti.o" "$(gcc -print-file-name=crtbegin.o)")
    libs=(-L"$generated" -l:libc.so.6 "$root/usr/lib64/libc_nonshared.a"
          "$(gcc -print-file-name=crtend.o)" "$root/usr/lib64/crtn.o")
    ld -o "c-full-$route" --dynamic-linker /lib64/ld-linux-x86-64.so.2 \
        "${inputs[@]}" "expanded-full-$route.o" "${libs[@]}" -Map="logs/full-$route-c-link.map"
    "$root/lib64/ld-linux-x86-64.so.2" --library-path "$root/lib64:$root/usr/lib64" \
        "./c-full-$route" > "logs/full-$route-c-run.txt"
    grep -qx 'Both memcpy versions and stdout work!' "logs/full-$route-c-run.txt"
    ld -o "data-views-full-$route" --dynamic-linker /lib64/ld-linux-x86-64.so.2 \
        "${inputs[@]}" "data-views-full-$route.o" "${libs[@]}" -Map="logs/full-$route-data-views-link.map"
    "$root/lib64/ld-linux-x86-64.so.2" --library-path "$root/lib64:$root/usr/lib64" \
        "./data-views-full-$route" > "logs/full-$route-data-views-run.txt"
    grep -qx 'Historical array sizes and values work!' "logs/full-$route-data-views-run.txt"
    readelf --wide --dyn-syms --relocs --version-info "data-views-full-$route" \
        > "logs/full-$route-data-views-executable.txt"
    if ld -o "too-new-full-$route" --dynamic-linker /lib64/ld-linux-x86-64.so.2 \
        "${inputs[@]}" "too-new-full-$route.o" "${libs[@]}" \
        > "logs/full-$route-negative.txt" 2>&1; then
        echo 'Unexpected success linking memfd_create.' >&2; exit 1
    fi
    grep -q 'undefined reference to .*memfd_create' "logs/full-$route-negative.txt"
    ld -o "fmemopen-default-$route" --dynamic-linker /lib64/ld-linux-x86-64.so.2 \
        "${inputs[@]}" "fmemopen-default-full-$route.o" "${libs[@]}"
    "$root/lib64/ld-linux-x86-64.so.2" --library-path "$root/lib64:$root/usr/lib64" \
        "./fmemopen-default-$route" > "logs/full-$route-fmemopen-default-run.txt"
    grep -qx 'Default fmemopen interface works!' "logs/full-$route-fmemopen-default-run.txt"
    readelf --wide --dyn-syms --version-info "fmemopen-default-$route" \
        > "logs/full-$route-fmemopen-default-executable.txt"
    if [ "$fmemopen_expectation" = accept ]; then
        ld -o "fmemopen-full-$route" --dynamic-linker /lib64/ld-linux-x86-64.so.2 \
            "${inputs[@]}" "fmemopen-2.22-full-$route.o" "${libs[@]}"
        "$root/lib64/ld-linux-x86-64.so.2" --library-path "$root/lib64:$root/usr/lib64" \
            "./fmemopen-full-$route" > "logs/full-$route-fmemopen-run.txt"
        grep -qx 'Both fmemopen versions work!' "logs/full-$route-fmemopen-run.txt"
        readelf --wide --dyn-syms --version-info "fmemopen-full-$route" \
            > "logs/full-$route-fmemopen-executable.txt"
        grep -q 'fmemopen@GLIBC_2.22' "logs/full-$route-fmemopen-executable.txt"
        grep -q 'fmemopen@GLIBC_2.2.5' "logs/full-$route-fmemopen-executable.txt"
    else
        if ld -o "fmemopen-full-$route" --dynamic-linker /lib64/ld-linux-x86-64.so.2 \
            "${inputs[@]}" "fmemopen-2.22-full-$route.o" "${libs[@]}" \
            > "logs/full-$route-fmemopen-negative.txt" 2>&1; then
            echo '2.17 unexpectedly accepted fmemopen@GLIBC_2.22' >&2; exit 1
        fi
        grep -q 'undefined reference to .*fmemopen@GLIBC_2.22' "logs/full-$route-fmemopen-negative.txt"
    fi
done
python3 "$example_dir/check-full.py" "$work" "$repo"
python3 "$example_dir/check-link-matrix.py" "$work"

#!/bin/bash
set -euo pipefail
example_dir=$(cd "$(dirname "$0")" && pwd)
source "$example_dir/experiment.sh"
repo=$(cd "$example_dir/../.." && pwd)
cd "${1:?Usage: application-and-check.sh EXISTING_EXPERIMENT_DIRECTORY}"
work=$PWD
export CARGO_TARGET_DIR="$work/rust-target"
unset LD_PRELOAD LD_AUDIT LD_LIBRARY_PATH
mkdir -p logs compiler-runtime
compiler=${ABI_BEHAVIOR_CC:-gcc}
"$compiler" --version > logs/application-compiler.txt
cp "$example_dir/rust-linker.sh" full-rust-linker.sh
chmod +x full-rust-linker.sh
rustc --version > logs/full-rustc.txt
fmemopen_expectation=$(python3 "$example_dir/check-matrix.py" "$work/elf-metadata/matrix.tsv" "$release" c fmemopen GLIBC_2.22)
pthread_defaults_expectation=$(python3 "$example_dir/check-matrix.py" "$work/elf-metadata/matrix.tsv" "$release" pthread,c pthread_getattr_default_np GLIBC_2.18)
python3 "$example_dir/check-matrix.py" "$work/elf-metadata/matrix.tsv" "$release" pthread,c pthread_setattr_default_np GLIBC_2.18 "$pthread_defaults_expectation"
python3 "$example_dir/check-matrix.py" "$work/elf-metadata/matrix.tsv" "$release" c fmemopen GLIBC_2.2.5 accept
too_new=$(python3 "$example_dir/too-new-symbol.py" "$work/elf-metadata/matrix.tsv" "$release")
for route in "${routes[@]}"; do
    root="$work/$route"
    generated="$work/full-libraries-$route"
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
    for program in expanded data-views too-new fmemopen-2.22 fmemopen-default pthread-default-2.18 sigvec-default-2.21; do
        "$compiler" -O0 -fno-builtin -fno-pie -fno-stack-protector -nostdinc -DTOO_NEW_SYMBOL="$too_new" \
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
        echo "Unexpected success linking newer $too_new." >&2; exit 1
    fi
    grep -q "undefined reference to .*$too_new" "logs/full-$route-negative.txt"
    pthread_command=(ld -o "pthread-default-full-$route" --dynamic-linker /lib64/ld-linux-x86-64.so.2
        "${inputs[@]}" "pthread-default-2.18-full-$route.o" -L"$generated" -lpthread "${libs[@]}")
    if [ "$pthread_defaults_expectation" = accept ]; then
        "${pthread_command[@]}"
        "$root/lib64/ld-linux-x86-64.so.2" --library-path "$root/lib64:$root/usr/lib64" \
            "./pthread-default-full-$route" > "logs/full-$route-pthread-default-run.txt"
        grep -qx 'glibc 2.18 pthread defaults work!' "logs/full-$route-pthread-default-run.txt"
        readelf --wide --dyn-syms --version-info "pthread-default-full-$route" \
            > "logs/full-$route-pthread-default-executable.txt"
        # check-full.py verifies each import against the measured default version
        # (GLIBC_2.18 until 2.34 moved both into libc under a new GLIBC_2.34 default).
        grep -q 'pthread_getattr_default_np@GLIBC_' "logs/full-$route-pthread-default-executable.txt"
        grep -q 'pthread_setattr_default_np@GLIBC_' "logs/full-$route-pthread-default-executable.txt"
    else
        if "${pthread_command[@]}" > "logs/full-$route-pthread-default-negative.txt" 2>&1; then
            echo 'Unexpected acceptance of glibc 2.18 pthread default APIs' >&2; exit 1
        fi
        grep -q 'undefined reference to .*pthread_getattr_default_np' "logs/full-$route-pthread-default-negative.txt"
        grep -q 'undefined reference to .*pthread_setattr_default_np' "logs/full-$route-pthread-default-negative.txt"
    fi
    # sigvec stays exported, but 2.21 made it compatibility-only: follow the measured default.
    sigvec_expectation=$(python3 -c 'import csv,sys
rows=[r for r in csv.DictReader(open(sys.argv[1]),delimiter="\t") if r["symbol"]=="sigvec" and r["version"]=="GLIBC_2.2.5"]
assert len(rows)==1, "missing sigvec evidence"
print("accept" if rows[0]["default"]=="yes" else "reject")' "$work/elf-metadata/$route-c.tsv")
    sigvec_command=(ld -o "sigvec-default-full-$route" --dynamic-linker /lib64/ld-linux-x86-64.so.2
        "${inputs[@]}" "sigvec-default-2.21-full-$route.o" "${libs[@]}")
    if [ "$sigvec_expectation" = accept ]; then
        "${sigvec_command[@]}"
        "$root/lib64/ld-linux-x86-64.so.2" --library-path "$root/lib64:$root/usr/lib64"             "./sigvec-default-full-$route" > "logs/full-$route-sigvec-default-run.txt"
        grep -qx 'Default sigvec interface works!' "logs/full-$route-sigvec-default-run.txt"
        readelf --wide --dyn-syms --version-info "sigvec-default-full-$route"             > "logs/full-$route-sigvec-default-executable.txt"
        grep -q 'sigvec@GLIBC_2.2.5' "logs/full-$route-sigvec-default-executable.txt"
    else
        if "${sigvec_command[@]}" > "logs/full-$route-sigvec-default-negative.txt" 2>&1; then
            echo "$release unexpectedly linked a new reference to compatibility-only sigvec" >&2; exit 1
        fi
        grep -q 'undefined reference to .sigvec' "logs/full-$route-sigvec-default-negative.txt"
    fi
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
            echo "$release unexpectedly accepted fmemopen@GLIBC_2.22" >&2; exit 1
        fi
        grep -q 'undefined reference to .*fmemopen@GLIBC_2.22' "logs/full-$route-fmemopen-negative.txt"
    fi
done
python3 "$example_dir/check-full.py" "$work" "$repo" --application-only
bash "$example_dir/behavior-and-check.sh" "$work"

#!/bin/bash
set -euo pipefail
example_dir=$(cd "$(dirname "$0")" && pwd)
source "$example_dir/experiment.sh"
repo=$(cd "$example_dir/../.." && pwd)
cd "${1:?Usage: prepare-and-check.sh EXISTING_EXPERIMENT_DIRECTORY}"
work=$PWD
export CARGO_TARGET_DIR="$work/rust-target"
unset LD_PRELOAD LD_AUDIT LD_LIBRARY_PATH
mkdir -p logs compiler-runtime run-runtime
# Programs link against a stub with GCC 4.8.5's libgcc_s interface (compiler-runtime,
# the provider's), so they need nothing newer, and run with the builder's real
# libgcc_s (run-runtime), never the host's.
if [ -e run-runtime/libgcc_s.so.1 ] || [ -L run-runtime/libgcc_s.so.1 ]; then
    cmp prebuilt/usr/lib64/libgcc_s.so.1 run-runtime/libgcc_s.so.1
else
    cp -L prebuilt/usr/lib64/libgcc_s.so.1 run-runtime/libgcc_s.so.1
fi
stub="$work/libgcc-stub"
rm -rf "$stub"
cargo run --manifest-path "$repo/Cargo.toml" --locked --offline --bin libc-abi -- \
    libgcc stub --output "$stub" | tee logs/libgcc-stub-generation.txt
as --64 "$stub/libgcc_s.S" -o "$stub/libgcc_s.o"
ld -shared --build-id=none -soname libgcc_s.so.1 --version-script="$stub/libgcc_s.map" \
    "$stub/libgcc_s.o" -o "$stub/libgcc_s.so.1"
if [ -e compiler-runtime/libgcc_s.so.1 ] || [ -L compiler-runtime/libgcc_s.so.1 ]; then
    cmp "$stub/libgcc_s.so.1" compiler-runtime/libgcc_s.so.1
else
    cp "$stub/libgcc_s.so.1" compiler-runtime/libgcc_s.so.1
fi
if [ -L compiler-runtime/libgcc_s.so ]; then
    test "$(readlink compiler-runtime/libgcc_s.so)" = libgcc_s.so.1
else
    test ! -e compiler-runtime/libgcc_s.so
    ln -s libgcc_s.so.1 compiler-runtime/libgcc_s.so
fi
readelf --wide --dyn-syms --version-info --dynamic compiler-runtime/libgcc_s.so.1 > logs/libgcc-stub-exports.txt
python3 "$example_dir/check-libgcc.py" stub "$work" "$repo" | tee logs/libgcc-stub-check.txt
libraries=(c m ld resolv pthread dl rt util)
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
done
python3 "$example_dir/check-full.py" "$work" "$repo" --abi-only
python3 "$example_dir/check-link-matrix.py" "$work"

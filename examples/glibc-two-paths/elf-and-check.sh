#!/bin/bash
# Collect all eight real DSOs and compare native parsing with GNU readelf.
set -euo pipefail
example_dir=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$example_dir/../.." && pwd)
work=${1:?Usage: elf-and-check.sh EXISTING_EXPERIMENT_DIRECTORY}
cd "$work"
work=$PWD
export CARGO_TARGET_DIR="$work/rust-target"
cargo build --manifest-path "$repo/Cargo.toml" --locked --offline
mkdir -p elf-metadata
readelf --version > elf-metadata/readelf-version.txt
libraries=(c m ld resolv pthread dl rt util)
files=(libc.so.6 libm.so.6 ld-linux-x86-64.so.2 libresolv.so.2 libpthread.so.0 libdl.so.2 librt.so.1 libutil.so.1)
"$CARGO_TARGET_DIR/debug/libc-abi" glibc snapshot --data "$repo/data/glibc" --release 2.17 \
    > elf-metadata/pinned-2.17.tsv
for route in prebuilt source; do
    for index in "${!libraries[@]}"; do
        library=${libraries[$index]}
        file="$work/$route/lib64/${files[$index]}"
        "$CARGO_TARGET_DIR/debug/libc-abi" glibc elf --input "$file" \
            > "elf-metadata/$route-$library.tsv"
        LC_ALL=C readelf --dyn-syms --wide --version-info --dynamic --section-headers "$file" \
            > "elf-metadata/$route-$library.readelf.txt"
    done
done
python3 "$example_dir/check-elf.py" "$work" "$repo"

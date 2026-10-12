#!/bin/bash
# Prepare this release's source-mode provider (`libc-abi provider prepare`, from
# the glibc-provider-sources crate and the runner's GCC) and require it to match
# this container build: the recorded library table equals this build's
# libraries', the stubs and the libgcc_s stub are byte-identical, and so are the
# headers. Every object was already verified against the recorded interfaces.
set -euo pipefail
example_dir=$(cd "$(dirname "$0")" && pwd)
source "$example_dir/experiment.sh"
repo=$(cd "$example_dir/../.." && pwd)
work=$(realpath "${1:?Usage: source-mode-and-check.sh WORK}")
export CARGO_TARGET_DIR="$work/rust-target"
cache="$work/provider-cache"
libc_abi() {
    cargo run --manifest-path "$repo/Cargo.toml" --locked --offline --quiet --bin libc-abi -- "$@"
}
descriptor=$(libc_abi provider prepare --glibc "$release" --cache "$cache" 2> "$work/logs/source-mode-prepare.txt")
cat "$work/logs/source-mode-prepare.txt"
dir=$(dirname "$descriptor")
cp "$dir/verification.tsv" "$work/logs/source-mode-verification.tsv"
libc_abi glibc libraries --release "$release" --input "$work/source/lib64" > "$work/logs/source-mode-libraries.tsv"
cmp "$dir/libraries.tsv" "$work/logs/source-mode-libraries.tsv"
for so in libc.so.6 libm.so.6 ld-linux-x86-64.so.2 libresolv.so.2 libpthread.so.0 libdl.so.2 librt.so.1 libutil.so.1; do
    cmp "$dir/stubs/$so" "$work/full-libraries-source/$so"
done
cmp "$dir/compiler-runtime/libgcc_s.so.1" "$work/compiler-runtime/libgcc_s.so.1"
diff -r "$dir/sysroot/usr/include" "$work/source/usr/include"
libc_abi provider verify --glibc "$release" --cache "$cache"
echo "glibc $release: the source-mode provider matches this container build."

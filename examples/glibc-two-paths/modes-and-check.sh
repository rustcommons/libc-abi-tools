#!/bin/bash
# Compare this release's container-mode provider (prepared with --keep-work into
# WORK) with the work tree it came from and with the source-mode provider
# (source-mode-and-check.sh), then export the container-mode provider to OUT (a
# tar archive) for the application jobs. Both providers are in $ABI_PROVIDER_CACHE.
set -euo pipefail
example_dir=$(cd "$(dirname "$0")" && pwd)
source "$example_dir/experiment.sh"
repo=$(cd "$example_dir/../.." && pwd)
work=$(realpath "${1:?Usage: modes-and-check.sh WORK OUT}")
out=${2:?Usage: modes-and-check.sh WORK OUT}
cache=${ABI_PROVIDER_CACHE:?Set ABI_PROVIDER_CACHE to the provider cache}
export CARGO_TARGET_DIR="$work/rust-target"
libc_abi() {
    cargo run --manifest-path "$repo/Cargo.toml" --locked --offline --quiet --bin libc-abi -- "$@"
}
container=$(dirname "$(libc_abi provider path --glibc "$release" --mode container --cache "$cache")")
source_mode=$(dirname "$(libc_abi provider path --glibc "$release" --mode source --cache "$cache")")
built="$work/source"
objects=(crt1.o crti.o crtn.o libc_nonshared.a)
if [ -e "$built/usr/lib64/libpthread_nonshared.a" ]; then objects+=(libpthread_nonshared.a); fi

# Container mode is this build: same objects, headers and runtime libraries.
for object in "${objects[@]}"; do
    cmp "$container/sysroot/usr/lib64/$object" "$built/usr/lib64/$object"
done
diff -r "$container/sysroot/usr/include" "$built/usr/include"
diff -r "$container/sysroot/lib64" "$built/lib64"
cmp "$container/libraries.tsv" "$work/logs/source-mode-libraries.tsv"
cmp "$container/compiler-runtime/libgcc_s.so.1" "$work/compiler-runtime/libgcc_s.so.1"

# Every mode generates the same stubs, compiler runtime stub and headers.
for so in libc.so.6 libm.so.6 ld-linux-x86-64.so.2 libresolv.so.2 libpthread.so.0 libdl.so.2 librt.so.1 libutil.so.1; do
    cmp "$container/stubs/$so" "$source_mode/stubs/$so"
    cmp "$container/stubs/$so" "$work/full-libraries-source/$so"
done
cmp "$container/libraries.tsv" "$source_mode/libraries.tsv"
cmp "$container/compiler-runtime/libgcc_s.so.1" "$source_mode/compiler-runtime/libgcc_s.so.1"
diff -r "$container/sysroot/usr/include" "$source_mode/sysroot/usr/include"
# Source-mode objects were verified when prepared: every object's interface
# (symbols, versions, sections, relocations) against the recorded container
# build, and its section contents (identical-bytes) where they agree. As files
# they differ from this build's: another compiler and assembler add sections.
verdicts=$(tail -n +2 "$source_mode/verification.tsv" | cut -f2 | sort | uniq -c |
    awk '{printf "%s%s: %s", sep, $2, $1; sep=", "}')
if grep -q $'\tdiffers' "$source_mode/verification.tsv"; then
    echo 'source mode recorded a differing object' >&2; exit 1
fi
libc_abi provider verify --glibc "$release" --mode container --cache "$cache"
libc_abi provider verify --glibc "$release" --mode source --cache "$cache"

export_dir="$work/container-provider-export"
rm -rf "$export_dir"
mkdir "$export_dir"
libc_abi provider export --glibc "$release" --mode container --cache "$cache" --to "$export_dir"
tar -C "$export_dir" -cf "$out" .
summary="glibc $release: container mode is this build; source mode has identical stubs, libgcc_s stub, library table and headers, and its objects verified against this build ($verdicts)."
echo "$summary" | tee "$work/logs/modes-summary.txt"

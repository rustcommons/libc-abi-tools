#!/usr/bin/env bash
# Online, one-time data refresh. Downloads the x86_64 syscall table from each
# mainline Linux release tag into a raw directory. The generator in
# scripts/build-syscall-data.sh then derives data/linux/syscalls-x86_64.tsv.
# Nothing here runs during normal (offline) builds.
set -euo pipefail
out=${1:?usage: fetch-syscall-tables.sh <raw-output-dir>}
mkdir -p "$out"
base=https://raw.githubusercontent.com/torvalds/linux
fetch() { # $1 = tag
  for path in arch/x86/entry/syscalls/syscall_64.tbl arch/x86/syscalls/syscall_64.tbl; do
    if curl -fsSL "$base/$1/$path" -o "$out/$1.tbl" 2>/dev/null; then return 0; fi
  done
  rm -f "$out/$1.tbl"; return 1
}
for major in 3 4 5 6 7; do
  misses=0
  start=0; [ $major = 3 ] && start=3   # syscall_64.tbl first exists in v3.3
  for minor in $(seq $start 25); do
    tag="v$major.$minor"
    if fetch "$tag"; then echo "fetched $tag"; misses=0
    else misses=$((misses+1)); [ $misses -ge 2 ] && break; fi
  done
done

#!/usr/bin/env bash
# Online, one-time data refresh. Downloads, for each mainline Linux release
# tag, the x86_64 syscall table (RAW/TAG.tbl) and the generic mman header that
# defines madvise advice values (RAW/TAG.mman.h). The build-*-data.sh scripts
# derive the committed data files. Nothing here runs during offline builds.
set -euo pipefail
out=${1:?usage: fetch-syscall-tables.sh <raw-output-dir>}
mkdir -p "$out"
base=https://raw.githubusercontent.com/torvalds/linux
get() { # $1 = tag, $2 = output file, $3... = candidate paths, newest layout first
  local tag=$1 dest=$2; shift 2
  for path in "$@"; do
    if curl -fsSL "$base/$tag/$path" -o "$dest" 2>/dev/null; then return 0; fi
  done
  rm -f "$dest"; return 1
}
for major in 3 4 5 6 7; do
  misses=0
  start=0; [ $major = 3 ] && start=3   # syscall_64.tbl first exists in v3.3
  for minor in $(seq $start 25); do
    tag="v$major.$minor"
    if get "$tag" "$out/$tag.tbl" \
         arch/x86/entry/syscalls/syscall_64.tbl arch/x86/syscalls/syscall_64.tbl; then
      get "$tag" "$out/$tag.mman.h" \
        include/uapi/asm-generic/mman-common.h include/asm-generic/mman-common.h \
        || echo "warning: no mman-common.h for $tag" >&2
      echo "fetched $tag"; misses=0
    else misses=$((misses+1)); [ $misses -ge 2 ] && break; fi
  done
done

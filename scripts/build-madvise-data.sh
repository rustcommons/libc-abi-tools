#!/usr/bin/env bash
# Derives the first mainline release defining each madvise advice value from
# the generic mman headers fetched by fetch-syscall-tables.sh. x86_64 uses the
# generic values. Kernels reject an advice value they do not know with EINVAL.
set -euo pipefail
raw=${1:?usage: build-madvise-data.sh <raw-dir> <out.tsv>}
out=${2:?}
tags=$(cd "$raw" && printf '%s\n' *.mman.h | sed 's/\.mman\.h$//' | sort -V)
first=$(echo "$tags" | head -n1); last=$(echo "$tags" | tail -n1)
{
  echo "# madvise advice values and the first mainline release defining them (x86_64)."
  echo "# Source: https://github.com/torvalds/linux include/{uapi/,}asm-generic/mman-common.h"
  echo "# Tags scanned: $first .. $last ($(echo "$tags" | wc -l) releases). Generated $(date -u +%F)."
  echo "# Rows present in $first are reported as $first (the scan floor), not their true origin."
  echo "# Columns: value<TAB>name<TAB>first_release"
  for t in $tags; do
    awk -v t="${t#v}" '$1=="#define" && $2 ~ /^MADV_/ && $3 ~ /^[0-9]+$/ {print $3"\t"$2"\t"t}' "$raw/$t.mman.h"
  done | awk -F'\t' '!seen[$2]++' | sort -n
} > "$out"

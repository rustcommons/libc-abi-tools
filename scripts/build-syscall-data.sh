#!/usr/bin/env bash
# Derives the per-syscall "first mainline release" table from raw tables
# fetched by fetch-syscall-tables.sh. Only the 64-bit ABI rows (abi "common"
# or "64") are kept; x32 rows are excluded.
set -euo pipefail
raw=${1:?usage: build-syscall-data.sh <raw-dir> <out.tsv>}
out=${2:?}
tags=$(ls "$raw" | sed 's/\.tbl$//' | sort -V)
first=$(echo "$tags" | head -n1); last=$(echo "$tags" | tail -n1)
{
  echo "# x86_64 Linux syscalls and the first mainline release whose table lists them."
  echo "# Source: https://github.com/torvalds/linux arch/x86/{entry/,}syscalls/syscall_64.tbl"
  echo "# Tags scanned: $first .. $last ($(echo "$tags" | wc -l) releases). Generated $(date -u +%F)."
  echo "# Rows present in $first are reported as $first (the scan floor), not their true origin."
  echo "# Columns: nr<TAB>name<TAB>first_release"
  for t in $tags; do
    awk -v t="${t#v}" '!/^#/ && NF>=3 && ($2=="common"||$2=="64") {print $1"\t"$3"\t"t}' "$raw/$t.tbl"
  done | awk -F'\t' '!seen[$2]++' | sort -n
} > "$out"

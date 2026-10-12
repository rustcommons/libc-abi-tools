#!/bin/bash
# Usage: scripts/record-libgcc-s.sh [--check]
#
# Records the interface of GCC 4.8.5's libgcc_s.so.1 (CentOS 7) in
# src/libgcc_s-gcc-4.8.5.tsv: each exported symbol's name, version, whether that
# version is the default, type, size and alignment. Nothing else of the library
# is recorded. The library comes from the pinned CentOS 7 builder image of the
# glibc 2.17-2.25 providers and must have the pinned SHA-256. With --check, the
# table is regenerated and compared instead of written.
set -euo pipefail
repo=$(cd "$(dirname "$0")/.." && pwd)
image=docker.io/library/centos@sha256:dead07b4d8ed7e29e98de0f4504d87e8880d4347859d839686a31da35a3b532f
package=libgcc-4.8.5-44.el7.x86_64
sha256=8f148d8b7ed050f93064fafca6df434706844cf8346d5feed8ecce8c67f39c68
table="$repo/src/libgcc_s-gcc-4.8.5.tsv"
runtime=$(command -v podman || command -v docker)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
"$runtime" pull -q "$image" > /dev/null
"$runtime" run --rm -v "$work:/out" "$image" bash -c \
    'test "$(rpm -qf /usr/lib64/libgcc_s.so.1)" = '"$package"' && cp -L /usr/lib64/libgcc_s.so.1 /out/'
echo "$sha256  $work/libgcc_s.so.1" | sha256sum -c --quiet
cargo run --quiet --manifest-path "$repo/Cargo.toml" --locked --offline --bin libc-abi -- \
    glibc elf --input "$work/libgcc_s.so.1" > "$work/elf.tsv"
{
    echo "# Interface of libgcc_s.so.1 from GCC 4.8.5: CentOS 7 package $package,"
    echo "# SHA-256 $sha256, in $image."
    echo "# Recorded by scripts/record-libgcc-s.sh. Names, versions, kinds, sizes and"
    echo "# alignments only; alignment is that of the definition, for objects."
    printf 'symbol\tversion\tdefault\ttype\tsize\talignment\n'
    # elf.tsv: soname symbol version default type binding visibility section
    # section_alignment value size same_range
    awk -F'\t' 'NR > 1 {
        if ($6 != "GLOBAL" || $7 != "DEFAULT" || ($5 != "FUNC" && $5 != "OBJECT")) {
            print "unexpected definition: " $0 > "/dev/stderr"; exit 1
        }
        alignment = "-"
        if ($5 == "OBJECT") {
            value = strtonum($10); alignment = $9 + 0
            while (alignment > 1 && value % alignment != 0) alignment /= 2
        }
        printf "%s\t%s\t%s\t%s\t%s\t%s\n", $2, $3, $4, $5, $11, alignment
    }' "$work/elf.tsv" | LC_ALL=C sort
} > "$work/table.tsv"
if [ "${1:-}" = --check ]; then
    diff -u "$table" "$work/table.tsv"
    echo "$table matches $package"
else
    cp "$work/table.tsv" "$table"
    echo "wrote $table"
fi

#!/bin/bash
# Prepares every release's source-mode provider with one GCC, and exercises the
# cache: verify, path, reuse, export and import, tampering, remove, and the
# refusals (prebuilt mode, Clang).
# Usage: check-provider-cache.sh LIBC_ABI GCC
set -euo pipefail
tool=$(realpath "${1:?Usage: check-provider-cache.sh LIBC_ABI GCC}")
cc=${2:?Usage: check-provider-cache.sh LIBC_ABI GCC}
scratch=$(mktemp -d)
trap 'chmod -R u+w "$scratch"; rm -rf "$scratch"' EXIT
cache="$scratch/cache"
releases=$(seq -f '2.%g' 17 44)

"$tool" provider prepare --all --cc "$cc" --cache "$cache" > "$scratch/descriptors.txt" 2> "$scratch/prepare.txt"
cat "$scratch/prepare.txt"
test "$(wc -l < "$scratch/descriptors.txt")" -eq 28
test "$(grep -c '^  verified every object against the container build' "$scratch/prepare.txt")" -eq 28
for release in $releases; do
    descriptor=$(grep -F "/glibc-$release/source-" "$scratch/descriptors.txt")
    "$tool" provider verify --glibc "$release" --cache "$cache"
    test "$("$tool" provider path --glibc "$release" --cc "$cc" --cache "$cache")" = "$descriptor"
    dir=$(dirname "$descriptor")
    grep -qx "glibc	$release" "$descriptor"
    test -f "$dir/sysroot/usr/lib64/crt1.o"
    test "$(readlink "$dir/stubs/libc.so")" = libc.so.6
    test "$(readlink "$dir/compiler-runtime/libgcc_s.so")" = libgcc_s.so.1
    if [ "$(printf '%s\n2.28\n' "$release" | sort -V | head -1)" = 2.28 ]; then
        test ! -e "$dir/sysroot/usr/lib64/libpthread_nonshared.a"
    else
        test -f "$dir/sysroot/usr/lib64/libpthread_nonshared.a"
    fi
done
test "$("$tool" provider list --cache "$cache" | wc -l)" -eq 29

# The same inputs reuse the provider; --force prepares it again.
"$tool" provider prepare --glibc 2.28 --cc "$cc" --cache "$cache" 2>&1 >/dev/null | grep -F 'already prepared'
"$tool" provider prepare --glibc 2.28 --cc "$cc" --cache "$cache" --force 2>&1 >/dev/null | grep -F 'source provider prepared'

# Export to another machine's cache and import there.
"$tool" provider export --glibc 2.28 --to "$scratch/export" --cache "$cache"
test ! -e "$scratch"/export/glibc-2.28/source-*/provider.txt
"$tool" provider import "$scratch/export" --cc "$cc" --cache "$scratch/other"
"$tool" provider verify --glibc 2.28 --cache "$scratch/other"
imported=$("$tool" provider path --glibc 2.28 --cc "$cc" --cache "$scratch/other")
case "$imported" in "$scratch/other/"*) ;; *) echo "import kept foreign paths: $imported" >&2; exit 1 ;; esac

# A changed file is found.
header=$(ls -d "$scratch"/other/x86_64-unknown-linux-gnu/glibc-2.28/source-*)/sysroot/usr/include/stdio.h
chmod u+w "$header"
echo '/* changed */' >> "$header"
if "$tool" provider verify --glibc 2.28 --cache "$scratch/other" 2> "$scratch/tampered.txt"; then
    echo 'verify accepted a changed header' >&2; exit 1
fi
grep -F 'files differ from manifest.json' "$scratch/tampered.txt"

# Remove, and the refusals.
"$tool" provider remove --glibc 2.28 --cache "$scratch/other"
test "$("$tool" provider list --cache "$scratch/other" | wc -l)" -eq 1
if "$tool" provider prepare --glibc 2.28 --mode prebuilt --cache "$cache" 2> "$scratch/prebuilt.txt"; then
    echo 'prebuilt mode was offered' >&2; exit 1
fi
grep -F 'legal review' "$scratch/prebuilt.txt"
if command -v clang > /dev/null; then
    if "$tool" provider prepare --glibc 2.28 --cc clang --cache "$scratch/clang" 2> "$scratch/clang.txt"; then
        echo 'Clang was accepted' >&2; exit 1
    fi
    grep -F 'is Clang' "$scratch/clang.txt"
fi
if "$tool" provider prepare --glibc 2.16 --cache "$cache" 2> "$scratch/old.txt"; then
    echo 'glibc 2.16 was accepted' >&2; exit 1
fi
echo "Source-mode providers with $("$cc" -dumpfullversion): all 28 releases prepared and verified; cache operations pass."

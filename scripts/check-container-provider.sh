#!/bin/bash
# Prepares one release's provider in container mode and in source mode, verifies
# both, and requires what must agree to agree: the library table, the stubs, the
# libgcc_s stub and the headers byte for byte, and the startup objects and
# nonshared archives' members equivalent (source mode verifies its objects
# against the recorded interfaces of the container build).
# Usage: check-container-provider.sh LIBC_ABI RELEASE
set -euo pipefail
tool=$(realpath "${1:?Usage: check-container-provider.sh LIBC_ABI RELEASE}")
release=${2:?Usage: check-container-provider.sh LIBC_ABI RELEASE}
scratch=$(mktemp -d)
trap 'chmod -R u+w "$scratch"; rm -rf "$scratch"' EXIT
cache="$scratch/cache"

container=$("$tool" provider prepare --glibc "$release" --mode container --cache "$cache")
source=$("$tool" provider prepare --glibc "$release" --cache "$cache")
"$tool" provider verify --glibc "$release" --cache "$cache"
c=$(dirname "$container")
s=$(dirname "$source")
cmp "$c/libraries.tsv" "$s/libraries.tsv"
for so in libc.so.6 libm.so.6 ld-linux-x86-64.so.2 libresolv.so.2 libpthread.so.0 libdl.so.2 librt.so.1 libutil.so.1; do
    cmp "$c/stubs/$so" "$s/stubs/$so"
done
cmp "$c/compiler-runtime/libgcc_s.so.1" "$s/compiler-runtime/libgcc_s.so.1"
diff -r "$c/sysroot/usr/include" "$s/sysroot/usr/include"
for object in crt1.o crti.o crtn.o; do
    test -f "$c/sysroot/usr/lib64/$object"
    test -f "$s/sysroot/usr/lib64/$object"
done
# Container mode keeps the real loader and libraries, for running tests.
# (A link stub is a few kilobytes; every real loader is larger than 100 KB.)
test "$(stat -L -c %s "$c/sysroot/lib64/ld-linux-x86-64.so.2")" -gt 100000
test "$(stat -L -c %s "$s/sysroot/lib64/ld-linux-x86-64.so.2")" -lt 100000
test "$("$tool" provider path --glibc "$release" --cache "$cache")" = "$source"
test "$("$tool" provider path --glibc "$release" --mode container --cache "$cache")" = "$container"
echo "glibc $release: container and source providers agree."

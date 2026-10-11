#!/bin/bash
# Fetch an official glibc release archive into the current directory and check
# its pinned SHA-256. The hash, not the mirror, authenticates the bytes.
# ABI_GLIBC_SOURCE_DIR may name a local directory of previously fetched archives.
set -euo pipefail
release=${1:?Usage: fetch-glibc.sh RELEASE SHA256}
sha256=${2:?Usage: fetch-glibc.sh RELEASE SHA256}
archive="glibc-$release.tar.xz"
if [ -n "${ABI_GLIBC_SOURCE_DIR:-}" ] && [ -f "$ABI_GLIBC_SOURCE_DIR/$archive" ]; then
    cp "$ABI_GLIBC_SOURCE_DIR/$archive" "$archive"
else
    for base in https://ftp.gnu.org/gnu/glibc https://mirrors.kernel.org/gnu/glibc; do
        if curl -fL --retry 2 --connect-timeout 30 "$base/$archive" -o "$archive"; then
            break
        fi
        rm -f "$archive"
    done
fi
echo "$sha256  $archive" | sha256sum -c -

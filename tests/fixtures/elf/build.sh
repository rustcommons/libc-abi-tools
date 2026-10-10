#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")"
object=$(mktemp "${TMPDIR:-/tmp}/libc-abi-symbols.XXXXXX.o")
trap 'rm -f "$object"' EXIT
as --64 symbols.S -o "$object"
ld -shared --build-id=none -soname libfixture.so.1 --version-script=symbols.map \
    "$object" -o symbols.so
sha256sum symbols.S symbols.map symbols.so > SHA256SUMS
as --version | head -1 > tools.txt
ld --version | head -1 >> tools.txt

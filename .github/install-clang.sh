#!/bin/bash
# CI Ubuntu 24.04 compiler selection; never silently substitute another major.
set -euo pipefail
major=${1:?Supply Clang major 21 or 23}
case "$major" in
    21) suite=llvm-toolchain-noble-21 ;;
    23) suite=llvm-toolchain-noble-23 ;;
    *) echo 'Supported CI Clang majors: 21, 23' >&2; exit 1 ;;
esac
sudo apt-get update
sudo apt-get install -y curl gnupg gcc binutils
key_dir=$(mktemp -d)
curl -fsSL https://apt.llvm.org/llvm-snapshot.gpg.key | gpg --dearmor > "$key_dir/llvm-keyring.gpg"
sudo install -m 644 "$key_dir/llvm-keyring.gpg" /usr/share/keyrings/llvm-provider.gpg
echo "deb [signed-by=/usr/share/keyrings/llvm-provider.gpg] https://apt.llvm.org/noble/ $suite main" | sudo tee /etc/apt/sources.list.d/llvm-provider.list
sudo apt-get update
sudo apt-get install -y "clang-$major"
"clang-$major" --version | tee "${RUNNER_TEMP:-/tmp}/clang-version.txt"
grep -E "clang version $major\." "${RUNNER_TEMP:-/tmp}/clang-version.txt"
{ rustc --version; "clang-$major" --version; dpkg-query -W "clang-$major" binutils gcc; } > "${RUNNER_TEMP:-/tmp}/modern-toolchain.txt"

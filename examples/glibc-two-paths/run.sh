#!/bin/bash
# Requires x86_64 Linux, Podman, GCC, GNU binutils, curl, and sha256sum.
set -euo pipefail
example_dir=$(cd "$(dirname "$0")" && pwd)
work=${1:?Usage: run.sh NEW_ABSOLUTE_WORK_DIRECTORY}
case "$work" in /*) ;; *) echo 'Use an absolute work directory.' >&2; exit 1 ;; esac
if [ -e "$work" ]; then echo 'Work directory must not exist.' >&2; exit 1; fi
mkdir -p "$work"
cp "$example_dir/hello.c" "$example_dir/build-inputs.sh" "$example_dir/link-and-check.sh" "$work/"
cd "$work"
curl -fL --retry 2 https://ftp.gnu.org/gnu/glibc/glibc-2.17.tar.xz -o glibc-2.17.tar.xz
echo '6914e337401e0e0ade23694e1b2c52a5f09e4eda3270c67e7c3ba93a89b5b23e  glibc-2.17.tar.xz' | sha256sum -c -
image=docker.io/library/centos@sha256:dead07b4d8ed7e29e98de0f4504d87e8880d4347859d839686a31da35a3b532f
podman pull "$image"
podman run --rm -v "$work:/work" "$image" bash /work/build-inputs.sh
bash ./link-and-check.sh "$work"

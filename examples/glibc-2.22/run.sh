#!/bin/bash
set -euo pipefail
example_dir=$(cd "$(dirname "$0")" && pwd)
work=${1:?Usage: run.sh NEW_ABSOLUTE_WORK_DIRECTORY}
case "$work" in /*) ;; *) echo 'Use an absolute work directory.' >&2; exit 1 ;; esac
if [ -e "$work" ]; then echo 'Work directory must not exist.' >&2; exit 1; fi
mkdir -p "$work"
cp "$example_dir/build-inputs.sh" "$work/"
cd "$work"
curl -fL --retry 2 https://ftp.gnu.org/gnu/glibc/glibc-2.22.tar.xz -o glibc-2.22.tar.xz
echo 'eb731406903befef1d8f878a46be75ef862b9056ab0cde1626d08a7a05328948  glibc-2.22.tar.xz' | sha256sum -c -
image=docker.io/library/centos@sha256:dead07b4d8ed7e29e98de0f4504d87e8880d4347859d839686a31da35a3b532f
podman pull "$image"
podman run --rm -v "$work:/work" "$image" bash /work/build-inputs.sh

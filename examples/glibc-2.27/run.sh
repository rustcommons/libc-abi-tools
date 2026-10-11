#!/bin/bash
set -euo pipefail
example_dir=$(cd "$(dirname "$0")" && pwd)
work=${1:?Usage: run.sh NEW_ABSOLUTE_WORK_DIRECTORY}
case "$work" in /*) ;; *) echo 'Use an absolute work directory.' >&2; exit 1 ;; esac
if [ -e "$work" ]; then echo 'Work directory must not exist.' >&2; exit 1; fi
mkdir -p "$work"
cp "$example_dir/build-inputs.sh" "$work/"
cd "$work"
bash "$example_dir/../glibc-two-paths/fetch-glibc.sh" 2.27 5172de54318ec0b7f2735e5a91d908afe1c9ca291fec16b5374d9faadfc1fc72
image=docker.io/library/debian@sha256:16ee3e11da473f6565ef94a715f1a38b3079e0664913be78869c336a71cb6085
podman pull "$image"
podman run --rm -v "$work:/work" "$image" bash /work/build-inputs.sh

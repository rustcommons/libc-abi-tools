#!/bin/bash
set -euo pipefail
example_dir=$(cd "$(dirname "$0")" && pwd)
work=${1:?Usage: run.sh NEW_ABSOLUTE_WORK_DIRECTORY}
case "$work" in /*) ;; *) echo 'Use an absolute work directory.' >&2; exit 1 ;; esac
if [ -e "$work" ]; then echo 'Work directory must not exist.' >&2; exit 1; fi
mkdir -p "$work"
cp "$example_dir/build-inputs.sh" "$work/"
cd "$work"
bash "$example_dir/../glibc-two-paths/fetch-glibc.sh" 2.42 d1775e32e4628e64ef930f435b67bb63af7599acb6be2b335b9f19f16509f17f
image=docker.io/library/debian@sha256:bc49dc1918ee1a47a93e65b5e4676e8680fb754b133197b92ca52bfe6731d5f0
podman pull "$image"
podman run --rm -v "$work:/work" "$image" bash /work/build-inputs.sh

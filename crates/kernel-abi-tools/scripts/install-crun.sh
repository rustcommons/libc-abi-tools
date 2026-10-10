#!/usr/bin/env bash
# Installs a pinned, checksum-verified static crun for self-tests and CI.
# Static crun carries a libseccomp new enough to name every syscall in the
# data, so all profiles can be verified on hosts whose own runtime cannot
# (e.g. Ubuntu 24.04: runc/crun with libseccomp 2.5.5). Not redistributed in
# the crate; on a disconnected network, carry this file across and verify the
# same checksum.
#
#   install-crun.sh DEST_FILE
#   then: KERNEL_ABI_ENGINE_ARGS="--runtime DEST_FILE"
set -euo pipefail
dest=${1:?usage: install-crun.sh DEST_FILE}
version=1.28
sha256=2aa6b7024a9c9f153895c0d11ae233d3758f54844011c3a039e3e89048d01d42
url=https://github.com/containers/crun/releases/download/$version/crun-$version-linux-amd64

tmp=$(mktemp "$(dirname "$dest")/.crun.XXXXXX")
trap 'rm -f "$tmp"' EXIT
curl -fsSL -o "$tmp" "$url"
echo "$sha256  $tmp" | sha256sum -c --quiet
chmod 0755 "$tmp"
mv "$tmp" "$dest"
"$dest" --version | head -n 1

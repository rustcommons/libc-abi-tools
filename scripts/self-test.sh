#!/usr/bin/env bash
# Proves each generated profile simulates its kernel on this host/runtime:
# every syscall the kernel lacks must return ENOSYS, and every syscall it has
# must behave exactly as it does without a profile.
#
#   self-test.sh PROBE IMAGE [KERNEL|DISTRO_ID ...]
#
# PROBE is a syscall-probe binary built for a glibc no newer than IMAGE's
# (e.g. with cargo-libc or a glibc 2.17 floor). With no selections, every
# generic kernel profile is tested.
set -euo pipefail

engine=${CONTAINER_ENGINE:-podman}
here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
gen=${KERNEL_SECCOMP:-$(command -v kernel-seccomp || echo "$here/../target/release/kernel-seccomp")}
probe=$(cd "$(dirname "${1:?usage: self-test.sh PROBE IMAGE [KERNEL|DISTRO_ID ...]}")" && pwd)/$(basename "$1")
image=${2:?usage: self-test.sh PROBE IMAGE [KERNEL|DISTRO_ID ...]}
shift 2
work=$(mktemp -d "${TMPDIR:-/tmp}/kernel-selftest.XXXXXX")
trap 'rm -rf "$work"' EXIT
cp "$probe" "$work/syscall-probe"

selections=("$@")
if [ ${#selections[@]} -eq 0 ]; then
  for f in "$here"/../profiles/kernel/linux-*.json; do
    f=${f##*/linux-}; selections+=("${f%.json}")
  done
fi

# shellcheck disable=SC2086
run() { "$engine" ${KERNEL_ABI_ENGINE_ARGS:-} run --rm -v "$work:/work:z" -w /work "$@"; }

echo "host kernel: $(run --security-opt seccomp=unconfined "$image" /bin/sh -c 'uname -r')"
run --security-opt seccomp=unconfined "$image" /work/syscall-probe dump > "$work/baseline.tsv"

failed=0
for sel in "${selections[@]}"; do
  if [[ $sel =~ ^[0-9]+\.[0-9]+$ ]]; then kind=--kernel; else kind=--distro; fi
  "$gen" profile "$kind" "$sel" --output "$work/profile.json"
  if ! run --security-opt "seccomp=$work/profile.json" "$image" \
        /work/syscall-probe check "$kind" "$sel" --baseline /work/baseline.tsv \
        | sed "s/^/[$sel] /"; then
    failed=$((failed + 1))
  fi
done
[ $failed -eq 0 ] && echo "self-test: all ${#selections[@]} profiles verified" \
  || { echo "self-test: $failed of ${#selections[@]} profiles FAILED"; exit 1; }

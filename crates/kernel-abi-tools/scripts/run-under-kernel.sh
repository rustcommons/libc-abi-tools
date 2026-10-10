#!/usr/bin/env bash
# Runs a Linux x86_64 executable so the host kernel answers like the selected
# kernel: on the host under a local seccomp filter, or in a container under the
# equivalent profile. Thin wrapper over `kernel-seccomp run`; see
# `kernel-seccomp help` for options and environment.
#
#   run-under-kernel.sh (--kernel X.Y | --distro ID) [--runner local|container]
#                       [--image IMAGE] -- PROGRAM [ARGS...]
#
# KERNEL_SECCOMP selects the kernel-seccomp binary (default: on PATH, else the
# workspace release build).
set -euo pipefail
here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
for candidate in "${KERNEL_SECCOMP:-}" "$(command -v kernel-seccomp || true)" \
                 "$here/../../../target/release/kernel-seccomp"; do
  if [ -n "$candidate" ] && [ -x "$candidate" ]; then
    exec "$candidate" run "$@"
  fi
done
echo "run-under-kernel: kernel-seccomp not found (set KERNEL_SECCOMP)" >&2
exit 2

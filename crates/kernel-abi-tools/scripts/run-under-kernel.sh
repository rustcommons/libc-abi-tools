#!/usr/bin/env bash
# Runs a Linux x86_64 executable in a container whose seccomp profile makes the
# host kernel answer like the selected kernel. Thin wrapper over
# `kernel-seccomp run`; see `kernel-seccomp help` for options and environment.
#
#   run-under-kernel.sh (--kernel X.Y | --distro ID) [--image IMAGE] -- PROGRAM [ARGS...]
#
# KERNEL_SECCOMP selects the kernel-seccomp binary (default: on PATH, else the
# workspace release build, else bin/ of a release bundle).
set -euo pipefail
here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
for candidate in "${KERNEL_SECCOMP:-}" "$(command -v kernel-seccomp || true)" \
                 "$here/../../../target/release/kernel-seccomp" "$here/../bin/kernel-seccomp"; do
  if [ -n "$candidate" ] && [ -x "$candidate" ]; then
    exec "$candidate" run "$@"
  fi
done
echo "run-under-kernel: kernel-seccomp not found (set KERNEL_SECCOMP)" >&2
exit 2

#!/usr/bin/env bash
# Runs a Linux x86_64 executable in a container whose seccomp profile makes the
# host kernel answer ENOSYS for every syscall the selected kernel lacks.
#
#   run-under-kernel.sh (--kernel X.Y | --distro ID) [--image IMAGE] [--] BINARY [ARGS...]
#
# Usable directly or as a Cargo runner (see README). The current directory and
# the binary's directory are mounted at their own absolute paths, so relative
# paths and compile-time CARGO_MANIFEST_DIR values keep working. CARGO* and
# RUST* environment variables are forwarded.
#
# Environment:
#   CONTAINER_ENGINE   podman (default) or docker
#   KERNEL_SECCOMP     path to the kernel-seccomp binary (default: on PATH,
#                      else the release build next to this script's crate)
#   KERNEL_ABI_ARGS    extra engine arguments, e.g. "--network none"
set -euo pipefail

engine=${CONTAINER_ENGINE:-podman}
here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
gen=${KERNEL_SECCOMP:-$(command -v kernel-seccomp || echo "$here/../target/release/kernel-seccomp")}

select=() image=""
while [ $# -gt 0 ]; do
  case $1 in
    --kernel|--distro) select=("$1" "$2"); shift 2 ;;
    --image) image=$2; shift 2 ;;
    --) shift; break ;;
    -*) echo "run-under-kernel: unknown option $1" >&2; exit 2 ;;
    *) break ;;
  esac
done
[ ${#select[@]} -eq 2 ] && [ $# -ge 1 ] || {
  echo "usage: run-under-kernel.sh (--kernel X.Y | --distro ID) [--image IMAGE] [--] BINARY [ARGS...]" >&2
  exit 2
}
[ -x "$gen" ] || { echo "run-under-kernel: kernel-seccomp not found at $gen (set KERNEL_SECCOMP)" >&2; exit 2; }

if [ -z "$image" ]; then
  if [ "${select[0]}" = --distro ]; then
    image=$("$gen" distros | awk -F'\t' -v id="${select[1]}" '$1==id {print $5}')
  fi
  [ -n "$image" ] || { echo "run-under-kernel: --image is required with --kernel" >&2; exit 2; }
fi

bin=$(cd "$(dirname "$1")" && pwd)/$(basename "$1"); shift
profile=$(mktemp "${TMPDIR:-/tmp}/kernel-seccomp.XXXXXX.json")
trap 'rm -f "$profile"' EXIT
"$gen" profile "${select[@]}" --output "$profile"

mounts=(-v "$PWD:$PWD:z")
case "$(dirname "$bin")/" in "$PWD"/*) ;; *) mounts+=(-v "$(dirname "$bin"):$(dirname "$bin"):ro,z") ;; esac

# shellcheck disable=SC2086
exec "$engine" ${KERNEL_ABI_ENGINE_ARGS:-} run --rm --security-opt "seccomp=$profile" "${mounts[@]}" -w "$PWD" \
  -e 'CARGO*' -e 'RUST*' ${KERNEL_ABI_ARGS:-} "$image" "$bin" "$@"

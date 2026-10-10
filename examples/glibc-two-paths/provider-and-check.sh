#!/bin/bash
set -euo pipefail
example_dir=$(cd "$(dirname "$0")" && pwd)
source "$example_dir/experiment.sh"
repo=$(cd "$example_dir/../.." && pwd)
work=$(realpath "${1:?Usage: provider-and-check.sh WORK CARGO_LIBC_REPO CLANG}")
companion=$(realpath "${2:?Supply cargo-libc checkout}")
clang=$(realpath "${3:?Supply explicit Clang executable}")
unset LD_PRELOAD LD_AUDIT LD_LIBRARY_PATH RUSTFLAGS CARGO_ENCODED_RUSTFLAGS
export CARGO_TARGET_DIR="$work/provider-tools"
mkdir -p "$work/logs"
gcc_startup=$(dirname "$(gcc -print-file-name=crtbegin.o)")
cargo build --manifest-path "$companion/Cargo.toml" --bins --locked --offline
for route in "${routes[@]}"; do
    descriptor="$work/provider-$route-v1-$$.txt"
    cargo run --manifest-path "$repo/Cargo.toml" --locked --offline --example provider_glibc -- \
        "$repo/data/glibc" "$release" "$work/$route" "$work/full-libraries-$route" \
        "$gcc_startup" "$work/compiler-runtime" "$descriptor"
    artifact="$work/cargo-$route/x86_64-unknown-linux-gnu/debug/glibc-hello"
    export CARGO_LIBC_PROVIDER_LINK_MAP="$work/logs/provider-$route-link.map"
    "$CARGO_TARGET_DIR/debug/cargo-libc" build --provider "$descriptor" --glibc "$release" \
        --cc "$clang" --ld /usr/bin/ld.bfd --artifact "$artifact" -- \
        --manifest-path "$companion/examples/hello/Cargo.toml" --target-dir "$work/cargo-$route" \
        --locked --offline 2>&1 | tee "$work/logs/provider-$route-build.txt"
    "$work/$route/lib64/ld-linux-x86-64.so.2" \
        --library-path "$work/compiler-runtime:$work/$route/lib64:$work/$route/usr/lib64" \
        "$artifact" | tee "$work/logs/provider-$route-runtime.txt"
    grep -F 'host build script ran' "$work/logs/provider-$route-runtime.txt"
    grep -F 'host procedural macro ran' "$work/logs/provider-$route-runtime.txt"
    grep -F 'target executable ran; thread result = 42' "$work/logs/provider-$route-runtime.txt"
    "$work/$route/lib64/ld-linux-x86-64.so.2" \
        --library-path "$work/compiler-runtime:$work/$route/lib64:$work/$route/usr/lib64" \
        --list "$artifact" > "$work/logs/provider-$route-runtime-libraries.txt"
    python3 "$example_dir/check-provider.py" "$work" "$route"
    # A different requested release must be rejected, never silently downgraded.
    other=2.22
    if [ "$release" = 2.22 ]; then other=2.17; fi
    if "$CARGO_TARGET_DIR/debug/cargo-libc" doctor --provider "$descriptor" --glibc "$other" \
        --cc "$clang" > "$work/logs/provider-$route-negative.txt" 2>&1; then
        echo 'Unexpected acceptance of mismatched provider version' >&2; exit 1
    fi
    # This must reach the target linker and fail on the newer symbol, not on setup.
    negative_artifact="$work/cargo-negative-$route/x86_64-unknown-linux-gnu/debug/glibc-too-new"
    export CARGO_LIBC_PROVIDER_LINK_MAP="$work/logs/provider-$route-negative-link.map"
    if "$CARGO_TARGET_DIR/debug/cargo-libc" build --provider "$descriptor" --glibc "$release" \
        --cc "$clang" --ld /usr/bin/ld.bfd --artifact "$negative_artifact" -- \
        --manifest-path "$companion/examples/too-new/Cargo.toml" \
        --target-dir "$work/cargo-negative-$route" --locked --offline \
        > "$work/logs/provider-$route-newer-symbol-negative.txt" 2>&1; then
        echo 'Cargo unexpectedly linked memfd_create' >&2; exit 1
    fi
    grep -E 'undefined reference.*memfd_create|undefined symbol: memfd_create' \
        "$work/logs/provider-$route-newer-symbol-negative.txt"
done

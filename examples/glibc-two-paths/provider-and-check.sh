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
python3 "$example_dir/check-matrix.py" "$work/elf-metadata/matrix.tsv" "$release" c memfd_create GLIBC_2.27 reject
gcc_startup=$(dirname "$(gcc -print-file-name=crtbegin.o)")
cargo build --manifest-path "$companion/Cargo.toml" --bins --locked --offline
for route in "${routes[@]}"; do
    descriptor="$work/provider-$route-v1-$$.txt"
    cargo run --manifest-path "$repo/Cargo.toml" --locked --offline --example provider_glibc -- \
        "$repo/data/glibc" "$release" "$work/$route" "$work/full-libraries-$route" \
        "$gcc_startup" "$work/compiler-runtime" "$descriptor"
    artifact="$work/cargo-$route/x86_64-unknown-linux-gnu/debug/glibc-hello"
    export CARGO_LIBC_PROVIDER_LINK_MAP="$work/logs/provider-$route-link.map"
    # Qualify the suffix API; Cargo must still report the ordinary target path.
    "$CARGO_TARGET_DIR/debug/cargo-libc" build --provider "$descriptor" --target "x86_64-unknown-linux-gnu.$release" \
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
    # Normal Cargo selections must survive the coordinator unchanged.
    workflow_dir="$work/cargo-workflow-$route"
    manifest="$companion/examples/hello/Cargo.toml"
    workflow_artifact="$workflow_dir/x86_64-unknown-linux-gnu/qualification/glibc-hello"
    for pass in first cached; do
        "$CARGO_TARGET_DIR/debug/cargo-libc" build --provider "$descriptor" \
            --target "x86_64-unknown-linux-gnu.$release" --cc "$clang" --ld /usr/bin/ld.bfd \
            --artifact "$workflow_artifact" -- --manifest-path "$manifest" \
            --target-dir "$workflow_dir" --workspace --exclude hello-macro \
            --bins --examples --features glibc-hello/qualification --no-default-features \
            --profile qualification --locked --offline \
            > "$work/logs/provider-$route-workflow-$pass.txt" 2>&1
    done
    for executable in glibc-hello qualification-member examples/qualification; do
        "$work/$route/lib64/ld-linux-x86-64.so.2" \
            --library-path "$work/compiler-runtime:$work/$route/lib64:$work/$route/usr/lib64" \
            "$workflow_dir/x86_64-unknown-linux-gnu/qualification/$executable" \
            > "$work/logs/provider-$route-workflow-$(basename "$executable")-runtime.txt"
    done
    grep -F 'target executable ran; thread result = 42' "$work/logs/provider-$route-workflow-glibc-hello-runtime.txt"
    grep -F 'workspace member ran' "$work/logs/provider-$route-workflow-qualification-member-runtime.txt"
    grep -F 'qualification example ran' "$work/logs/provider-$route-workflow-qualification-runtime.txt"
    "$CARGO_TARGET_DIR/debug/cargo-libc" build --provider "$descriptor" \
        --target "x86_64-unknown-linux-gnu.$release" --cc "$clang" --ld /usr/bin/ld.bfd \
        --artifact "$workflow_dir/x86_64-unknown-linux-gnu/release/examples/qualification" -- \
        --manifest-path "$manifest" --target-dir "$workflow_dir" \
        -p glibc-hello --example qualification --features qualification --release --locked --offline \
        > "$work/logs/provider-$route-workflow-example.txt" 2>&1
    "$work/$route/lib64/ld-linux-x86-64.so.2" \
        --library-path "$work/compiler-runtime:$work/$route/lib64:$work/$route/usr/lib64" \
        "$workflow_dir/x86_64-unknown-linux-gnu/release/examples/qualification" \
        > "$work/logs/provider-$route-workflow-release-runtime.txt"
    grep -F 'qualification example ran' "$work/logs/provider-$route-workflow-release-runtime.txt"
    # A stale executable from the workspace build cannot satisfy a package-only invocation.
    if "$CARGO_TARGET_DIR/debug/cargo-libc" build --provider "$descriptor" \
        --target "x86_64-unknown-linux-gnu.$release" --cc "$clang" --ld /usr/bin/ld.bfd \
        --artifact "$workflow_artifact" -- --manifest-path "$manifest" \
        --target-dir "$workflow_dir" -p qualification-member --bin qualification-member \
        --profile qualification --locked --offline \
        > "$work/logs/provider-$route-workflow-stale-negative.txt" 2>&1; then
        echo 'Unexpected acceptance of executable absent from selected Cargo build' >&2; exit 1
    fi
    grep -F 'was not reported as an executable by this Cargo invocation' \
        "$work/logs/provider-$route-workflow-stale-negative.txt"
    # A different requested release must be rejected, never silently downgraded.
    other=2.22
    if [ "$release" = 2.22 ]; then other=2.17; fi
    if "$CARGO_TARGET_DIR/debug/cargo-libc" doctor --provider "$descriptor" --target "x86_64-unknown-linux-gnu.$other" \
        --cc "$clang" > "$work/logs/provider-$route-negative.txt" 2>&1; then
        echo 'Unexpected acceptance of mismatched provider version' >&2; exit 1
    fi
    grep -F 'provider targets glibc' "$work/logs/provider-$route-negative.txt"
    if "$CARGO_TARGET_DIR/debug/cargo-libc" doctor --provider "$descriptor" \
        --target "x86_64-unknown-linux-gnu.$release" --glibc "$other" \
        > "$work/logs/provider-$route-conflict-negative.txt" 2>&1; then
        echo 'Unexpected acceptance of conflicting suffix and glibc' >&2; exit 1
    fi
    grep -F -- '--target version conflicts with --glibc' "$work/logs/provider-$route-conflict-negative.txt"
    # This must reach the target linker and fail on the newer symbol, not on setup.
    negative_artifact="$work/cargo-negative-$route/x86_64-unknown-linux-gnu/debug/glibc-too-new"
    export CARGO_LIBC_PROVIDER_LINK_MAP="$work/logs/provider-$route-negative-link.map"
    if "$CARGO_TARGET_DIR/debug/cargo-libc" build --provider "$descriptor" --target "x86_64-unknown-linux-gnu.$release" \
        --cc "$clang" --ld /usr/bin/ld.bfd --artifact "$negative_artifact" -- \
        --manifest-path "$companion/examples/too-new/Cargo.toml" \
        --target-dir "$work/cargo-negative-$route" --locked --offline \
        > "$work/logs/provider-$route-newer-symbol-negative.txt" 2>&1; then
        echo 'Cargo unexpectedly linked memfd_create' >&2; exit 1
    fi
    grep -E 'undefined reference.*memfd_create|undefined symbol: memfd_create' \
        "$work/logs/provider-$route-newer-symbol-negative.txt"
done

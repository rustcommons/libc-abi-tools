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
too_new=$(python3 "$example_dir/too-new-symbol.py" "$work/elf-metadata/matrix.tsv" "$release")
# The companion's too-new crate names memfd_create; bind it to this release's newer symbol.
rm -rf "$work/too-new-crate"
mkdir -p "$work/too-new-crate"
cp -r "$companion/examples/too-new/Cargo.toml" "$companion/examples/too-new/Cargo.lock" \
    "$companion/examples/too-new/src" "$work/too-new-crate/"
sed -i "s|^    fn memfd_create(|    #[link_name = \"$too_new\"]\n    fn memfd_create(|" "$work/too-new-crate/src/main.rs"
grep -qF "#[link_name = \"$too_new\"]" "$work/too-new-crate/src/main.rs"
gcc_startup=$(dirname "$(gcc -print-file-name=crtbegin.o)")
cargo fetch --manifest-path "$companion/Cargo.toml" --locked
cargo build --manifest-path "$companion/Cargo.toml" --bins --locked --offline
# Each route's provider assembled from this work tree, then the providers that
# `libc-abi provider` prepared in each mode listed in ABI_PROVIDER_MODES (in
# $ABI_PROVIDER_CACHE). Mode providers run on the work tree's GNU build.
labels=("${routes[@]}")
for mode in ${ABI_PROVIDER_MODES:-}; do labels+=("mode-$mode"); done
for route in "${labels[@]}"; do
    if [[ "$route" == mode-* ]]; then
        runtime=source
        descriptor=$(cargo run --manifest-path "$repo/Cargo.toml" --locked --offline --quiet --bin libc-abi -- \
            provider path --glibc "$release" --mode "${route#mode-}" --cache "${ABI_PROVIDER_CACHE:?}")
    else
        runtime=$route
        descriptor="$work/provider-$route-v1-$$.txt"
        cargo run --manifest-path "$repo/Cargo.toml" --locked --offline --example provider_glibc -- \
            "$repo/data/glibc" "$release" "$work/$route" "$work/full-libraries-$route" \
            "$gcc_startup" "$work/compiler-runtime" "$descriptor"
    fi
    artifact="$work/cargo-$route/x86_64-unknown-linux-gnu/debug/glibc-hello"
    export CARGO_LIBC_PROVIDER_LINK_MAP="$work/logs/provider-$route-link.map"
    # Qualify the suffix API; Cargo must still report the ordinary target path.
    "$CARGO_TARGET_DIR/debug/cargo-libc" build --provider "$descriptor" --target "x86_64-unknown-linux-gnu.$release" \
        --cc "$clang" --ld /usr/bin/ld.bfd --kernel 3.10 --artifact "$artifact" -- \
        --manifest-path "$companion/examples/hello/Cargo.toml" --target-dir "$work/cargo-$route" \
        --locked --offline 2>&1 | tee "$work/logs/provider-$route-build.txt"
    # Tier 1: the scan of the provider-built executable against this glibc and Linux 3.10.
    grep -F 'Tier 1 scan (linux-abi-scan): pass' "$work/logs/provider-$route-build.txt"
    # Tier 2: the same executable through libc-runner under a 3.10 seccomp filter.
    "$CARGO_TARGET_DIR/debug/cargo-libc" run --provider "$descriptor" --target "x86_64-unknown-linux-gnu.$release" \
        --cc "$clang" --ld /usr/bin/ld.bfd --kernel 3.10 --runner local -- \
        --manifest-path "$companion/examples/hello/Cargo.toml" --target-dir "$work/cargo-$route" \
        --bin glibc-hello --locked --offline 2>&1 | tee "$work/logs/provider-$route-run.txt"
    grep -F 'Kernel simulation: Linux 3.10 with the local seccomp filter' "$work/logs/provider-$route-run.txt"
    grep -F 'target executable ran; thread result = 42' "$work/logs/provider-$route-run.txt"
    # The local runner uses the host's glibc: a program needing a newer one must
    # be refused before the loader sees it (only testable for newer providers).
    host_glibc=$(getconf GNU_LIBC_VERSION | cut -d' ' -f2)
    host_newer=$(python3 "$example_dir/host-newer-symbol.py" "$work/elf-metadata/matrix.tsv" "$release" "$host_glibc")
    if [ -n "$host_newer" ]; then
        rm -rf "$work/host-newer-crate"
        cp -r "$work/too-new-crate" "$work/host-newer-crate"
        sed -i "s|#\[link_name = \"$too_new\"\]|#[link_name = \"$host_newer\"]|" "$work/host-newer-crate/src/main.rs"
        grep -qF "#[link_name = \"$host_newer\"]" "$work/host-newer-crate/src/main.rs"
        if "$CARGO_TARGET_DIR/debug/cargo-libc" run --provider "$descriptor" --target "x86_64-unknown-linux-gnu.$release" \
            --cc "$clang" --ld /usr/bin/ld.bfd --kernel 3.10 --runner local -- \
            --manifest-path "$work/host-newer-crate/Cargo.toml" --target-dir "$work/cargo-host-newer-$route" \
            --locked --offline > "$work/logs/provider-$route-host-glibc-negative.txt" 2>&1; then
            echo "the local runner started a program that needs $host_newer beyond host glibc $host_glibc" >&2
            exit 1
        fi
        grep -F "the host's libraries lack libc.so.6 GLIBC_" "$work/logs/provider-$route-host-glibc-negative.txt"
        grep -F "so the loader would refuse it" "$work/logs/provider-$route-host-glibc-negative.txt"
    fi
    # F03: check the same workspace with this provider; nothing may be linked.
    check_dir="$work/cargo-check-$route"
    "$CARGO_TARGET_DIR/debug/cargo-libc" check --provider "$descriptor" --target "x86_64-unknown-linux-gnu.$release" \
        --cc "$clang" --ld /usr/bin/ld.bfd -- \
        --manifest-path "$companion/examples/hello/Cargo.toml" --target-dir "$check_dir" \
        --workspace --all-targets --locked --offline 2>&1 | tee "$work/logs/provider-$route-check.txt"
    grep -F 'Nothing was linked' "$work/logs/provider-$route-check.txt"
    if [ -e "$check_dir/x86_64-unknown-linux-gnu/debug/glibc-hello" ]; then
        echo "check linked an executable for $route" >&2
        exit 1
    fi
    if "$CARGO_TARGET_DIR/debug/cargo-libc" check --provider "$descriptor" \
        --target "x86_64-unknown-linux-gnu.$release" --glibc 2.99 --cc "$clang" --ld /usr/bin/ld.bfd -- \
        --manifest-path "$companion/examples/hello/Cargo.toml" --target-dir "$check_dir" \
        --locked --offline > "$work/logs/provider-$route-check-conflict.txt" 2>&1; then
        echo "check accepted a --glibc that conflicts with the --target suffix" >&2
        exit 1
    fi
    grep -F -- '--target version conflicts with --glibc' "$work/logs/provider-$route-check-conflict.txt"
    "$work/$runtime/lib64/ld-linux-x86-64.so.2" \
        --library-path "$work/run-runtime:$work/$runtime/lib64:$work/$runtime/usr/lib64" \
        "$artifact" | tee "$work/logs/provider-$route-runtime.txt"
    grep -F 'host build script ran' "$work/logs/provider-$route-runtime.txt"
    grep -F 'host procedural macro ran' "$work/logs/provider-$route-runtime.txt"
    grep -F 'target executable ran; thread result = 42' "$work/logs/provider-$route-runtime.txt"
    "$work/$runtime/lib64/ld-linux-x86-64.so.2" \
        --library-path "$work/run-runtime:$work/$runtime/lib64:$work/$runtime/usr/lib64" \
        --list "$artifact" > "$work/logs/provider-$route-runtime-libraries.txt"
    python3 "$example_dir/check-provider.py" "$work" "$route" "$runtime" "$descriptor"
    readelf --wide --dyn-syms --version-info "$artifact" > "$work/logs/provider-$route-executable.txt"
    python3 "$example_dir/check-libgcc.py" imports "$repo" "$work/logs/provider-$route-executable.txt"
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
        "$work/$runtime/lib64/ld-linux-x86-64.so.2" \
            --library-path "$work/run-runtime:$work/$runtime/lib64:$work/$runtime/usr/lib64" \
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
    "$work/$runtime/lib64/ld-linux-x86-64.so.2" \
        --library-path "$work/run-runtime:$work/$runtime/lib64:$work/$runtime/usr/lib64" \
        "$workflow_dir/x86_64-unknown-linux-gnu/release/examples/qualification" \
        > "$work/logs/provider-$route-workflow-release-runtime.txt"
    grep -F 'qualification example ran' "$work/logs/provider-$route-workflow-release-runtime.txt"
    readelf --wide --dyn-syms --version-info "$workflow_dir/x86_64-unknown-linux-gnu/release/examples/qualification" \
        > "$work/logs/provider-$route-workflow-release-executable.txt"
    python3 "$example_dir/check-libgcc.py" imports "$repo" "$work/logs/provider-$route-workflow-release-executable.txt"
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
        --manifest-path "$work/too-new-crate/Cargo.toml" \
        --target-dir "$work/cargo-negative-$route" --locked --offline \
        > "$work/logs/provider-$route-newer-symbol-negative.txt" 2>&1; then
        echo "Cargo unexpectedly linked newer $too_new" >&2; exit 1
    fi
    grep -E "undefined reference.*$too_new|undefined symbol: $too_new" \
        "$work/logs/provider-$route-newer-symbol-negative.txt"
done

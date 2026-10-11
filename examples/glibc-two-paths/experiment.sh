# Shared release selection. Existing scripts default to both 2.17 input routes.
# Any later release needs its own pinned inputs; there is no fallback.
release=${ABI_GLIBC_RELEASE:-2.17}
if [ "$release" = 2.17 ]; then
    routes=(prebuilt source)
elif [[ "$release" =~ ^2\.[0-9]+$ ]] &&
    grep -qx "release = \"$release\"" "$(dirname "${BASH_SOURCE[0]}")/../glibc-$release/INPUTS.toml" 2>/dev/null; then
    routes=(source)
else
    echo "Experiment release must be 2.17 or have examples/glibc-$release/INPUTS.toml" >&2; exit 1
fi

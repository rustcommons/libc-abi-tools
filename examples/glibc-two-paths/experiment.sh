# Shared release selection. Existing scripts default to both 2.17 input routes.
release=${ABI_GLIBC_RELEASE:-2.17}
case "$release" in
    2.17) routes=(prebuilt source) ;;
    2.18|2.19|2.20|2.22) routes=(source) ;;
    *) echo 'Experiment releases: 2.17, 2.18, 2.19, 2.20, 2.22' >&2; exit 1 ;;
esac

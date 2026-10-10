#!/usr/bin/env bash
# Builds the release bundle for the checked-out commit:
#
#   OUT/kernel-abi-tools-VERSION-x86_64-linux.tar.gz
#   OUT/kernel-abi-tools-VERSION-x86_64-linux.tar.gz.sha256
#
# The binaries are static (x86_64-unknown-linux-musl), so they run on any
# x86_64 Linux host and inside any image. The archive is reproducible: fixed
# file order, owners, modes, and timestamps from the commit. Existing outputs
# are never overwritten. Requires a clean work tree unless ALLOW_DIRTY=1.
#
#   package-release.sh OUT
set -euo pipefail
out=${1:?usage: package-release.sh OUT}
crate=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$crate"
targets_data=$crate/../linux-targets/data/distros.tsv

version=$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -n 1)
commit=$(git rev-parse HEAD)
epoch=$(git log -1 --format=%ct)
if [ -n "$(git status --porcelain)" ] && [ "${ALLOW_DIRTY:-}" != 1 ]; then
  echo "package-release: work tree is not clean (set ALLOW_DIRTY=1 for a test build)" >&2
  exit 2
fi
name=kernel-abi-tools-$version-x86_64-linux
archive=$out/$name.tar.gz
mkdir -p "$out"
for f in "$archive" "$archive.sha256"; do
  [ ! -e "$f" ] || { echo "package-release: $f exists; refusing to overwrite" >&2; exit 2; }
done

target=x86_64-unknown-linux-musl
target_dir=$(cargo metadata --format-version 1 --no-deps --offline \
  | sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p')
CARGO_PROFILE_RELEASE_STRIP=true \
  cargo build -p kernel-abi-tools --release --locked --offline --target "$target" --bins
built=$target_dir/$target/release
bins=(kernel-seccomp syscall-probe)
for b in "${bins[@]}"; do
  elf=$built/$b
  if readelf -lW "$elf" | grep -q INTERP || readelf -dW "$elf" | grep -q NEEDED; then
    echo "package-release: $b is not statically linked" >&2; exit 1
  fi
done

# The committed profiles must be exactly what this commit's data generates.
check=$(mktemp -d); trap 'rm -rf "$check" "${stage_root:-}"' EXIT
"$built/kernel-seccomp" generate "$check/profiles" 2>/dev/null
diff -r "$check/profiles" profiles >/dev/null \
  || { echo "package-release: profiles/ is stale; run kernel-seccomp generate profiles" >&2; exit 1; }

stage_root=$(mktemp -d)
stage=$stage_root/$name
mkdir -p "$stage/bin" "$stage/scripts" "$stage/docs"
for b in "${bins[@]}"; do install -m 0755 "$built/$b" "$stage/bin/$b"; done
cp -R profiles data "$stage/"
install -m 0644 "$targets_data" "$stage/data/distros.tsv"
for s in run-under-kernel.sh self-test.sh; do install -m 0755 "scripts/$s" "$stage/scripts/$s"; done
install -m 0644 LICENSE NOTICE README.md "$stage/"
install -m 0644 docs/PROVIDER-CONTRACT.md docs/RELEASE.md "$stage/docs/"
find "$stage" -type d -exec chmod 0755 {} +
find "$stage" -type f ! -perm -u+x -exec chmod 0644 {} +

"$stage/bin/kernel-seccomp" descriptor --commit "$commit" > "$stage/kernel-abi-provider.txt"
chmod 0644 "$stage/kernel-abi-provider.txt"
(cd "$stage" && find . -type f ! -name SHA256SUMS | sed 's|^\./||' | LC_ALL=C sort \
  | while read -r f; do sha256sum "$f"; done > SHA256SUMS)
chmod 0644 "$stage/SHA256SUMS"
"$stage/bin/kernel-seccomp" verify-bundle "$stage"

find "$stage" -exec touch -h -d "@$epoch" {} +
tar -C "$stage_root" --sort=name --mtime="@$epoch" --owner=0 --group=0 --numeric-owner \
  --format=gnu -cf - "$name" | gzip -n -9 > "$archive"
(cd "$out" && sha256sum "$name.tar.gz" > "$name.tar.gz.sha256")
echo "package-release: wrote $archive"
cat "$archive.sha256"

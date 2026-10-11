#!/bin/bash
# Disposable Debian 9 builder (GCC 6.3, binutils 2.28, make 4.1, Python 3.5);
# 2.35 needs GCC 6.2, make 4.0 and Python 3.4, and binutils 2.25. Its glibc is bootstrap-only, not a 2.35 runtime.
set -euo pipefail
cd /work
mkdir -p logs source prebuilt/usr/lib64
# Archived stretch packages; apt still verifies the signed Release files.
cat > /etc/apt/sources.list <<'SOURCES'
deb http://archive.debian.org/debian stretch main
SOURCES
apt-get -o Acquire::Check-Valid-Until=false update > logs/packages.log 2>&1
DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
    gcc make gawk bison python3 xz-utils linux-libc-dev libc6-dev >> logs/packages.log 2>&1
dpkg-query -W -f '${Package} ${Version} ${Architecture}\n' | sort > logs/packages.txt
gcc --version > logs/builder-gcc.txt
cp -L /lib/x86_64-linux-gnu/libgcc_s.so.1 prebuilt/usr/lib64/libgcc_s.so.1
tar -xf glibc-2.35.tar.xz
# 2.35 already has the upstream freestanding syscall-header fix (added in 2.18). No source patch.
: > logs/source-adjustment.patch
mkdir source-build
cd source-build
../glibc-2.35/configure --prefix=/usr --libdir=/usr/lib64 --disable-werror --enable-kernel=3.2 > ../logs/source-configure.log 2>&1
make -j4 > ../logs/source-build.log 2>&1
make install_root=/work/source install > ../logs/source-install.log 2>&1
# Debian keeps the asm UAPI headers in the multiarch directory.
cp -a /usr/include/linux /usr/include/asm-generic /work/source/usr/include/
cp -aL /usr/include/x86_64-linux-gnu/asm /work/source/usr/include/asm
printf 'Source-built glibc 2.35 inputs installed in the isolated work tree.\n'

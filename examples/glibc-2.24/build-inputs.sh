#!/bin/bash
# Disposable CentOS 7 builder. Its glibc is bootstrap-only, not a 2.24 runtime.
set -euo pipefail
cd /work
mkdir -p logs source prebuilt/usr/lib64
cat > /etc/yum.repos.d/experiment.repo <<'EOF'
[experiment]
name=Archived CentOS 7.9 bootstrap tools
baseurl=https://vault.centos.org/7.9.2009/os/x86_64/
enabled=1
gpgcheck=1
gpgkey=file:///etc/pki/rpm-gpg/RPM-GPG-KEY-CentOS-7
EOF
yum --disablerepo='*' --enablerepo=experiment -y install gcc make glibc-devel kernel-headers tar xz gawk > logs/packages.log 2>&1
rpm -qa | sort > logs/packages.txt
gcc --version > logs/builder-gcc.txt
cp -L /usr/lib64/libgcc_s.so.1 prebuilt/usr/lib64/libgcc_s.so.1
tar -xf glibc-2.24.tar.xz
# 2.24 already has the upstream freestanding syscall-header fix (added in 2.18). No source patch.
: > logs/source-adjustment.patch
mkdir source-build
cd source-build
../glibc-2.24/configure --prefix=/usr --libdir=/usr/lib64 --disable-werror --enable-kernel=3.2 > ../logs/source-configure.log 2>&1
make -j4 > ../logs/source-build.log 2>&1
make install_root=/work/source install > ../logs/source-install.log 2>&1
for dir in linux asm asm-generic; do
    cp -a "/usr/include/$dir" /work/source/usr/include/
done
printf 'Source-built glibc 2.24 inputs installed in the isolated work tree.\n'

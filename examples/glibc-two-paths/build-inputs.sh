#!/bin/bash
# Run inside a disposable CentOS 7 builder, with /work mounted.
set -euo pipefail
cd /work
mkdir -p logs prebuilt/usr/include prebuilt/usr/lib64 prebuilt/lib64 source
cat > /etc/yum.repos.d/experiment.repo <<'EOF'
[experiment]
name=Archived CentOS 7.9 baseline
baseurl=https://vault.centos.org/7.9.2009/os/x86_64/
enabled=1
gpgcheck=1
gpgkey=file:///etc/pki/rpm-gpg/RPM-GPG-KEY-CentOS-7
EOF
yum --disablerepo='*' --enablerepo=experiment -y install gcc make glibc-devel kernel-headers tar xz gawk > logs/packages.log 2>&1
rpm -qa | sort > logs/packages.txt
gcc --version > logs/builder-gcc.txt
# Predefined macros record defaults such as PIE that build commands leave implicit.
echo | gcc -dM -E - | sort > logs/builder-defaults.txt
cp -a /usr/include/. prebuilt/usr/include/
cp -a /usr/lib64/. prebuilt/usr/lib64/
cp -a /lib64/. prebuilt/lib64/
tar -xf glibc-2.17.tar.xz
# GCC 4.8 implicitly includes stdc-predef.h in hosted preprocessing. Restrict
# this workaround to syscall-header generation, where it creates a make loop.
python - <<'PY'
path = 'glibc-2.17/sysdeps/unix/sysv/linux/Makefile'
with open(path) as source_file:
    original = source_file.read()
before = '$(CC) -E -MD -MP -MF'
after = '$(CC) -ffreestanding -E -MD -MP -MF'
assert original.count(before) == 2, 'Unexpected upstream Makefile'
with open(path + '.original', 'w') as original_file:
    original_file.write(original)
with open(path, 'w') as patched_file:
    patched_file.write(original.replace(before, after))
PY
diff -u glibc-2.17/sysdeps/unix/sysv/linux/Makefile.original glibc-2.17/sysdeps/unix/sysv/linux/Makefile > logs/source-adjustment.patch || test "$?" -eq 1
mkdir -p source-build
cd source-build
../glibc-2.17/configure --prefix=/usr --disable-werror --enable-kernel=2.6.32 > ../logs/source-configure.log 2>&1
make -j4 > ../logs/source-build.log 2>&1
make install_root=/work/source install > ../logs/source-install.log 2>&1
# glibc installs its own headers; Linux UAPI headers are a separate input.
for dir in linux asm asm-generic; do
    cp -a "/usr/include/$dir" /work/source/usr/include/
done
printf 'Both input trees assembled.\n'

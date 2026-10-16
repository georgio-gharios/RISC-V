#!/bin/sh
# Build a minimal picolibc for rv32im (clang) into third_party/picolibc-install.
set -e
cd "$(dirname "$0")/.."
[ -d third_party/picolibc ] || git clone --depth 1 https://github.com/picolibc/picolibc third_party/picolibc
MESON=${MESON:-meson}
rm -rf third_party/picolibc-build
$MESON setup third_party/picolibc-build third_party/picolibc \
  --cross-file doom/cross-rv32im.txt \
  --prefix "$PWD/third_party/picolibc-install" \
  -Dmultilib=false -Dpicocrt=false -Dsemihost=false -Dtests=false \
  -Dthread-local-storage=false -Dformat-default=integer -Dspecsdir=none \
  -Dincludedir=include -Dlibdir=lib \
  -Dc_args='-O2'
ninja -C third_party/picolibc-build install

#!/bin/sh
# Build riscv-tests rv32ui + rv32um into raw binaries in tests/riscv-bin/.
# Uses our minimal env (tests/riscv-env) instead of env/p, and clang + lld.
set -e
cd "$(dirname "$0")/.."
SRC=third_party/riscv-tests
REV=d7e86f701b929b5d801f8cda1a4871ab05f452bb
if [ ! -d $SRC ]; then
  git clone --recurse-submodules https://github.com/riscv-software-src/riscv-tests $SRC
fi
git -C $SRC checkout -q $REV
OUT=tests/riscv-bin
mkdir -p $OUT
for suite in rv32ui rv32um; do
  tests=$(sed -n "/^${suite}_sc_tests = /,/^\$/p" $SRC/isa/$suite/Makefrag | tr -d '\\' | sed 's/.*=//')
  for t in $tests; do
    clang --target=riscv32 -march=rv32im -mabi=ilp32 -nostdlib -fuse-ld=lld -Wl,--no-relax \
      -I tests/riscv-env -I $SRC/isa/macros/scalar -T tests/riscv-env/link.ld \
      $SRC/isa/$suite/$t.S -o $OUT/$suite-$t.elf
    llvm-objcopy -O binary $OUT/$suite-$t.elf $OUT/$suite-$t.bin
    rm $OUT/$suite-$t.elf
  done
done
ls $OUT | wc -l

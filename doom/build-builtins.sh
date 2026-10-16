#!/bin/sh
# Build compiler-rt builtins (64-bit division, soft-float) for rv32im into third_party/builtins/libbuiltins.a.
set -e
cd "$(dirname "$0")/.."
SRC=third_party/llvm-project/compiler-rt/lib/builtins
if [ ! -d $SRC ]; then
  git clone --depth 1 --filter=blob:none --sparse https://github.com/llvm/llvm-project third_party/llvm-project
  git -C third_party/llvm-project sparse-checkout set compiler-rt/lib/builtins
fi
OUT=third_party/builtins
rm -rf $OUT && mkdir -p $OUT
ok=0; skipped=0
for f in $SRC/*.c; do
  o=$OUT/$(basename ${f%.c}).o
  if clang --target=riscv32-unknown-elf -march=rv32im -mabi=ilp32 -O2 -ffreestanding \
       -fno-builtin -w -c $f -o $o 2>/dev/null; then ok=$((ok+1)); else skipped=$((skipped+1)); fi
done
llvm-ar rcs $OUT/libbuiltins.a $OUT/*.o
echo "builtins: $ok compiled, $skipped skipped (not applicable to rv32 bare metal)"

# Bare-metal RV32IM programs for rv-doom. Toolchain (from Phase 0): clang + ld.lld + llvm-objcopy.
# Every program is linked at 0x80000000 and turned into a raw binary.
CC      = clang
CFLAGS  = --target=riscv32 -march=rv32im -mabi=ilp32 -O2 -ffreestanding -nostdlib \
          -fno-builtin -Wall -Wextra
LDFLAGS = -fuse-ld=lld -Wl,--no-relax -Wl,-T,link.ld
OBJCOPY = llvm-objcopy

C_PROGS = hello mandel primes
BINS    = hello_asm.bin $(C_PROGS:=.bin)

all: $(BINS)

hello_asm.elf: hello_asm.S
	$(CC) --target=riscv32 -march=rv32i -mabi=ilp32 -nostdlib -fuse-ld=lld -Wl,--no-relax \
	  -Wl,-Ttext=0x80000000 $< -o $@

%.elf: %.c crt0.S rt.c rt.h link.ld
	$(CC) $(CFLAGS) $(LDFLAGS) crt0.S rt.c $< -o $@

%.bin: %.elf
	$(OBJCOPY) -O binary $< $@

# Reference output: the same C sources compiled for the host (rt_host.c instead of rt.c).
expected: $(C_PROGS:=.expected)
%.expected: %.c rt_host.c rt.h
	cc -O2 -o /tmp/$*.host rt_host.c $<
	/tmp/$*.host > $@

.PRECIOUS: %.elf
clean:
	rm -f *.elf *.bin

# Assemble/compile bare-metal RV32 programs into raw binaries loaded at 0x80000000.
CC      = clang --target=riscv32 -march=rv32i -mabi=ilp32 -nostdlib -ffreestanding
LDFLAGS = -fuse-ld=lld -Wl,-Ttext=0x80000000 -Wl,--no-relax
OBJCOPY = llvm-objcopy

.PRECIOUS: %.elf

all: hello.bin

%.elf: %.S
	$(CC) $(LDFLAGS) $< -o $@

%.bin: %.elf
	$(OBJCOPY) -O binary $< $@

clean:
	rm -f *.elf *.bin

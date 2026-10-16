# Doom for rv-doom: doomgeneric + picolibc + compiler-rt builtins, WAD embedded.
# Prereqs: ./build-picolibc.sh, ./build-builtins.sh, doomgeneric cloned in ../third_party.
TP      = ../third_party
DG      = $(TP)/doomgeneric/doomgeneric
PICO    = $(TP)/picolibc-install
WAD    ?= /usr/share/games/doom/doom1.wad

CC      = clang
CFLAGS  = --target=riscv32-unknown-elf -march=rv32im -mabi=ilp32 -O2 -ffreestanding -nostdlib \
          -isystem $(PICO)/include -I$(DG) -DDOOMGENERIC_RESX=320 -DDOOMGENERIC_RESY=200 \
          -DNORMALUNIX -w
LDFLAGS = -fuse-ld=lld -Wl,--no-relax -Wl,-T,link.ld -Wl,--gc-sections
LIBS    = -L$(PICO)/lib -lc $(TP)/builtins/libbuiltins.a

DG_SRCS := $(filter-out doomgeneric_xlib.c,$(patsubst %.o,%.c,$(shell sed -n 's/^SRC_DOOM = //p' $(DG)/Makefile)))
OBJS    = $(addprefix obj/,$(DG_SRCS:.c=.o)) obj/doomgeneric_rvdoom.o obj/syscalls.o obj/wad.o obj/crt0.o

all: doom.bin

obj/%.o: $(DG)/%.c | obj
	$(CC) $(CFLAGS) -ffunction-sections -fdata-sections -c $< -o $@
obj/%.o: %.c | obj
	$(CC) $(CFLAGS) -Wall -c $< -o $@
obj/wad.o: wad.S $(WAD) | obj
	$(CC) $(CFLAGS) -DWAD_PATH='"$(WAD)"' -c $< -o $@
obj/crt0.o: ../programs/crt0.S | obj
	$(CC) $(CFLAGS) -c $< -o $@
obj:
	mkdir -p obj

doom.elf: $(OBJS) link.ld
	$(CC) $(CFLAGS) $(LDFLAGS) $(OBJS) $(LIBS) -o $@

doom.bin: doom.elf
	llvm-objcopy -O binary $< $@

clean:
	rm -rf obj doom.elf doom.bin

# rv-doom

RV32IM emulator in Rust (work in progress). See PLAN.md.

## Layout
- `src/decode.rs` – 32-bit word → `Instr`
- `src/cpu.rs` – registers, pc, execute step
- `src/mem.rs` – bus: RAM at 0x80000000 + device routing
- `src/devices.rs` – UART
- `programs/` – bare-metal programs: `crt0.S`, `link.ld`, `rt.c` (UART print helpers, memset/memcpy), examples

## Build / run
    cargo build --release
    cargo run --release -- programs/hello.bin [max_steps]
    cargo test --release          # unit tests, asm hello, C examples, all riscv-tests

The emulator exits with the program's exit code (`main`'s return value).

## C toolchain (bare metal, no libc)
Needs `clang`, `ld.lld`, `llvm-objcopy` (Ubuntu: `apt-get install clang lld llvm`).

    make -C programs              # builds hello_asm.bin, hello.bin, mandel.bin, primes.bin
    make -C programs expected     # host-compiled reference output (*.expected), uses cc
    cargo run --release -- programs/mandel.bin

One program by hand (what the Makefile does):

    cd programs
    clang --target=riscv32 -march=rv32im -mabi=ilp32 -O2 -ffreestanding -nostdlib -fno-builtin \
          -fuse-ld=lld -Wl,--no-relax -Wl,-T,link.ld crt0.S rt.c hello.c -o hello.elf
    llvm-objcopy -O binary hello.elf hello.bin

`crt0.S` sets sp to the top of RAM, zeroes .bss, calls `main`, then `ecall`s with its return value.
Built `.bin` and `.expected` files are committed so `cargo test` works without the cross toolchain.

## Official riscv-tests (rv32ui + rv32um)
    ./scripts/build-riscv-tests.sh            # clones riscv-tests into third_party/, rebuilds tests/riscv-bin/*.bin
    cargo run --release --bin riscv-tests     # prints pass/fail table
The prebuilt binaries in `tests/riscv-bin/` are committed, so `cargo test` needs no cross toolchain.
They use a minimal env (`tests/riscv-env/riscv_test.h`): pass = `ecall` with a0=0, fail = `ecall` with a0=(test#<<1)|1.

## Machine
| Address | What |
|---|---|
| 0x10000000 | UART: write byte to +0 prints it; +5 reads 0x20 (TX ready) |
| 0x80000000 | RAM (64 MiB). Raw binary loaded here, pc starts here, sp = top of RAM |

`ecall` halts with exit code = a0. `ebreak` halts.

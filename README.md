# rv-doom

RV32IM emulator in Rust (work in progress). See PLAN.md.

## Layout
- `src/decode.rs` – 32-bit word → `Instr`
- `src/cpu.rs` – registers, pc, execute step
- `src/mem.rs` – bus: RAM at 0x80000000 + device routing
- `src/devices.rs` – UART
- `programs/` – bare-metal test programs (built with clang + lld)

## Build / run
    make -C programs              # needs clang, ld.lld, llvm-objcopy
    cargo run --release -- programs/hello.bin [max_steps]
    cargo test                    # unit tests + hello + all riscv-tests

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

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
    cargo test

## Machine
| Address | What |
|---|---|
| 0x10000000 | UART: write byte to +0 prints it; +5 reads 0x20 (TX ready) |
| 0x80000000 | RAM (64 MiB). Raw binary loaded here, pc starts here, sp = top of RAM |

`ecall` halts with exit code = a0. `ebreak` halts.

# PLAN

Hard stop after each phase. Do not start the next until told "next".

- [x] **Phase 1: RV32I core.** Flat memory, UART at 0x10000000, raw binary loader.
  *Accept:* a hand-assembled or tiny compiled program prints "hello" via the UART.
  *Fallback:* hand-assemble words in a Rust test if no compiler works.
  *Done:* `programs/hello.S` (clang-assembled) prints "hello" via UART; `cargo test` passes.
- [ ] **Phase 2: M extension + riscv-tests runner** (rv32ui, rv32um).
  *Accept:* all tests pass; pass/fail table printed.
  *Fallback:* write our own self-checking asm tests if riscv-tests can't be built.
- [ ] **Phase 3: Bare-metal C toolchain** (crt0, linker script, Makefile) + examples (hello, ASCII mandelbrot, primes).
  *Accept:* all run correctly on the emulator.
  *Fallback:* use the alternate compiler (see Phase 0 table).
- [ ] **Phase 4: Devices.** 320x200 framebuffer, keyboard register, ms timer; memory map documented in README.
  *Accept:* a C test program draws to the framebuffer and the PNG dump matches expectations.
  *Fallback:* drop keyboard/timer polish, keep framebuffer.
- [ ] **Phase 5: doomgeneric port** with picolibc and doom1.wad embedded in memory.
  *Accept:* first rendered frame dumps to PNG.
  *Fallback:* ship Phase 3 programs in the browser.
- [ ] **Phase 6: WASM build.** Canvas rendering, keyboard input, side panel (registers, current disassembled instruction, instructions/sec).
  *Accept:* runs in a browser.
  *Fallback:* render without the side panel; or ship Phase 3 programs only.

## Phase 0 findings
| Item | Status | Fallback / note |
|---|---|---|
| rustc/cargo 1.97 | OK | |
| wasm32-unknown-unknown target | OK (installed) | |
| wasm-pack 0.15 | OK (installed via cargo) | |
| RISC-V compiler | `clang --target=riscv32` (v18) compiles; `ld.lld`, `llvm-objcopy` present. `riscv64-linux-gnu-gcc` installed via apt, works with `-march=rv32im -mabi=ilp32 -nostdlib` | Prefer clang+lld; gcc as alternate. No `riscv64-unknown-elf-gcc` package |
| riscv-tests | OK: `git clone` works (github.com HTTPS/codeload tarballs return 403; git clone and raw.githubusercontent.com work) | clone into `third_party/` (gitignored) |
| doomgeneric | OK via git clone | |
| picolibc | OK via git clone (source only; must build for rv32im ourselves) | newlib not checked; or write minimal libc |
| doom1.wad | Not fetched yet: archive.org/doomworld/ibiblio blocked; apt has `doom-wad-shareware` and `freedoom`; a raw.githubusercontent mirror returned 200 | use apt package |


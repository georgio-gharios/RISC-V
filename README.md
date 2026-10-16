# rv-doom

RV32IM emulator in Rust (work in progress). See PLAN.md.

## Layout
- `src/decode.rs` – 32-bit word → `Instr`
- `src/cpu.rs` – registers, pc, execute step
- `src/mem.rs` – bus: RAM at 0x80000000 + device routing
- `src/devices.rs` – UART, timer, keyboard, framebuffer
- `src/disasm.rs`, `src/png.rs` – disassembler (browser panel), PNG writer
- `web/` – WASM front end (`web/src/lib.rs` exports, `web/www/` static page)
- `programs/` – bare-metal programs: `crt0.S`, `link.ld`, `rt.c` (UART print helpers, memset/memcpy), examples

## Build / run
    cargo build --release
    cargo run --release -- programs/hello.bin [max_steps]
    cargo test --release          # unit tests, asm hello, C examples, all riscv-tests

The emulator exits with the program's exit code (`main`'s return value).

## C toolchain (bare metal, no libc)
Needs `clang`, `ld.lld`, `llvm-objcopy` (Ubuntu: `apt-get install clang lld llvm`).

    make -C programs              # builds hello_asm, hello, mandel, primes, fbtest, devtest (.bin)
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

## Memory map
| Address | Size | Device | Access |
|---|---|---|---|
| `0x1000_0000` | 0x100 | UART: write byte to +0 prints it; +5 reads 0x20 (TX ready) | R/W |
| `0x1100_0000` | 4 | Timer: milliseconds since start (u32). Set by the host: wall clock in the CLI | R |
| `0x1200_0000` | 4 | Keyboard: a word read pops one event. 0 = none, else `(pressed << 8) \| keycode` | R |
| `0x1300_0000` | 4 | Display control: any store = frame done (counter +1); read = frame counter | R/W |
| `0x2000_0000` | 256000 | Framebuffer: 320x200, 32-bit `0x00RRGGBB`, pixel (x,y) at +4*(y*320+x) | R/W |
| `0x8000_0000` | 64 MiB | RAM. Raw binary loaded here, pc starts here, sp = top of RAM | R/W |

C definitions are in `programs/devices.h`. `ecall` halts with exit code = a0. `ebreak` halts.

## CLI
    rv-doom <program.bin> [--max-steps N] [--png out.png] [--png-at-frame N]
    cargo run --release -- programs/fbtest.bin --png fb.png     # dump the framebuffer (see docs/fbtest.png)
`--png-at-frame N` stops once the guest has signalled N frames. Prints instruction count and MIPS on exit.

## C toolchain (bare metal, no libc)
Needs `clang`, `ld.lld`, `llvm-objcopy` (Ubuntu: `apt-get install clang lld llvm`).

    make -C programs              # builds hello_asm, hello, mandel, primes, fbtest, devtest (.bin)
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

## Doom (doomgeneric + picolibc)
Prereqs: clang/lld/llvm, `pip install meson`, ninja, and the shareware WAD (`apt-get install doom-wad-shareware`,
installs `/usr/share/games/doom/doom1.wad`). Neither the WAD nor `doom.bin` is committed.

    git clone https://github.com/ozkl/doomgeneric third_party/doomgeneric
    ./doom/build-picolibc.sh       # minimal picolibc for rv32im -> third_party/picolibc-install
    ./doom/build-builtins.sh       # compiler-rt builtins (64-bit div, soft float) -> third_party/builtins
    make -C doom -j8               # -> doom/doom.bin (WAD embedded; override with WAD=path)
    cargo run --release -- doom/doom.bin --png-at-frame 1 --png frame1.png
    cargo test --release -- --include-ignored   # includes the Doom first-frame test

Port files: `doom/doomgeneric_rvdoom.c` (DG_* hooks on our devices), `doom/syscalls.c` (UART stdio,
in-memory read-only `doom1.wad`, other file ops fail), `doom/wad.S` (.incbin), `doom/link.ld` (heap = end of bss .. stack).
Boot to first frame: ~260M instructions (~4.5 s); about 2.5M instructions per frame afterwards.

## Browser (WASM)
Plain `extern "C"` exports + a hand-written JS loader (no wasm-bindgen/wasm-pack: their binary
download from GitHub is blocked here). Needs `rustup target add wasm32-unknown-unknown`.

    ./web/build.sh                          # -> web/www/{rv_doom_web.wasm, programs/, doom.bin if built}
    python3 -m http.server -d web/www 8000  # open http://localhost:8000
    cd web && npm install && npm run smoke  # headless Chromium test (Playwright), checks Doom + Mandelbrot
    cd web && npm run measure                # headless run that prints the debug overlay (fps, MIPS, timer drift, key log)

In the browser, press <kbd>`</kbd> (backquote, never sent to Doom) or open `/?debug` for the debug overlay:
display/guest fps, MIPS, share of wall time the guest ran, guest-clock vs wall drift, Doom `gametic` rate (35/s
is correct) and game-time drift, held keys, and the last 10 key events with their fate (delivered / queued / dropped).

Controls: arrows move, Ctrl fire, Space use, Shift run, Alt strafe, Enter/Esc menu, Y to confirm.
If `doom.bin` is not deployed, the page falls back to the Phase 3 programs. See `docs/web.png`.
Speed: ~45-50 MIPS / ~20 fps in headless Chromium; ~79 MIPS natively.

# PLAN

Hard stop after each phase. Do not start the next until told "next".

- [x] **Phase 1: RV32I core.** Flat memory, UART at 0x10000000, raw binary loader.
  *Accept:* a hand-assembled or tiny compiled program prints "hello" via the UART.
  *Fallback:* hand-assemble words in a Rust test if no compiler works.
  *Done:* `programs/hello_asm.S` (clang-assembled, renamed from hello.S in Phase 3) prints "hello" via UART; `cargo test` passes.
- [x] **Phase 2: M extension + riscv-tests runner** (rv32ui, rv32um).
  *Accept:* all tests pass; pass/fail table printed.
  *Fallback:* write our own self-checking asm tests if riscv-tests can't be built.
  *Done:* 50/50 pass (42 rv32ui + 8 rv32um), riscv-tests @ d7e86f7, built with clang against a minimal no-CSR env. Runner verified to catch planted bugs.
- [x] **Phase 3: Bare-metal C toolchain** (crt0, linker script, Makefile) + examples (hello, ASCII mandelbrot, primes).
  *Accept:* all run correctly on the emulator.
  *Fallback:* use the alternate compiler (see Phase 0 table).
  *Done:* clang+lld, no libc (crt0.S, link.ld, rt.c). hello, mandel (Q12 fixed point), primes output byte-identical to the host-compiled reference.
- [x] **Phase 4: Devices.** 320x200 framebuffer, keyboard register, ms timer; memory map documented in README.
  *Accept:* a C test program draws to the framebuffer and the PNG dump matches expectations.
  *Fallback:* drop keyboard/timer polish, keep framebuffer.
  *Done:* fbtest.c pattern checked pixel-by-pixel in tests/phase4.rs, PNG decoded with the png crate and compared; devtest.c checks timer wait + keyboard queue. ~55-60 MIPS (release).
- [x] **Phase 5: doomgeneric port** with picolibc and doom1.wad embedded in memory.
  *Accept:* first rendered frame dumps to PNG.
  *Fallback:* ship Phase 3 programs in the browser.
  *Done:* first frame equals TITLEPIC+PLAYPAL (with Doom's gamma table) on all 64000 pixels (tests/phase5.rs, ignored by default). Demo renders in 3D (docs/doom-frame150.png). No emulator changes needed.
- [x] **Phase 6: WASM build.** Canvas rendering, keyboard input, side panel (registers, current disassembled instruction, instructions/sec).
  *Accept:* runs in a browser.
  *Fallback:* render without the side panel; or ship Phase 3 programs only.
  *Done:* raw wasm32 exports (wasm-pack blocked), dark UI: MIPS/fps top, canvas left, registers + disassembly right, UART console. Headless Chromium smoke test passes (Doom boots, Enter opens menu, Mandelbrot fallback). Bus RAM fast path + no register snapshot: 54 -> 79 MIPS native. Verified headless only; real gameplay, phone layout and dropdown focus are unverified (see HANDOFF.md §4).

## Phase 0 findings
| Item | Status | Fallback / note |
|---|---|---|
| rustc/cargo 1.97 | OK | |
| wasm32-unknown-unknown target | OK (installed) | |
| wasm-pack 0.15 | Installed, but unusable: it downloads wasm-bindgen from GitHub releases (blocked) | plain `extern "C"` wasm exports instead |
| RISC-V compiler | `clang --target=riscv32` (v18) compiles; `ld.lld`, `llvm-objcopy` present. `riscv64-linux-gnu-gcc` installed via apt, works with `-march=rv32im -mabi=ilp32 -nostdlib` | Prefer clang+lld; gcc as alternate. No `riscv64-unknown-elf-gcc` package |
| riscv-tests | OK: `git clone` works (github.com HTTPS/codeload tarballs return 403; git clone and raw.githubusercontent.com work) | clone into `third_party/` (gitignored) |
| doomgeneric | OK via git clone | |
| picolibc | OK via git clone (source only; must build for rv32im ourselves) | newlib not checked; or write minimal libc |
| doom1.wad | archive.org/doomworld/ibiblio blocked | `apt-get install doom-wad-shareware` (used; v1.9, sha1 5b2e249b) |


## Polish phase
- [x] **Step 1: debug overlay** (` key or `?debug`; `npm run measure` prints it headless).
  *Measured (headless Chromium):* 60 rAF/s, 13-21 guest fps, 41-49 MIPS, guest runs ~77% of wall time,
  guest clock = wall clock (drift 0 ms, max jump 7-10 ms), gametic 34-37 tics/s, but game time is ~1.8 s behind
  wall time after starting E1M1 (lost during the level start, never caught up). All sent key events were
  delivered; repeats and unmapped keys are dropped as intended; Ctrl fired (ammo 50 -> 48). Screenshot: docs/debug-overlay.png.
- [x] **Step 2: focus and stuck keys.** Game keys handled at the document level (capture phase) whatever has
  focus; dropdown blurs after a pick; all held keys released on window blur, pagehide and tab hide.
  *Verified:* `npm run smoke` reads Doom's `gamekeydown[]`: Right+Ctrl = 1/1 while held, 0/0 after a window blur;
  ArrowDown on the focused dropdown reaches the guest and leaves it on Doom. Against the old main.js these checks
  fail (ArrowDown switched the program to Mandelbrot, resetting Doom).
- [x] **Step 3: frame-time breakdown (measure only).** Overlay adds per-rAF split, instr budget, guest wait share.
  *Measured (headless):* per 16.7 ms rAF: emulation 12.8, rgba convert 0.1, putImageData 0.03, UI 0.1, idle 3.5.
  0.66-0.76M instr/rAF at 51-59 MIPS while running; Doom needs 2.3M instr/frame (title), 3.3M (E1M1) -> 12-14 fps
  in game. Guest waits on the clock 0-4% of the time, so it is CPU-bound. Native: 73-88 MIPS; on a real trace,
  isolated fetch ~3 ns + decode ~9 ns vs 11.3 ns per full step (overlap, so decode share is uncertain).
  35 fps in E1M1 needs ~115 MIPS while running (~2x). Ranking in the step 3 report: decode cache, then Web Worker.

## Next (not started; see HANDOFF.md §6 for acceptance tests)
- [ ] Gameplay smoke test (fire/move in E1M1), verifies hypothesis A (input)
- [ ] Measure game speed via `gametic` (35 tics/s), verifies hypothesis B (timing)
- [ ] Decoded-instruction cache (target >= 120 MIPS native)
- [ ] Emulator in a Web Worker
- [ ] Phone-width layout check

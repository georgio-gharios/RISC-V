# HANDOFF: rv-doom

A from-scratch RISC-V **RV32IM** emulator in Rust that boots **Doom** (doomgeneric + picolibc, shareware WAD embedded), with a **WASM browser front end**.
Working rules are in `CLAUDE.md` (one phase at a time, never claim untested results, keep the core small). Phase history is in `PLAN.md`; user-facing commands are in `README.md`.
Branch: `claude/practical-keller-966nla`. All six planned phases are done. Nothing is known to be broken, but real gameplay has never been tested (see §4).

---

## 1. Current state and how to verify it

### What works (each item was run and its output checked)
| Feature | Verified by |
|---|---|
| RV32I + M core, flat RAM, UART | 50/50 official riscv-tests (42 rv32ui + 8 rv32um) |
| Bare-metal C (clang + lld, no libc) | hello / mandel / primes output byte-identical to the same C compiled for the host |
| Devices: framebuffer, keyboard, timer, PNG dump | C programs `fbtest.c` and `devtest.c`, checked pixel by pixel and line by line |
| Doom boots and renders | first frame equals TITLEPIC from the WAD on all 64000 pixels; the attract demo renders in 3D (`docs/doom-frame150.png`) |
| Browser front end | headless Chromium smoke test: Doom boots, Enter opens the menu, the Mandelbrot fallback works (`docs/web.png`) |

### Fresh-container setup (`third_party/` is gitignored, so it must be rebuilt)
```sh
cd /home/user/RISC-V                     # repo root
# clang, ld.lld, llvm-objcopy, ninja, node are preinstalled in this image; the WAD is NOT:
apt-get install -y doom-wad-shareware    # -> /usr/share/games/doom/doom1.wad (sha1 5b2e249b...)
pip install --user meson                 # -> ~/.local/bin/meson (not on PATH)
rustup target add wasm32-unknown-unknown
git clone https://github.com/ozkl/doomgeneric third_party/doomgeneric     # built at e257a09
MESON=~/.local/bin/meson ./doom/build-picolibc.sh   # clones picolibc (built at c8df19e) -> third_party/picolibc-install
./doom/build-builtins.sh                 # sparse-clones llvm compiler-rt -> third_party/builtins/libbuiltins.a
                                         # expect: "builtins: 167 compiled, 14 skipped ..."
make -C doom -j8                         # -> doom/doom.bin (~4.6 MB, WAD embedded). Not committed.
```
`./scripts/build-riscv-tests.sh` (clones riscv-tests @ d7e86f7) and `make -C programs` are only needed if you change those
sources: their `.bin` outputs are committed, so `cargo test` works without the cross toolchain.

### Native: build, run, test
```sh
cargo build --release
cargo test --release                      # expect 10 passed, 1 ignored (the Doom test), 0 failed
cargo test --release -- --include-ignored # expect 11 passed; needs doom/doom.bin and the WAD (~4 s)
cargo run --release --bin riscv-tests     # expect a table of 50 rows all PASS, then "50/50 passed"
cargo run --release -- programs/mandel.bin    # ASCII Mandelbrot, 28 lines x 72 chars, exit 0
cargo run --release -- programs/primes.bin    # ends with "count below 100000: 9592"
cargo run --release -- programs/fbtest.bin --png /tmp/fb.png   # "frames = 1"; PNG looks like docs/fbtest.png
./target/release/rv-doom doom/doom.bin --png-at-frame 1 --png /tmp/f1.png
#   expect Doom's boot log, then "[~260M instructions in ~3-4s ... 1 frames]"; f1.png = docs/doom-frame1.png
./target/release/rv-doom doom/doom.bin --png-at-frame 150 --png /tmp/f150.png
#   expect "[~610M instructions in ~7.7s = ~79 MIPS, 150 frames]" (wall-clock timer, so counts vary a bit)
```
Test files: `src/decode.rs` and `src/disasm.rs` (unit tests), `tests/phase1.rs` (asm hello), `tests/riscv_tests.rs` (50 binaries),
`tests/phase3.rs` (C vs `programs/*.expected`), `tests/phase4.rs` (devices), `tests/phase5.rs` (Doom, `#[ignore]`).

### WASM / browser
```sh
./web/build.sh                            # cargo wasm build + copies wasm, programs/*.bin, doom/doom.bin into web/www/
python3 -m http.server -d web/www 8000    # open http://localhost:8000
cd web && npm install && npm run smoke    # expect a JSON state dump, Mandelbrot lines, then "SMOKE TEST PASSED"
```
The smoke test (`web/smoke-test.mjs`) serves `web/www` itself on a random port and uses `/opt/pw-browsers/chromium`.
It saves a screenshot to `$SCREENSHOT` (default `/tmp/rv-doom-web.png`).
What it asserts:
- Doom is loaded and the canvas has at least 32 colors.
- MIPS > 1, pc starts with `80`, and the disassembly line starts with the pc.
- No halt status, and the console contains `I_InitGraphics`.
- After switching to Mandelbrot: exit code 0 and 28 lines of 72 chars.

---

## 2. Architecture
```
src/decode.rs   u32 -> enum Instr (Lui, Auipc, Jal, Jalr, Branch, Load, Store, OpImm, Op, Fence, Ecall, Ebreak); M ops are AluOp variants
src/cpu.rs      Cpu { regs, pc, bus, instret }; step() = fetch/decode/execute; run(n) -> Option<Halt>; ecall = exit(a0)
src/mem.rs      Bus: RAM + device routing by address; read/write(addr, width) are little-endian; read takes &mut (keyboard pops on read)
src/devices.rs  Uart, Timer, Keyboard (VecDeque), Framebuffer (+ frame counter)
src/disasm.rs   Instr -> text (no pseudo-ops); src/png.rs: dependency-free PNG writer (stored deflate)
src/lib.rs      machine(image, ram, echo) loads at 0x80000000, sp = top of RAM; riscv-test runner helpers; framebuffer_png
src/main.rs     CLI; runs in 100k-instruction chunks, setting timer.ms from wall clock before each chunk
web/src/lib.rs  cdylib with plain extern "C" exports (upload_buffer, load, run(steps, now_ms), key, frame_rgba, regs, disasm_current, take_uart...)
web/www/main.js the loader and UI loop; strings are passed back as (text_ptr(), len)

Memory map (C defines in programs/devices.h; the Doom port has its own copy in doom/doomgeneric_rvdoom.c)
  0x1000_0000 UART       +0 write = print byte; +5 read = 0x20 (TX ready)
  0x1100_0000 Timer      u32 ms, set by the HOST (wall clock in CLI and browser, exact values in tests)
  0x1200_0000 Keyboard   word read pops one event: 0 = none, else (pressed<<8)|doom_keycode; the pop happens on the byte-0 read
  0x1300_0000 FB ctrl    any store = frame done (frames += 1); read = frame count
  0x2000_0000 Framebuf   320x200 u32 0x00RRGGBB, row-major (256000 B)
  0x8000_0000 RAM        64 MiB; image loaded here; pc starts here
Browser loop: main.js tick() is driven by requestAnimationFrame. It calls wasm.run(100000, floor(performance.now())) repeatedly
  until 12 ms of that animation frame are used, then draws the canvas, drains the UART, and refreshes the panel at most every 100 ms.
  The guest does not run in the gaps between animation frames, but its clock keeps advancing.
RAM fast path: src/mem.rs Bus::read (line ~85) and Bus::write (line ~104): whole access in RAM -> slice + from/to_le_bytes.
Decode cache: src/cpu.rs Cpu::fetch keeps decoded Instrs in lazily allocated 4 KiB pages; a Store clears the
  words it touches (Cpu::invalidate). tests/icache.rs covers patching already-executed code (riscv-tests fence_i does not).
Register change: src/cpu.rs step() reads self.regs directly (no 128-byte snapshot per step); Jalr (line ~91) computes the target before writing rd.
```

---

## 3. Decisions and gotchas
**Toolchain**
- Use **clang 18 + ld.lld + llvm-objcopy** for everything: `--target=riscv32 -march=rv32im -mabi=ilp32`, plus `-Wl,--no-relax`.
- `riscv64-linux-gnu-gcc` was apt-installed in Phase 0 but is unused. There is no bare-metal `riscv64-unknown-elf-gcc` package.

**riscv-tests environment**
- The official tests' standard setup file needs CSRs, traps and `mret`. Instead, `tests/riscv-env/riscv_test.h` replaces it: pass = `ecall` with a0=0, fail = `ecall` with a0 = (test# << 1) | 1.
- The test sources themselves are unmodified. This is why the core has **no CSRs or traps**; keep it that way unless a phase needs them.

**C runtime**
- Plain programs: `programs/crt0.S`, `programs/link.ld`, `programs/rt.c`. `rt.c` provides memset/memcpy because clang emits calls to them.
- `mandel.c` uses Q12 fixed point, since there are no floats.
- `*.expected` files come from `make -C programs expected`: the same C built with host `cc` plus `rt_host.c`.

**Doom port** (files in `doom/`)
- `doomgeneric_rvdoom.c`:
  - `DG_Init` points `DG_ScreenBuffer` directly at the framebuffer, so there is no copy.
  - `DG_DrawFrame` only bumps the frame counter. The host can see a frame mid-update.
  - Built with `-DDOOMGENERIC_RESX=320 -DDOOMGENERIC_RESY=200`. doomgeneric's 32-bit "rgba8888" format matches our `0x00RRGGBB` exactly.
- `syscalls.c`:
  - stdout and stderr go to the UART through picolibc tinystdio `FDEV_SETUP_STREAM`.
  - `open/read/lseek/close/fstat` serve one read-only file, `doom1.wad`, from `wad.S` (`.incbin`).
  - Writes, mkdir, rename, unlink and system return -1, so config and savegames silently fail. The boot message "saving config in .default.cfg" is harmless.
- picolibc options: `-Dformat-default=integer`, so printf has no float support. Also no picocrt, no semihosting, no thread-local storage, multilib off.
- compiler-rt: `__addtf3` and the other 128-bit `long double` helpers are missing (rv32 has no `__int128`). Nothing links against them today.
- `doom/link.ld`: heap runs from the end of `.bss` to 1 MiB below the top of RAM, and it provides the `__bothinit_array_*` symbols picolibc wants.
- Doom never exits. Always use `--png-at-frame N` or `--max-steps` with it.

**The two test mistakes (the emulator was right both times)**
- **fbtest spot check:** I asserted pixel (160,100) was gradient color `0x7f8040`. But fbtest draws a red diagonal at x = y+60, so (160,100) is red.
  - The correct test checks (170,100) = `0x878040` (r = 170·255/319 = 135, g = 128).
  - It also compares all 64000 pixels against an independent Rust model (`expected_pixel`), and decodes the PNG with the `png` crate (dev-dependency only).
- **Doom gamma:** first I compared against the raw PLAYPAL palette (62573 pixels differ). Then I assumed gamma level 0 was v+1 everywhere (27053 differ). Both were wrong.
  - Doom's `gammatable[0]` bumps low values by 1 and is the identity near the top.
  - The correct test, `tests/phase5.rs`, parses row 0 of `gammatable` from `third_party/doomgeneric/doomgeneric/tables.c`. It decodes TITLEPIC (patch format) and PLAYPAL from the WAD, then asserts 0 of 64000 pixels differ.
  - The first frame is captured with a deterministic clock: +20 ms per million instructions.

**Performance**
- Native was ~54 MIPS. The RAM fast path brought it to ~67, and dropping the register snapshot to ~79 (Doom demo, release build, no `perf` available).
- Browser (headless Chromium): ~43–50 MIPS, ~17–20 fps. Doom costs ~2.5M instructions per frame after a ~260M-instruction boot, so the title screen appears after ~5–6 s in the browser.
- Native at ~79 MIPS would be ~30 fps if not frame-capped.

**Network and tooling in this environment**
- github.com tarball and codeload URLs return 403, but `git clone` and raw.githubusercontent.com work.
- crates.io, npm and PyPI work. archive.org, doomworld and ibiblio are blocked, which is why the WAD comes from apt.
- wasm-pack is installed but unusable: it downloads wasm-bindgen from GitHub releases, which is blocked. Hence the plain `extern "C"` exports.
- Playwright: use **playwright-core 1.56.1** with `/opt/pw-browsers/chromium`. Version 1.48 fails because that Chromium removed old headless mode. Never run `playwright install`.

**Generated files**
- `cargo run` needs `default-run = "rv-doom"` in Cargo.toml, because `src/bin/riscv-tests.rs` exists.
- Committed: `programs/*.bin`, `programs/*.expected`, `tests/riscv-bin/*.bin`. Rebuild and commit them if you change their sources.
- Gitignored: `doom/doom.bin`, `web/www/{*.wasm, doom.bin, programs/}`, `web/node_modules`.

---

## 4. Known weak spots (never verified)
- **Real gameplay:** nobody has started a game, held or combined keys, played a full level, or measured whether game speed is correct (35 tics/s). The only key ever sent was Enter, which opened the menu.
- **Narrow or phone layouts:** there is a `@media (max-width: 800px)` rule in `web/www/style.css`, but it has never been rendered or checked.
- **Keyboard focus with the program dropdown focused:** the window key handler in `main.js` returns early when `e.target === select`, without `preventDefault`. The browser then probably uses arrow keys to change the selection, firing `onchange`, which reloads the program and resets Doom. Not reproduced yet.
- **Window blur:** keys released while the tab is unfocused send no keyup, so keys can stay held. There is no blur handler.
- **Torn frames:** the host may draw a frame that Doom is still writing, since the screen buffer is the framebuffer itself.

## 5. Open hypotheses (nothing changed yet; the user chooses which to pursue)
**A. Input:** `e.repeat` is dropped, and Ctrl/Alt can trigger browser shortcuts, so keys may stick or never reach Doom.
- Where: `web/www/main.js`, the `for (const type of ["keydown","keyup"])` listener (line ~127), the `KEYS` map (line ~15), and `e.repeat` (line ~133).
- Guest side: `doom/doomgeneric_rvdoom.c` `DG_GetKey`.
- Dropping repeats is intended, since Doom tracks held state from down/up events. The risks:
  - Ctrl+W, Ctrl+T and similar can't be prevented.
  - Alt may focus the browser menu bar.
  - A keyup lost to blur, or to the dropdown having focus, leaves a key held.
- How to test: extend `web/smoke-test.mjs`:
  1. Start a game (Enter, Enter, Enter).
  2. `keyboard.down('Control')`, wait 1 s, `up`, and assert the ammo region of the status bar changed (compare canvas pixels at x≈0–50, y≈170–190 before and after). Or read `players[0].ammo` from RAM: get its address with `llvm-nm doom/doom.elf | grep players`, which needs a small `peek` export.
  3. Hold ArrowUp for 2 s and assert the view changed.
  4. Focus `#program`, press ArrowDown, and assert `#program.value` is unchanged.
  5. Do keydown, then page blur, then focus, and assert nothing is still held.

**B. Timing:** the guest clock is wall time and the guest only runs in ≤12 ms slices per animation frame. `DG_SleepMs` busy-waits on that clock, so ~20 fps may feel jerky or run at the wrong game speed.
- Where: `web/www/main.js` `tick()` (line ~90, the `do { wasm.run(100000, …) } while (… < 12)` loop), and `doom/doomgeneric_rvdoom.c` `DG_SleepMs` / `DG_GetTicksMs` (lines ~17–22).
- The guest gets at most ~72% of wall time at 60 Hz, less if the tab is throttled. The clock jumps across the gaps between animation frames, so Doom's tic loop sees uneven time steps. Sleeping burns guest instructions instead of yielding.
- How to test:
  - Measure game speed: read Doom's `gametic` from RAM (address via `llvm-nm doom/doom.elf | grep -w gametic`, plus a `peek` export) once per second in the smoke test, and assert ~35 tics/s.
  - Count the share of instructions retired inside `DG_SleepMs` by checking the pc range from `llvm-nm`, in a native run.
  - Compare against a run where the timer is advanced deterministically.

## 6. Next steps, in priority order (each with a runnable acceptance test)
1. **Focus and stuck keys** (cheap, likely real): don't let the dropdown eat arrow keys (blur it after change), and release all held keys on `window.blur`.
   *Accept:* new smoke-test checks: ArrowDown with the dropdown focused leaves `#program.value` unchanged; after blur, a Control keydown with no keyup doesn't keep firing. `npm run smoke` prints SMOKE TEST PASSED.
2. **Gameplay smoke test** (verifies hypothesis A): start E1M1, fire, move.
   *Accept:* `npm run smoke` asserts the ammo or status-bar pixels change after Control, and the view pixels change after ArrowUp.
3. **Measure game speed** (verifies hypothesis B): add a `peek(addr)` export and read `gametic`.
   *Accept:* the smoke test logs tics/s, and it is 35 ± 3 over 5 s after boot. If not, consider a deterministic guest clock (advance the timer by executed instructions or a fixed step per slice), or run the emulator in a Web Worker.
4. **Speed** (more fps): cache decoded instructions per RAM word, invalidated on store to that word (fence_i still has to pass).
   *Accept:* `cargo test --release -- --include-ignored` all pass, and the `--png-at-frame 150` run reports ≥ 120 MIPS.
5. **Web Worker** for the emulator loop, so the UI never stalls.
   *Accept:* the smoke test still passes, and fps in the browser ≥ native-equivalent minus 20%.
6. **Phone layout**.
   *Accept:* a Playwright run at a 390×844 viewport reports `document.documentElement.scrollWidth <= innerWidth`, and the canvas is fully visible in a screenshot.

## 7. Saves rediscovery time
- Run all commands from the repo root. Cargo, the scripts and the tests use relative paths (`programs/`, `tests/riscv-bin`, `doom/doom.bin`, `third_party/...`).
- Keyboard codes are **Doom keycodes** (`third_party/doomgeneric/doomgeneric/doomkeys.h`): arrows 0xac–0xaf, fire 0xa3, use 0xa2, RSHIFT 0xb6, RALT 0xb8, Enter 13, Esc 27. The browser maps keys in `main.js` `KEYS`.
- Doom's default bindings: fire = KEY_FIRE, use = KEY_USE, strafe = RALT, run = RSHIFT, menu confirm = 'y'.
- Doom args are hardcoded in `doom/doomgeneric_rvdoom.c` `main`: `-iwad doom1.wad`. The file name must be `doom1.wad`; `syscalls.c` `is_wad` matches the basename.
- In JS, always create a fresh typed-array view on `wasm.memory.buffer` for each call. The 64 MiB RAM allocation grows wasm memory, which detaches old views.
- The doomgeneric source list comes from its own Makefile's `SRC_DOOM` line (minus `doomgeneric_xlib.c`). See `doom/Makefile`.
- To find where doomgeneric needs libc: compile its sources natively with `cc -c` and run `nm -u` (this is how the Phase 5 list was made). 32-bit host headers are not installed, so don't use `-m32`.
- The riscv-tests runner reports a fail as `FAIL (test #N)`, where N is the `TESTNUM` from the test source. `CRASH` means an illegal instruction, a bus fault or a timeout.
- The emulator's exit code is the guest's `a0` at `ecall`. The CLI prints `[N instructions in Xs = Y MIPS, F frames]` to stderr on exit.

# rv-doom working rules

From-scratch RISC-V (RV32IM) emulator in Rust that eventually boots Doom, with a WASM browser front end. See PLAN.md for phases.

1. Work one phase at a time. At the end of a phase: run the tests, commit, update PLAN.md, then stop and report in under 10 lines (what works, what doesn't, what you'd do next). Wait for "next".
2. Never mark something done unless it was actually run and the output was checked.
3. Prefer the simplest thing that works. No extra extensions (no floats, no compressed instructions, no virtual memory, no interrupts) unless a phase says so.
4. Keep the emulator core in a small, readable module: `decode.rs`, `cpu.rs`, `mem.rs`, `devices.rs`.
5. If something is blocked (toolchain, downloads, network), say so immediately with a fallback, rather than working around it silently.
6. Commit after every phase with a clear message. Keep README.md current with exact build/run commands.
7. Keep responses short. Don't paste large files back.

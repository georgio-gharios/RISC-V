//! CLI: rv-doom <program.bin> [--max-steps N] [--png out.png] [--png-at-frame N]
//!   --png FILE        write the framebuffer as PNG when the program stops
//!   --png-at-frame N  stop after the guest finishes its Nth frame (requires --png)
use rv_doom::cpu::Halt;
use std::process::exit;
use std::time::Instant;

fn usage() -> ! {
    eprintln!("usage: rv-doom <program.bin> [--max-steps N] [--png out.png] [--png-at-frame N]");
    exit(2)
}

fn main() {
    let mut args = std::env::args().skip(1);
    let mut path = None;
    let (mut max_steps, mut png, mut stop_frame) = (u64::MAX, None, None);
    while let Some(a) = args.next() {
        let mut value = || args.next().unwrap_or_else(|| usage());
        match a.as_str() {
            "--max-steps" => max_steps = value().parse().unwrap_or_else(|_| usage()),
            "--png" => png = Some(value()),
            "--png-at-frame" => stop_frame = Some(value().parse::<u32>().unwrap_or_else(|_| usage())),
            _ if path.is_none() && !a.starts_with("--") => path = Some(a),
            _ => usage(),
        }
    }
    let path = path.unwrap_or_else(|| usage());
    let image = std::fs::read(&path).unwrap_or_else(|e| { eprintln!("{path}: {e}"); exit(2) });
    let mut cpu = rv_doom::machine(&image, rv_doom::DEFAULT_RAM, true).expect("image too large");

    // Run in chunks; between chunks, update the timer from the wall clock.
    let start = Instant::now();
    const CHUNK: u64 = 100_000;
    let mut halt = None;
    while halt.is_none() && cpu.instret < max_steps {
        cpu.bus.timer.ms = start.elapsed().as_millis() as u32;
        halt = cpu.run(CHUNK.min(max_steps - cpu.instret));
        if stop_frame.is_some_and(|n| cpu.bus.fb.frames >= n) { break; }
    }
    let secs = start.elapsed().as_secs_f64();
    eprintln!("\n[{} instructions in {:.2}s = {:.1} MIPS, {} frames]",
        cpu.instret, secs, cpu.instret as f64 / secs / 1e6, cpu.bus.fb.frames);

    if let Some(file) = png {
        std::fs::write(&file, rv_doom::framebuffer_png(&cpu)).expect("writing png");
        eprintln!("[framebuffer written to {file}]");
    }
    match halt {
        Some(Halt::Exit(code)) => exit(code as i32),
        Some(other) => { eprintln!("halted: {other:x?} at pc={:#010x}", cpu.pc); exit(1) }
        None if stop_frame.is_some() => exit(0),
        None => { eprintln!("step limit reached at pc={:#010x}", cpu.pc); exit(1) }
    }
}

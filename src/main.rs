use rv_doom::cpu::Halt;
use std::process::exit;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let Some(path) = args.get(1) else {
        eprintln!("usage: rv-doom <program.bin> [max_steps]");
        exit(2);
    };
    let max_steps: u64 = args.get(2).map(|s| s.parse().expect("max_steps")).unwrap_or(u64::MAX);
    let image = std::fs::read(path).unwrap_or_else(|e| { eprintln!("{path}: {e}"); exit(2) });

    let mut cpu = rv_doom::machine(&image, rv_doom::DEFAULT_RAM, true).expect("image too large");
    match cpu.run(max_steps) {
        Some(Halt::Exit(code)) => exit(code as i32),
        Some(other) => {
            eprintln!("\nhalted: {other:x?} after {} instructions", cpu.instret);
            exit(1);
        }
        None => {
            eprintln!("\nstep limit reached at pc={:#010x}", cpu.pc);
            exit(1);
        }
    }
}

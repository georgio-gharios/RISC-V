pub mod cpu;
pub mod decode;
pub mod devices;
pub mod mem;

use cpu::Cpu;
use mem::{Bus, RAM_BASE};

pub const DEFAULT_RAM: usize = 64 * 1024 * 1024;

/// Build a machine with `image` loaded at RAM_BASE, pc at RAM_BASE, sp at top of RAM.
pub fn machine(image: &[u8], ram_size: usize, echo_uart: bool) -> Result<Cpu, mem::BusError> {
    let mut bus = Bus::new(ram_size, echo_uart);
    bus.load(RAM_BASE, image)?;
    let mut cpu = Cpu::new(bus, RAM_BASE);
    cpu.regs[2] = RAM_BASE.wrapping_add(ram_size as u32); // sp
    Ok(cpu)
}

/// Result of running one riscv-tests binary (see tests/riscv-env/riscv_test.h).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestOutcome {
    Pass,
    Fail { test_num: u32 },
    Crash(String),
}

pub fn run_riscv_test(image: &[u8]) -> TestOutcome {
    let mut cpu = match machine(image, 1 << 20, false) {
        Ok(c) => c,
        Err(e) => return TestOutcome::Crash(format!("load: {e:x?}")),
    };
    match cpu.run(1_000_000) {
        Some(cpu::Halt::Exit(0)) => TestOutcome::Pass,
        Some(cpu::Halt::Exit(code)) => TestOutcome::Fail { test_num: code >> 1 },
        Some(other) => TestOutcome::Crash(format!("{other:x?}")),
        None => TestOutcome::Crash("timeout".into()),
    }
}

/// Run every *.bin in `dir` (sorted). Returns (name, outcome) pairs.
pub fn run_riscv_test_dir(dir: &str) -> std::io::Result<Vec<(String, TestOutcome)>> {
    let mut paths: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "bin"))
        .collect();
    paths.sort();
    let mut results = Vec::new();
    for p in paths {
        let name = p.file_stem().unwrap().to_string_lossy().into_owned();
        results.push((name, run_riscv_test(&std::fs::read(&p)?)));
    }
    Ok(results)
}

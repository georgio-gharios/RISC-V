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

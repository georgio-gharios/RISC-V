//! The bus: flat RAM plus memory-mapped devices. Little-endian.

use crate::devices::{Framebuffer, Keyboard, Timer, Uart};

pub const RAM_BASE: u32 = 0x8000_0000;

pub struct Bus {
    pub ram: Vec<u8>,
    pub uart: Uart,
    pub timer: Timer,
    pub keyboard: Keyboard,
    pub fb: Framebuffer,
}

fn in_range(addr: u32, base: u32, size: u32) -> bool {
    addr >= base && addr - base < size
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BusError(pub u32);

impl Bus {
    pub fn new(ram_size: usize, echo_uart: bool) -> Self {
        Bus {
            ram: vec![0; ram_size],
            uart: Uart::new(echo_uart),
            timer: Timer { ms: 0 },
            keyboard: Keyboard::new(),
            fb: Framebuffer::new(),
        }
    }

    /// Copy a raw binary into RAM at `addr`.
    pub fn load(&mut self, addr: u32, data: &[u8]) -> Result<(), BusError> {
        let start = addr.wrapping_sub(RAM_BASE) as usize;
        let end = start + data.len();
        if addr < RAM_BASE || end > self.ram.len() {
            return Err(BusError(addr));
        }
        self.ram[start..end].copy_from_slice(data);
        Ok(())
    }

    fn read8(&mut self, addr: u32) -> Result<u8, BusError> {
        if addr >= RAM_BASE {
            self.ram.get((addr - RAM_BASE) as usize).copied().ok_or(BusError(addr))
        } else if in_range(addr, Framebuffer::BASE, Framebuffer::SIZE) {
            Ok(self.fb.bytes[(addr - Framebuffer::BASE) as usize])
        } else if in_range(addr, Uart::BASE, Uart::SIZE) {
            Ok(self.uart.read8(addr - Uart::BASE))
        } else if in_range(addr, Timer::BASE, Timer::SIZE) {
            Ok(self.timer.read8(addr - Timer::BASE))
        } else if in_range(addr, Keyboard::BASE, Keyboard::SIZE) {
            Ok(self.keyboard.read8(addr - Keyboard::BASE))
        } else if in_range(addr, Framebuffer::CTRL, 4) {
            Ok((self.fb.frames >> (8 * (addr - Framebuffer::CTRL))) as u8)
        } else {
            Err(BusError(addr))
        }
    }

    fn write8(&mut self, addr: u32, value: u8) -> Result<(), BusError> {
        if addr >= RAM_BASE {
            let slot = self.ram.get_mut((addr - RAM_BASE) as usize).ok_or(BusError(addr))?;
            *slot = value;
            Ok(())
        } else if in_range(addr, Framebuffer::BASE, Framebuffer::SIZE) {
            self.fb.bytes[(addr - Framebuffer::BASE) as usize] = value;
            Ok(())
        } else if in_range(addr, Uart::BASE, Uart::SIZE) {
            self.uart.write8(addr - Uart::BASE, value);
            Ok(())
        } else if addr == Framebuffer::CTRL {
            self.fb.frames += 1; // any store to the control word = "frame done"
            Ok(())
        } else if in_range(addr, Framebuffer::CTRL, 4) {
            Ok(()) // upper bytes of a word store: ignore
        } else {
            Err(BusError(addr))
        }
    }

    /// Read `width` bytes (1, 2 or 4) little-endian.
    pub fn read(&mut self, addr: u32, width: u32) -> Result<u32, BusError> {
        let mut v = 0u32;
        for k in 0..width {
            v |= (self.read8(addr.wrapping_add(k))? as u32) << (8 * k);
        }
        Ok(v)
    }

    /// Write the low `width` bytes (1, 2 or 4) of `value` little-endian.
    pub fn write(&mut self, addr: u32, value: u32, width: u32) -> Result<(), BusError> {
        for k in 0..width {
            self.write8(addr.wrapping_add(k), (value >> (8 * k)) as u8)?;
        }
        Ok(())
    }
}

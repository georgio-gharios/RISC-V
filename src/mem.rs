//! The bus: flat RAM plus memory-mapped devices. Little-endian.

use crate::devices::Uart;

pub const RAM_BASE: u32 = 0x8000_0000;

pub struct Bus {
    pub ram: Vec<u8>,
    pub uart: Uart,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BusError(pub u32);

impl Bus {
    pub fn new(ram_size: usize, echo_uart: bool) -> Self {
        Bus { ram: vec![0; ram_size], uart: Uart::new(echo_uart) }
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

    fn read8(&self, addr: u32) -> Result<u8, BusError> {
        if addr >= RAM_BASE {
            self.ram.get((addr - RAM_BASE) as usize).copied().ok_or(BusError(addr))
        } else if (Uart::BASE..Uart::BASE + Uart::SIZE).contains(&addr) {
            Ok(self.uart.read8(addr - Uart::BASE))
        } else {
            Err(BusError(addr))
        }
    }

    fn write8(&mut self, addr: u32, value: u8) -> Result<(), BusError> {
        if addr >= RAM_BASE {
            let slot = self.ram.get_mut((addr - RAM_BASE) as usize).ok_or(BusError(addr))?;
            *slot = value;
            Ok(())
        } else if (Uart::BASE..Uart::BASE + Uart::SIZE).contains(&addr) {
            self.uart.write8(addr - Uart::BASE, value);
            Ok(())
        } else {
            Err(BusError(addr))
        }
    }

    /// Read `width` bytes (1, 2 or 4) little-endian.
    pub fn read(&self, addr: u32, width: u32) -> Result<u32, BusError> {
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

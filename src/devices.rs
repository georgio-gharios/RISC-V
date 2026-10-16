//! Memory-mapped devices.

/// UART at 0x1000_0000. Writing a byte to offset 0 outputs that character.
/// Reading offset 5 (line status) returns 0x20 = "transmitter ready", like a 16550.
pub struct Uart {
    pub output: Vec<u8>,
    pub echo: bool, // also print to stdout
}

impl Uart {
    pub const BASE: u32 = 0x1000_0000;
    pub const SIZE: u32 = 0x100;

    pub fn new(echo: bool) -> Self { Uart { output: Vec::new(), echo } }

    pub fn read8(&self, offset: u32) -> u8 {
        if offset == 5 { 0x20 } else { 0 }
    }

    pub fn write8(&mut self, offset: u32, value: u8) {
        if offset == 0 {
            self.output.push(value);
            if self.echo {
                use std::io::Write;
                let mut out = std::io::stdout();
                let _ = out.write_all(&[value]);
                let _ = out.flush();
            }
        }
    }
}

/// Millisecond timer at 0x1100_0000. The host sets `ms` (wall clock in the CLI,
/// exact values in tests); the guest reads it as a u32.
pub struct Timer {
    pub ms: u32,
}

impl Timer {
    pub const BASE: u32 = 0x1100_0000;
    pub const SIZE: u32 = 4;
    pub fn read8(&self, offset: u32) -> u8 { (self.ms >> (8 * offset)) as u8 }
}

/// Keyboard at 0x1200_0000. Reading the 32-bit word at +0 pops one event:
/// 0 = no event, otherwise (pressed << 8) | keycode.
pub struct Keyboard {
    pub queue: std::collections::VecDeque<u32>,
    current: u32, // event being read byte by byte
}

impl Keyboard {
    pub const BASE: u32 = 0x1200_0000;
    pub const SIZE: u32 = 4;

    pub fn new() -> Self { Keyboard { queue: Default::default(), current: 0 } }

    pub fn push(&mut self, keycode: u8, pressed: bool) {
        self.queue.push_back(((pressed as u32) << 8) | keycode as u32);
    }

    /// Byte 0 starts a read and pops the event; bytes 1-3 return the rest of it.
    pub fn read8(&mut self, offset: u32) -> u8 {
        if offset == 0 {
            self.current = self.queue.pop_front().unwrap_or(0);
        }
        (self.current >> (8 * offset)) as u8
    }
}

/// 320x200 framebuffer at 0x2000_0000 (32-bit 0x00RRGGBB pixels), plus the display
/// control register at 0x1300_0000 that counts finished frames.
pub struct Framebuffer {
    pub bytes: Vec<u8>,
    pub frames: u32,
}

impl Framebuffer {
    pub const WIDTH: usize = 320;
    pub const HEIGHT: usize = 200;
    pub const BASE: u32 = 0x2000_0000;
    pub const SIZE: u32 = (Self::WIDTH * Self::HEIGHT * 4) as u32;
    pub const CTRL: u32 = 0x1300_0000;

    pub fn new() -> Self { Framebuffer { bytes: vec![0; Self::SIZE as usize], frames: 0 } }

    pub fn pixel(&self, x: usize, y: usize) -> u32 {
        let i = 4 * (y * Self::WIDTH + x);
        u32::from_le_bytes(self.bytes[i..i + 4].try_into().unwrap()) & 0x00ff_ffff
    }

    /// Pixels as RGB bytes, row-major (for PNG dumps and the browser canvas).
    pub fn rgb(&self) -> Vec<u8> {
        self.bytes.chunks(4).flat_map(|p| [p[2], p[1], p[0]]).collect()
    }
}

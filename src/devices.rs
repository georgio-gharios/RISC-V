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

//! WASM bindings for the browser front end: plain `extern "C"` exports, no wasm-bindgen.
//! JS passes data through linear memory (see web/www/main.js).
use rv_doom::cpu::{Cpu, Halt};
use rv_doom::devices::Framebuffer as Fb;
use std::cell::RefCell;

struct Web {
    cpu: Option<Cpu>,
    halt: Option<Halt>,
    upload: Vec<u8>,                    // program image written by JS
    rgba: Vec<u8>,                      // framebuffer converted for <canvas>
    regs: [u32; 33],                    // x0..x31, pc
    text: String,                       // last string returned to JS
}

thread_local! {
    static WEB: RefCell<Web> = RefCell::new(Web {
        cpu: None, halt: None, upload: Vec::new(),
        rgba: vec![0; Fb::WIDTH * Fb::HEIGHT * 4], regs: [0; 33], text: String::new(),
    });
}

fn with<R>(f: impl FnOnce(&mut Web) -> R) -> R { WEB.with(|w| f(&mut w.borrow_mut())) }

/// Reserve `len` bytes for a program image; JS copies the file there, then calls `load`.
#[no_mangle]
pub extern "C" fn upload_buffer(len: usize) -> *mut u8 {
    with(|w| { w.upload = vec![0; len]; w.upload.as_mut_ptr() })
}

/// Start a fresh machine with the uploaded image. Returns 0 on success.
#[no_mangle]
pub extern "C" fn load() -> i32 {
    with(|w| match rv_doom::machine(&w.upload, rv_doom::DEFAULT_RAM, false) {
        Ok(cpu) => { w.cpu = Some(cpu); w.halt = None; 0 }
        Err(_) => -1,
    })
}

/// Run up to `steps` instructions with the timer at `now_ms`.
/// Returns 0 = still running, 1 = halted (see `status_text`).
#[no_mangle]
pub extern "C" fn run(steps: u32, now_ms: u32) -> i32 {
    with(|w| {
        let Some(cpu) = w.cpu.as_mut() else { return 1 };
        if w.halt.is_some() { return 1; }
        cpu.bus.timer.ms = now_ms;
        w.halt = cpu.run(steps as u64);
        w.halt.is_some() as i32
    })
}

#[no_mangle]
pub extern "C" fn key(code: u32, pressed: u32) {
    with(|w| if let Some(cpu) = w.cpu.as_mut() { cpu.bus.keyboard.push(code as u8, pressed != 0) })
}

/// Convert the framebuffer (0x00RRGGBB words) to RGBA bytes; returns a pointer to them.
#[no_mangle]
pub extern "C" fn frame_rgba() -> *const u8 {
    with(|w| {
        if let Some(cpu) = w.cpu.as_ref() {
            for (dst, src) in w.rgba.chunks_mut(4).zip(cpu.bus.fb.bytes.chunks(4)) {
                dst.copy_from_slice(&[src[2], src[1], src[0], 255]);
            }
        }
        w.rgba.as_ptr()
    })
}

/// Pointer to 33 u32s: x0..x31 then pc.
#[no_mangle]
pub extern "C" fn regs() -> *const u32 {
    with(|w| {
        if let Some(cpu) = w.cpu.as_ref() {
            w.regs[..32].copy_from_slice(&cpu.regs);
            w.regs[32] = cpu.pc;
        }
        w.regs.as_ptr()
    })
}

#[no_mangle]
pub extern "C" fn instret() -> f64 { with(|w| w.cpu.as_ref().map_or(0, |c| c.instret) as f64) }

#[no_mangle]
pub extern "C" fn frames() -> u32 { with(|w| w.cpu.as_ref().map_or(0, |c| c.bus.fb.frames)) }

/// Debug overlay: key events the guest has popped from the keyboard queue so far.
#[no_mangle]
pub extern "C" fn keys_delivered() -> f64 { with(|w| w.cpu.as_ref().map_or(0, |c| c.bus.keyboard.delivered) as f64) }

/// Debug overlay: the timer value the guest currently sees (ms).
#[no_mangle]
pub extern "C" fn timer_ms() -> u32 { with(|w| w.cpu.as_ref().map_or(0, |c| c.bus.timer.ms)) }

/// Debug overlay: read a guest word (e.g. Doom's `gametic`); 0 if unmapped.
#[no_mangle]
pub extern "C" fn peek(addr: u32) -> u32 {
    with(|w| w.cpu.as_mut().and_then(|c| c.bus.read(addr, 4).ok()).unwrap_or(0))
}

fn set_text(w: &mut Web, s: String) -> usize { w.text = s; w.text.len() }

/// Disassembly of the instruction at pc. Returns the length; read it at `text_ptr()`.
#[no_mangle]
pub extern "C" fn disasm_current() -> usize {
    with(|w| {
        let s = match w.cpu.as_mut() {
            Some(cpu) => {
                let pc = cpu.pc;
                match cpu.bus.read(pc, 4) {
                    Ok(word) => format!("{pc:08x}:  {word:08x}  {}", rv_doom::disasm::disasm(word, pc)),
                    Err(_) => format!("{pc:08x}:  <unmapped>"),
                }
            }
            None => String::new(),
        };
        set_text(w, s)
    })
}

/// UART output produced since the last call (drains it). Returns the length; read at `text_ptr()`.
#[no_mangle]
pub extern "C" fn take_uart() -> usize {
    with(|w| {
        let out = w.cpu.as_mut().map(|c| std::mem::take(&mut c.bus.uart.output)).unwrap_or_default();
        set_text(w, String::from_utf8_lossy(&out).into_owned())
    })
}

/// Why the machine stopped ("" while running). Returns the length; read at `text_ptr()`.
#[no_mangle]
pub extern "C" fn status_text() -> usize {
    with(|w| {
        let s = match w.halt {
            None => String::new(),
            Some(Halt::Exit(code)) => format!("exited with code {code}"),
            Some(h) => format!("{h:x?}"),
        };
        set_text(w, s)
    })
}

#[no_mangle]
pub extern "C" fn text_ptr() -> *const u8 { with(|w| w.text.as_ptr()) }

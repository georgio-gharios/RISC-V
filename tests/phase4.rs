//! Phase 4 devices, tested with C programs (programs/fbtest.c, programs/devtest.c).
use rv_doom::cpu::Halt;
use rv_doom::devices::Framebuffer as Fb;

fn load(name: &str) -> rv_doom::cpu::Cpu {
    let image = std::fs::read(format!("programs/{name}.bin")).expect("run `make -C programs`");
    rv_doom::machine(&image, rv_doom::DEFAULT_RAM, false).unwrap()
}

/// The pattern fbtest.c is supposed to draw, written independently from the C code.
fn expected_pixel(x: usize, y: usize) -> u32 {
    let (w, h) = (Fb::WIDTH, Fb::HEIGHT);
    if x == 0 || y == 0 || x == w - 1 || y == h - 1 { return 0xffffff; }
    if y >= 40 && x == y + 60 { return 0xff0000; }
    if y < 40 {
        return [0xffffff, 0xffff00, 0x00ffff, 0x00ff00, 0xff00ff, 0xff0000, 0x0000ff, 0x000000][x / 40];
    }
    let r = (x * 255 / 319) as u32;
    let g = (y * 255 / 199) as u32;
    (r << 16) | (g << 8) | 0x40
}

#[test]
fn framebuffer_matches_pattern() {
    let mut cpu = load("fbtest");
    assert_eq!(cpu.run(10_000_000), Some(Halt::Exit(0)));
    assert_eq!(cpu.bus.fb.frames, 1);
    assert_eq!(String::from_utf8_lossy(&cpu.bus.uart.output), "frames = 1\n");

    // A few literal spot checks, so the test doesn't only compare formula with formula.
    assert_eq!(cpu.bus.fb.pixel(0, 0), 0xffffff);      // border
    assert_eq!(cpu.bus.fb.pixel(60, 20), 0xffff00);    // bar 1: yellow
    assert_eq!(cpu.bus.fb.pixel(300, 20), 0x000000);   // bar 7: black
    assert_eq!(cpu.bus.fb.pixel(170, 100), 0x878040);  // gradient: r=135, g=128
    assert_eq!(cpu.bus.fb.pixel(150, 90), 0xff0000);   // diagonal

    let mut bad = 0;
    for y in 0..Fb::HEIGHT {
        for x in 0..Fb::WIDTH {
            if cpu.bus.fb.pixel(x, y) != expected_pixel(x, y) { bad += 1; }
        }
    }
    assert_eq!(bad, 0, "{bad} pixels differ");

    // PNG dump: decode with an independent decoder (png crate) and compare.
    let file = rv_doom::framebuffer_png(&cpu);
    std::fs::write(std::env::temp_dir().join("rv-doom-fbtest.png"), &file).unwrap();
    let decoder = png::Decoder::new(&file[..]);
    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).unwrap();
    assert_eq!((info.width, info.height), (320, 200));
    assert_eq!(info.color_type, png::ColorType::Rgb);
    for y in 0..Fb::HEIGHT {
        for x in 0..Fb::WIDTH {
            let i = 3 * (y * Fb::WIDTH + x);
            let rgb = (buf[i] as u32) << 16 | (buf[i + 1] as u32) << 8 | buf[i + 2] as u32;
            assert_eq!(rgb, expected_pixel(x, y), "png pixel ({x},{y})");
        }
    }
}

#[test]
fn timer_and_keyboard() {
    let mut cpu = load("devtest");
    cpu.bus.timer.ms = 1000;
    // Advance the clock 1 ms per 1000 instructions; inject keys once the program waits for them.
    let mut injected = false;
    let halt = loop {
        if let Some(h) = cpu.run(1000) { break h; }
        cpu.bus.timer.ms += 1;
        let out = String::from_utf8_lossy(&cpu.bus.uart.output).into_owned();
        if !injected && out.contains("waiting for keys") {
            cpu.bus.keyboard.push(0xad, true);   // KEY_UPARROW down
            cpu.bus.keyboard.push(0xad, false);  // and up
            cpu.bus.keyboard.push(13, true);     // enter down
            injected = true;
        }
        assert!(cpu.bus.timer.ms < 10_000, "timed out; output so far: {out}");
    };
    assert_eq!(halt, Halt::Exit(0));
    let out = String::from_utf8_lossy(&cpu.bus.uart.output);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "t0 = 1000");
    let waited: u32 = lines[1].strip_prefix("waited until ").unwrap().parse().unwrap();
    assert!((1100..1110).contains(&waited), "waited until {waited}");
    assert_eq!(&lines[2..], ["empty read = 0", "waiting for keys", "down 173", "up 173", "down 13"]);
}

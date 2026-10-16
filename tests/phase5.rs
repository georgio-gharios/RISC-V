//! Phase 5: Doom's first frame must be the title screen, pixel for pixel.
//! The reference image is decoded here, independently, from TITLEPIC + PLAYPAL in doom1.wad.
//! Needs doom/doom.bin (make -C doom) and the WAD, so it is ignored by default:
//!   cargo test --release -- --ignored
use rv_doom::devices::Framebuffer as Fb;

const WAD: &str = "/usr/share/games/doom/doom1.wad";

fn u32_at(b: &[u8], i: usize) -> usize { u32::from_le_bytes(b[i..i + 4].try_into().unwrap()) as usize }
fn u16_at(b: &[u8], i: usize) -> usize { u16::from_le_bytes(b[i..i + 2].try_into().unwrap()) as usize }

fn lump<'a>(wad: &'a [u8], name: &str) -> &'a [u8] {
    let (count, dir) = (u32_at(wad, 4), u32_at(wad, 8));
    for i in 0..count {
        let e = dir + 16 * i;
        let lump_name = wad[e + 8..e + 16].split(|&c| c == 0).next().unwrap();
        if lump_name == name.as_bytes() {
            return &wad[u32_at(wad, e)..u32_at(wad, e) + u32_at(wad, e + 4)];
        }
    }
    panic!("lump {name} not found");
}

/// Decode a Doom "patch" picture (column posts) into palette indices.
fn decode_patch(p: &[u8]) -> (usize, usize, Vec<Option<u8>>) {
    let (w, h) = (u16_at(p, 0), u16_at(p, 2));
    let mut px = vec![None; w * h];
    for x in 0..w {
        let mut i = u32_at(p, 8 + 4 * x);
        while p[i] != 0xff {
            let (top, len) = (p[i] as usize, p[i + 1] as usize);
            for k in 0..len { px[(top + k) * w + x] = Some(p[i + 3 + k]); }
            i += len + 4;
        }
    }
    (w, h, px)
}

/// Row 0 of Doom's gammatable (the default gamma level), read from doomgeneric's tables.c.
/// It is not the identity: small values are bumped up by one.
fn gamma_level0() -> Vec<u8> {
    let src = std::fs::read_to_string("third_party/doomgeneric/doomgeneric/tables.c").unwrap();
    let start = src.find("gammatable[5][256]").unwrap();
    let body = &src[start..];
    let row0 = &body[body.find('{').unwrap() + 1..];
    let row0 = &row0[row0.find('{').unwrap() + 1..row0.find('}').unwrap()];
    let vals: Vec<u8> = row0.split(',').filter_map(|v| v.trim().parse().ok()).collect();
    assert_eq!(vals.len(), 256);
    vals
}

#[test]
#[ignore]
fn first_frame_is_title_screen() {
    let image = std::fs::read("doom/doom.bin").expect("build with `make -C doom`");
    let wad = std::fs::read(WAD).expect("doom1.wad (apt-get install doom-wad-shareware)");

    let mut cpu = rv_doom::machine(&image, rv_doom::DEFAULT_RAM, false).unwrap();
    while cpu.bus.fb.frames == 0 {
        assert_eq!(cpu.run(1_000_000), None, "Doom halted; output:\n{}",
            String::from_utf8_lossy(&cpu.bus.uart.output));
        cpu.bus.timer.ms += 20; // deterministic clock
        assert!(cpu.instret < 2_000_000_000, "no frame after 2e9 instructions");
    }
    std::fs::write(std::env::temp_dir().join("rv-doom-frame1.png"), rv_doom::framebuffer_png(&cpu)).unwrap();

    let palette = &lump(&wad, "PLAYPAL")[..768];
    let gamma = gamma_level0();
    let (w, h, title) = decode_patch(lump(&wad, "TITLEPIC"));
    assert_eq!((w, h), (Fb::WIDTH, Fb::HEIGHT));
    let mut bad = 0;
    for y in 0..h {
        for x in 0..w {
            let idx = title[y * w + x].expect("TITLEPIC has no holes") as usize;
            let g = |v: u8| gamma[v as usize] as u32;
            let rgb = g(palette[3 * idx]) << 16 | g(palette[3 * idx + 1]) << 8 | g(palette[3 * idx + 2]);
            if cpu.bus.fb.pixel(x, y) != rgb {
                if bad < 8 { eprintln!("({x},{y}) idx {idx}: got {:06x} want {rgb:06x}", cpu.bus.fb.pixel(x, y)); }
                bad += 1;
            }
        }
    }
    assert_eq!(bad, 0, "{bad} of 64000 pixels differ from TITLEPIC");
}

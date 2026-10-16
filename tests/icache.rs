// The decoded-instruction cache must notice stores to code that has already run.
fn i_type(imm: u32, rs1: u32, f3: u32, rd: u32, op: u32) -> u32 { imm << 20 | rs1 << 15 | f3 << 12 | rd << 7 | op }

#[test]
fn store_to_executed_code_is_seen() {
    let addi_a0 = |v| i_type(v, 0, 0, 10, 0x13);
    let words: [u32; 13] = [
        0x0000_0297,                                 // 00: auipc t0, 0
        i_type(0x20, 5, 0, 1, 0x67),                 // 04: jalr ra, 0x20(t0)   first call: a0 = 1 (now cached)
        i_type(0x30, 5, 2, 6, 0x03),                 // 08: lw t1, 0x30(t0)
        1 << 25 | 6 << 20 | 5 << 15 | 2 << 12 | 0x23, // 0c: sw t1, 0x20(t0)    patch the routine
        i_type(0x20, 5, 0, 1, 0x67),                 // 10: jalr ra, 0x20(t0)   second call must see a0 = 2
        0x0000_0073,                                 // 14: ecall (exit a0)
        0, 0,                                        // 18, 1c
        addi_a0(1),                                  // 20: addi a0, zero, 1
        i_type(0, 1, 0, 0, 0x67),                    // 24: jalr zero, 0(ra)
        0, 0,                                        // 28, 2c
        addi_a0(2),                                  // 30: data: addi a0, zero, 2
    ];
    let image: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
    let mut cpu = rv_doom::machine(&image, 1 << 20, false).unwrap();
    assert_eq!(cpu.run(100), Some(rv_doom::cpu::Halt::Exit(2)));
}

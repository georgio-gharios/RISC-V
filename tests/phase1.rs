use rv_doom::cpu::Halt;

#[test]
fn hello_prints_via_uart() {
    let image = std::fs::read("programs/hello.bin").expect("run `make -C programs` first");
    let mut cpu = rv_doom::machine(&image, 1 << 20, false).unwrap();
    assert_eq!(cpu.run(10_000), Some(Halt::Exit(0)));
    assert_eq!(String::from_utf8_lossy(&cpu.bus.uart.output), "hello\n");
}

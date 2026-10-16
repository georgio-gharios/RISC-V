//! C programs: emulator UART output must equal the host-compiled reference (*.expected).
use rv_doom::cpu::Halt;

fn check(name: &str) {
    let image = std::fs::read(format!("programs/{name}.bin")).expect("run `make -C programs`");
    let expected = std::fs::read(format!("programs/{name}.expected")).unwrap();
    let mut cpu = rv_doom::machine(&image, rv_doom::DEFAULT_RAM, false).unwrap();
    assert_eq!(cpu.run(100_000_000), Some(Halt::Exit(0)), "{name} did not exit cleanly");
    assert_eq!(String::from_utf8_lossy(&cpu.bus.uart.output), String::from_utf8_lossy(&expected));
}

#[test] fn hello() { check("hello"); }
#[test] fn mandel() { check("mandel"); }
#[test] fn primes() { check("primes"); }

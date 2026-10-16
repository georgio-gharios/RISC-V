//! Run the official riscv-tests binaries and print a pass/fail table.
//! Usage: cargo run --release --bin riscv-tests [dir]   (default tests/riscv-bin)
use rv_doom::TestOutcome;

fn main() {
    let dir = std::env::args().nth(1).unwrap_or_else(|| "tests/riscv-bin".into());
    let results = rv_doom::run_riscv_test_dir(&dir).expect("reading test dir");
    let mut passed = 0;
    println!("{:<16} result", "test");
    println!("{:-<16} ------", "");
    for (name, outcome) in &results {
        let text = match outcome {
            TestOutcome::Pass => { passed += 1; "PASS".to_string() }
            TestOutcome::Fail { test_num } => format!("FAIL (test #{test_num})"),
            TestOutcome::Crash(why) => format!("CRASH {why}"),
        };
        println!("{name:<16} {text}");
    }
    println!("\n{passed}/{} passed", results.len());
    std::process::exit(if passed == results.len() { 0 } else { 1 });
}

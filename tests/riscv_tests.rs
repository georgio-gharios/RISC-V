#[test]
fn official_riscv_tests_pass() {
    let results = rv_doom::run_riscv_test_dir("tests/riscv-bin").expect("tests/riscv-bin missing");
    assert_eq!(results.len(), 50, "expected 42 rv32ui + 8 rv32um binaries");
    let failed: Vec<_> = results.iter().filter(|(_, o)| *o != rv_doom::TestOutcome::Pass).collect();
    assert!(failed.is_empty(), "failures: {failed:?}");
}

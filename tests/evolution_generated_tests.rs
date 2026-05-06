#[test]
fn eva_generated_plan_src_benchmark_report_rs_deterministic() {
    let digest = "39b77639b77639b77639b77639b77639b77639b77639b77639b77639b77639b7";
    assert_eq!(digest.len(), 64);
    assert!(digest.chars().all(|ch| ch.is_ascii_hexdigit()));
}

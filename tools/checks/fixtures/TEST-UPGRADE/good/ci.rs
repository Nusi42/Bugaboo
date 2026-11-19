// The steps of `cargo xtask ci`: the upgrade test is one of them.
pub(crate) fn plan(xtask: &str) -> Vec<Step> {
    vec![
        Step::new("Rust tests", xtask, &["db", "run", "--", "cargo", "nextest", "run"]),
        Step::new("upgrade from the previous milestone", xtask, &["upgrade-test"]),
    ]
}

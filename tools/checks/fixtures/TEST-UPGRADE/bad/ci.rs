// The steps of `cargo xtask ci`: the upgrade test is missing ("upgrade-test" in a comment does not count).
pub(crate) fn plan(xtask: &str) -> Vec<Step> {
    vec![Step::new("Rust tests", xtask, &["db", "run", "--", "cargo", "nextest", "run"])]
}

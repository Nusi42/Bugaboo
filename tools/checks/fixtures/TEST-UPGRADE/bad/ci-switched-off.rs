// The steps of `cargo xtask ci`: the upgrade test is listed, but only on Linux, so it is switched off elsewhere.
pub(crate) fn plan(xtask: &str) -> Vec<Step> {
    vec![
        Step::new("Rust tests", xtask, &["db", "run", "--", "cargo", "nextest", "run"]),
        linux_only(Step::new("upgrade from the previous milestone", xtask, &["upgrade-test"])),
    ]
}

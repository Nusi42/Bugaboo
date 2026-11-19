// The steps of `cargo xtask ci`: the upgrade test is listed, but a call chained to it runs it only on Linux.
pub(crate) fn plan(root: &Path, xtask: &str) -> Vec<Step> {
    vec![
        Step::new("Rust tests", xtask, &["db", "run", "--", "cargo", "nextest", "run"]),
        needs_browser_app(root, Step::new("upgrade from the previous milestone", xtask, &["upgrade-test"]))
            .only_on("linux"),
    ]
}

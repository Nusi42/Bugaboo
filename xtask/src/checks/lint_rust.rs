//! LINT-RUST: Clippy finds no warnings (coding-standards §3).

use std::path::Path;

use super::{Check, Context, Finding};
use crate::tools;

pub(crate) struct LintRust;

impl Check for LintRust {
    fn id(&self) -> &'static str {
        "LINT-RUST"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        check_root(context.repo.root())
    }
}

/// The lint levels live in `[workspace.lints]`; `-D warnings` turns every remaining warning,
/// including compiler warnings, into an error.
fn check_root(root: &Path) -> anyhow::Result<Vec<Finding>> {
    let output = tools::run(
        Path::new("cargo"),
        &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ],
        root,
    )?;
    Ok(tools::findings_from(
        Ok(output),
        "Clippy found problems. Fix them in the code (do not silence them).",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;

    const MANIFEST: &str =
        "[package]\nname = \"demo\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[workspace]\n";

    #[test]
    fn bad_example() {
        let temp = TempDir::new("lint-rust-bad");
        temp.write("Cargo.toml", MANIFEST);
        temp.write("src/main.rs", "fn main() {\n    let unused = 1;\n}\n");
        assert!(!check_root(temp.path()).unwrap().is_empty());
    }

    #[test]
    fn good_example() {
        let temp = TempDir::new("lint-rust-good");
        temp.write("Cargo.toml", MANIFEST);
        temp.write("src/main.rs", "fn main() {}\n");
        assert!(check_root(temp.path()).unwrap().is_empty());
    }
}

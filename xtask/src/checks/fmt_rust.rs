//! FMT-RUST: Rust code is formatted (coding-standards §3, `rustfmt.toml`).

use std::path::Path;

use super::{Check, Context, Finding};
use crate::tools;

pub(crate) struct FmtRust;

impl Check for FmtRust {
    fn id(&self) -> &'static str {
        "FMT-RUST"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        check_root(context.repo.root())
    }
}

/// `cargo fmt --all --check` always looks at the whole workspace: it is fast and a file's
/// formatting never depends on the others.
fn check_root(root: &Path) -> anyhow::Result<Vec<Finding>> {
    let output = tools::run(Path::new("cargo"), &["fmt", "--all", "--check"], root)?;
    Ok(tools::findings_from(
        Ok(output),
        "Rust code is not formatted. Run `cargo fmt --all`.",
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
        let temp = TempDir::new("fmt-rust-bad");
        temp.write("Cargo.toml", MANIFEST);
        temp.write("src/main.rs", "fn main(){let x=1;let _=x;}\n");
        assert!(!check_root(temp.path()).unwrap().is_empty());
    }

    #[test]
    fn good_example() {
        let temp = TempDir::new("fmt-rust-good");
        temp.write("Cargo.toml", MANIFEST);
        temp.write("src/main.rs", "fn main() {}\n");
        assert!(check_root(temp.path()).unwrap().is_empty());
    }
}

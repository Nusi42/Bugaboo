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
        if context.files.is_none() {
            return check_root(context.repo.root());
        }
        check_files(context.repo.root(), &context.selected(self.id())?)
    }
}

/// What to do when a file is not formatted.
const UNFORMATTED: &str = "Rust code is not formatted. Run `cargo fmt --all`.";

/// `cargo fmt --all --check` looks at the whole workspace: for a run over the whole repository.
fn check_root(root: &Path) -> anyhow::Result<Vec<Finding>> {
    let output = tools::run(Path::new("cargo"), &["fmt", "--all", "--check"], root)?;
    Ok(tools::findings_from(Ok(output), UNFORMATTED))
}

/// rustfmt on the files that changed (and the modules they declare), with the settings of
/// `rustfmt.toml`: a file's formatting never depends on the others, and the whole workspace took
/// more than the 3 s an edit-time check may take (META-SPEED).
fn check_files(root: &Path, files: &[String]) -> anyhow::Result<Vec<Finding>> {
    if files.is_empty() {
        return Ok(Vec::new());
    }
    let mut args = vec!["--check"];
    args.extend(files.iter().map(String::as_str));
    let output = tools::run(Path::new("rustfmt"), &args, root)?;
    Ok(tools::findings_from(Ok(output), UNFORMATTED))
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
    fn bad_example_of_one_file() {
        let temp = TempDir::new("fmt-rust-file-bad");
        temp.write("src/main.rs", "fn main(){let x=1;let _=x;}\n");
        let files = ["src/main.rs".to_owned()];
        assert!(!check_files(temp.path(), &files).unwrap().is_empty());
        // No file of Rust among the changed ones: nothing to look at.
        assert!(check_files(temp.path(), &[]).unwrap().is_empty());
    }

    #[test]
    fn good_example() {
        let temp = TempDir::new("fmt-rust-good");
        temp.write("Cargo.toml", MANIFEST);
        temp.write("src/main.rs", "fn main() {}\n");
        assert!(check_root(temp.path()).unwrap().is_empty());
    }
}

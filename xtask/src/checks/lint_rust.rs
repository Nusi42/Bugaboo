//! LINT-RUST: Clippy finds no warnings (coding-standards §3).

use std::path::Path;

use super::{Check, Context, Finding};
use crate::metadata;
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
///
/// Clippy runs once per native target listed in `rust-toolchain.toml` (the single source), so
/// code behind `#[cfg(unix)]` or `#[cfg(windows)]` is checked on every development machine.
/// Checking needs no linker, so a Windows machine can check the Linux code and vice versa.
fn check_root(root: &Path) -> anyhow::Result<Vec<Finding>> {
    let mut findings = Vec::new();
    for target in native_targets(root)? {
        let mut args = vec![
            "clippy".to_owned(),
            "--workspace".to_owned(),
            "--all-targets".to_owned(),
        ];
        if let Some(target) = &target {
            args.extend(["--target".to_owned(), target.clone()]);
            if target != &host_target(root)? {
                for package in needing_a_c_compiler(root, target)? {
                    args.extend(["--exclude".to_owned(), package]);
                }
            }
        }
        args.extend(["--".to_owned(), "-D".to_owned(), "warnings".to_owned()]);
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        let output = tools::run(Path::new("cargo"), &args, root)?;
        let message = match &target {
            Some(target) => format!(
                "Clippy found problems when checking for {target}. Fix them in the code (do not silence them)."
            ),
            None => "Clippy found problems. Fix them in the code (do not silence them).".to_owned(),
        };
        findings.extend(tools::findings_from(Ok(output), &message));
    }
    Ok(findings)
}

/// The target this machine builds for natively.
fn host_target(root: &Path) -> anyhow::Result<String> {
    let output = tools::run(Path::new("rustc"), &["-vV"], root)?;
    Ok(output
        .text
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .unwrap_or_default()
        .trim()
        .to_owned())
}

/// The workspace members that compile C code in a build script (`ring`, SQLite, ...) when built
/// for `target`. Another system's C compiler is rarely installed, so those members are left out
/// of the check for a target other than the native one; the native check covers them completely
/// (and runs on Linux and Windows machines alike).
fn needing_a_c_compiler(root: &Path, target: &str) -> anyhow::Result<Vec<String>> {
    let mut found = Vec::new();
    for package in metadata::load(root)? {
        let tree = tools::run(
            Path::new("cargo"),
            &[
                "tree",
                "--package",
                &package.name,
                "--target",
                target,
                "--edges",
                "normal,build,dev",
                "--prefix",
                "none",
            ],
            root,
        )?;
        if uses_the_cc_crate(&tree.text) {
            found.push(package.name);
        }
    }
    Ok(found)
}

/// Whether `cargo tree --prefix none` output lists the `cc` crate, the C compiler driver.
fn uses_the_cc_crate(tree: &str) -> bool {
    tree.lines().any(|line| line.starts_with("cc v"))
}

/// Native targets from `rust-toolchain.toml` (WebAssembly is ARCH-WASM's job). Without that file
/// (for example in test projects), Clippy checks the host target only.
fn native_targets(root: &Path) -> anyhow::Result<Vec<Option<String>>> {
    let path = root.join("rust-toolchain.toml");
    if !path.exists() {
        return Ok(vec![None]);
    }
    let text = std::fs::read_to_string(&path)?;
    let value: toml::Value = toml::from_str(&text)?;
    let targets: Vec<Option<String>> = value
        .get("toolchain")
        .and_then(|toolchain| toolchain.get("targets"))
        .and_then(toml::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(toml::Value::as_str)
        .filter(|target| !target.starts_with("wasm"))
        .map(|target| Some(target.to_owned()))
        .collect();
    Ok(if targets.is_empty() {
        vec![None]
    } else {
        targets
    })
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
    fn targets_come_from_the_toolchain_file() {
        let temp = TempDir::new("lint-rust-targets");
        temp.write(
            "rust-toolchain.toml",
            "[toolchain]
channel = \"1.97.1\"
targets = [\"wasm32-unknown-unknown\", \"x86_64-unknown-linux-gnu\"]
",
        );
        assert_eq!(
            native_targets(temp.path()).unwrap(),
            [Some("x86_64-unknown-linux-gnu".to_owned())]
        );
    }

    #[test]
    fn crates_with_the_cc_crate_are_found_in_the_tree() {
        assert!(uses_the_cc_crate(
            "bb-testing v0.0.0 (/x)
ring v0.17.14
cc v1.6.0
"
        ));
        assert!(!uses_the_cc_crate(
            "bb-core v0.0.0 (/x)
serde v1.0.229
"
        ));
    }

    #[test]
    fn good_example() {
        let temp = TempDir::new("lint-rust-good");
        temp.write("Cargo.toml", MANIFEST);
        temp.write("src/main.rs", "fn main() {}\n");
        assert!(check_root(temp.path()).unwrap().is_empty());
    }
}

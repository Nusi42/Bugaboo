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
///
/// Clippy runs once per native target listed in `rust-toolchain.toml` (the single source), so
/// code behind `#[cfg(unix)]` or `#[cfg(windows)]` is checked on every development machine.
/// Checking needs no linker, so a Windows machine can check the Linux code and vice versa.
fn check_root(root: &Path) -> anyhow::Result<Vec<Finding>> {
    let mut findings = Vec::new();
    for target in native_targets(root)? {
        let mut args = vec!["clippy", "--workspace", "--all-targets"];
        if let Some(target) = &target {
            args.extend(["--target", target.as_str()]);
        }
        args.extend(["--", "-D", "warnings"]);
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
    fn good_example() {
        let temp = TempDir::new("lint-rust-good");
        temp.write("Cargo.toml", MANIFEST);
        temp.write("src/main.rs", "fn main() {}\n");
        assert!(check_root(temp.path()).unwrap().is_empty());
    }
}

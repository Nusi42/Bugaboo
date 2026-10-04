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
        let mut config = None;
        if let Some(target) = &target {
            args.extend(["--target", target.as_str()]);
            if target != &host_target(root)? {
                config = skip_c_build(root, target);
            }
        }
        if let Some(config) = &config {
            args.extend(["--config", config.as_str()]);
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

/// The target of this machine, from `rustc -vV`.
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

/// `ring` (the TLS crypto, ADR-0002) compiles C and assembly for its target in a build script.
/// Linting never links, so for a target other than this machine that step is switched off for
/// this run only (a Cargo `links` override): no C compiler for the other system is needed. Real
/// builds are not touched. Returns the `--config` value, or `None` when `ring` is not used.
fn skip_c_build(root: &Path, target: &str) -> Option<String> {
    let lock: toml::Value =
        toml::from_str(&std::fs::read_to_string(root.join("Cargo.lock")).ok()?).ok()?;
    let version = lock
        .get("package")?
        .as_array()?
        .iter()
        .find(|package| package.get("name").and_then(toml::Value::as_str) == Some("ring"))?
        .get("version")?
        .as_str()?;
    // The name ring gives its native library in its `Cargo.toml` (`links`).
    let links = format!("ring_core_{}_", version.replace('.', "_"));
    Some(format!("target.{target}.{links}.rustc-link-lib=[]"))
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
    fn the_c_build_of_ring_is_skipped_by_name_and_version() {
        let temp = TempDir::new("lint-rust-ring");
        assert_eq!(skip_c_build(temp.path(), "x86_64-unknown-linux-gnu"), None);
        temp.write(
            "Cargo.lock",
            "[[package]]
name = \"ring\"
version = \"0.17.14\"
",
        );
        assert_eq!(
            skip_c_build(temp.path(), "x86_64-unknown-linux-gnu").as_deref(),
            Some("target.x86_64-unknown-linux-gnu.ring_core_0_17_14_.rustc-link-lib=[]")
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

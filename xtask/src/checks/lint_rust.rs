//! LINT-RUST: Clippy finds no warnings (coding-standards §3).

use std::fmt::Write as _;
use std::path::Path;

use anyhow::Context as _;

use super::{Check, Context, Finding};
use crate::fakecc;
use crate::output;
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
    let host = host_target(root)?;
    for target in native_targets(root)? {
        let mut args = vec![
            "clippy".to_owned(),
            "--workspace".to_owned(),
            "--all-targets".to_owned(),
        ];
        let mut skipped = Vec::new();
        let mut env: Vec<(String, String)> = Vec::new();
        if let Some(target) = &target {
            args.extend(["--target".to_owned(), target.clone()]);
            if target != &host {
                skipped = c_build_packages(root, target)?;
                if !skipped.is_empty() {
                    env = stand_in_compiler(target)?;
                }
            }
        }
        args.extend(["--".to_owned(), "-D".to_owned(), "warnings".to_owned()]);
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        let env: Vec<(&str, &str)> = env.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        let output = tools::run_with_env(Path::new("cargo"), &args, root, &env)?;
        let mut message = match &target {
            Some(target) => format!(
                "Clippy found problems when checking for {target}. Fix them in the code (do not silence them)."
            ),
            None => "Clippy found problems. Fix them in the code (do not silence them).".to_owned(),
        };
        if !skipped.is_empty() {
            let names: Vec<String> = skipped.iter().map(|p| p.name.clone()).collect();
            let _ = write!(
                message,
                " (C compiler replaced by a no-op for this lint run: {}.)",
                names.join(", ")
            );
        }
        findings.extend(tools::findings_from(Ok(output), &message));
        if let (true, Some(target)) = (!skipped.is_empty(), &target) {
            output::info(&format!(
                "        {target}: C compiler replaced by a no-op for {}",
                skipped
                    .iter()
                    .map(|p| p.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
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

/// A package whose build script compiles C (`ring`, SQLite, ...).
#[derive(Debug, PartialEq, Eq)]
struct CBuild {
    name: String,
    links: String,
}

/// The environment that makes `cc` call xtask itself, as a no-op compiler and archiver, for
/// `target` (`CC_x86_64_unknown_linux_gnu` and friends).
fn stand_in_compiler(target: &str) -> anyhow::Result<Vec<(String, String)>> {
    let me = std::env::current_exe().context("could not find the running xtask program")?;
    let me = me.to_string_lossy().into_owned();
    anyhow::ensure!(
        !me.contains(char::is_whitespace),
        "The path of xtask contains a space ({me}); the lint run for another system cannot use it as a compiler."
    );
    let suffix = target.replace('-', "_");
    Ok(vec![
        (format!("CC_{suffix}"), me.clone()),
        (format!("AR_{suffix}"), me),
        (fakecc::VARIABLE.to_owned(), "1".to_owned()),
    ])
}

/// Linting never links, so for a target other than this machine the C build step of every
/// package in the target's dependency graph that has a `links` key and a build script using the
/// `cc` crate is switched off for this run only (a Cargo `links` override): no C compiler for the
/// other system is needed, and every workspace crate is still linted. Real builds are untouched.
fn c_build_packages(root: &Path, target: &str) -> anyhow::Result<Vec<CBuild>> {
    let output = std::process::Command::new("cargo")
        .args([
            "metadata",
            "--format-version",
            "1",
            "--filter-platform",
            target,
        ])
        .current_dir(root)
        .output()
        .context("could not run cargo metadata")?;
    anyhow::ensure!(
        output.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
    c_builds_in(&String::from_utf8_lossy(&output.stdout))
}

fn c_builds_in(metadata: &str) -> anyhow::Result<Vec<CBuild>> {
    let metadata: serde_json::Value = serde_json::from_str(metadata)?;
    let mut found = Vec::new();
    for package in metadata["packages"].as_array().into_iter().flatten() {
        let Some(links) = package["links"].as_str() else {
            continue;
        };
        let has_build_script = package["targets"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|t| {
                t["kind"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .any(|kind| kind == "custom-build")
            });
        let uses_cc = package["dependencies"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|d| d["name"] == "cc" && d["kind"] == "build");
        if has_build_script && uses_cc {
            found.push(CBuild {
                name: package["name"].as_str().unwrap_or_default().to_owned(),
                links: links.to_owned(),
            });
        }
    }
    found.sort_by(|a, b| a.links.cmp(&b.links));
    found.dedup_by(|a, b| a.links == b.links);
    Ok(found)
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
    fn packages_with_links_and_a_cc_build_script_are_overridden() {
        let metadata = r#"{"packages":[
            {"name":"ring","links":"ring_core_0_17_14_",
             "targets":[{"kind":["custom-build"]},{"kind":["lib"]}],
             "dependencies":[{"name":"cc","kind":"build"}]},
            {"name":"libsqlite3-sys","links":"sqlite3",
             "targets":[{"kind":["custom-build"]}],
             "dependencies":[{"name":"cc","kind":"build"}]},
            {"name":"pure","links":"pure","targets":[{"kind":["custom-build"]}],"dependencies":[]},
            {"name":"nolinks","links":null,"targets":[{"kind":["lib"]}],
             "dependencies":[{"name":"cc","kind":"build"}]}]}"#;
        let found = c_builds_in(metadata).unwrap();
        let names: Vec<&str> = found.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["ring", "libsqlite3-sys"]);
    }

    #[test]
    fn good_example() {
        let temp = TempDir::new("lint-rust-good");
        temp.write("Cargo.toml", MANIFEST);
        temp.write("src/main.rs", "fn main() {}\n");
        assert!(check_root(temp.path()).unwrap().is_empty());
    }
}

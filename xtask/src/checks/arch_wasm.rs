//! ARCH-WASM (minimal form): the crates shared with the browser still build for WebAssembly
//! (ADR-0001, ADR-0005, ADR-0011).
//!
//! `bb-core`, `bb-searchlang` and `bb-markdown` do no I/O, so a clock, a random source or a file
//! read that sneaks in breaks the build for `wasm32-unknown-unknown`. This check builds them, and
//! `bb-wasm`, which is the module itself. It also holds the `wasm-bindgen` program of
//! `tools/dev/tools.toml` to the version of the `wasm-bindgen` library in `Cargo.lock`: the two
//! must be the same, or the glue does not fit the module. The size of the module (PERF-04) is
//! measured in the browser build by PERF-BUNDLE.

use std::path::Path;

use anyhow::Context as _;
use serde::Deserialize;

use super::{Check, Context, Finding};
use crate::tools::{self, TOOLS_PATH};
use crate::wasm::{CRATE as MODULE_CRATE, TARGET};

/// The crates that must build for the browser.
const CRATES: [&str; 4] = ["bb-core", "bb-searchlang", "bb-markdown", MODULE_CRATE];

/// The library of the bridge, and the tool that goes with it.
const LIBRARY: &str = "wasm-bindgen";
/// The lockfile.
const LOCK_PATH: &str = "Cargo.lock";

pub(crate) struct ArchWasm;

impl Check for ArchWasm {
    fn id(&self) -> &'static str {
        "ARCH-WASM"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let root = context.repo.root();
        let mut findings = versions_findings(
            &std::fs::read_to_string(root.join(TOOLS_PATH))
                .with_context(|| format!("could not read {TOOLS_PATH}"))?,
            &std::fs::read_to_string(root.join(LOCK_PATH))
                .with_context(|| format!("could not read {LOCK_PATH}"))?,
        )?;
        findings.extend(check_packages(root, &CRATES)?);
        Ok(findings)
    }
}

#[derive(Deserialize)]
struct Lock {
    package: Vec<LockedPackage>,
}

#[derive(Deserialize)]
struct LockedPackage {
    name: String,
    version: String,
}

/// The program that joins the module and its glue must be the version of the library.
fn versions_findings(tools_text: &str, lock_text: &str) -> anyhow::Result<Vec<Finding>> {
    let program = tools::parse(tools_text)?
        .into_iter()
        .find(|tool| tool.name == LIBRARY);
    let lock: Lock = toml::from_str(lock_text).context("Cargo.lock is not valid")?;
    let library = lock.package.iter().find(|package| package.name == LIBRARY);
    Ok(match (program, library) {
        (Some(program), Some(library)) if program.version != library.version => {
            vec![Finding::at(
                TOOLS_PATH,
                format!(
                    "The {LIBRARY} program is {} but the library in {LOCK_PATH} is {}. Make them the same (the program in {TOOLS_PATH}, the library in the root Cargo.toml), or the glue does not fit the module.",
                    program.version, library.version
                ),
            )]
        }
        (None, _) => vec![Finding::at(
            TOOLS_PATH,
            format!("There is no tool named `{LIBRARY}`; `cargo xtask wasm` needs it."),
        )],
        _ => Vec::new(),
    })
}

fn check_packages(root: &Path, packages: &[&str]) -> anyhow::Result<Vec<Finding>> {
    let mut args = vec!["check", "--target", TARGET];
    for package in packages {
        args.extend(["-p", package]);
    }
    let output = tools::run(Path::new("cargo"), &args, root)?;
    Ok(tools::findings_from(
        Ok(output),
        &format!(
            "{} must build for WebAssembly ({TARGET}) without I/O, a clock or system randomness. Pass time and random bytes in from the caller.",
            packages.join(", ")
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;

    const TOOL: &str = "[[tool]]
name = \"wasm-bindgen\"
kind = \"cargo\"
crate = \"wasm-bindgen-cli\"
version = \"0.2.9\"
";
    const LOCK: &str = "[[package]]
name = \"wasm-bindgen\"
version = \"0.2.9\"
";

    #[test]
    fn bad_example_program_and_library_differ() {
        let found = versions_findings(TOOL, &LOCK.replace("0.2.9", "0.2.10")).unwrap();
        assert_eq!(found.len(), 1);
        assert!(found[0].message.contains("0.2.10"));
    }

    #[test]
    fn bad_example_the_program_is_missing() {
        assert_eq!(versions_findings("tool = []", LOCK).unwrap().len(), 1);
    }

    #[test]
    fn good_example_program_and_library_agree() {
        assert!(versions_findings(TOOL, LOCK).unwrap().is_empty());
    }

    #[test]
    fn the_real_program_and_library_agree() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let read = |path: &str| std::fs::read_to_string(root.join(path)).unwrap();
        assert!(
            versions_findings(&read(TOOLS_PATH), &read(LOCK_PATH))
                .unwrap()
                .is_empty()
        );
    }

    const MANIFEST: &str =
        "[package]\nname = \"demo\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[workspace]\n";

    #[test]
    fn bad_example() {
        let temp = TempDir::new("wasm-bad");
        temp.write("Cargo.toml", MANIFEST);
        temp.write(
            "src/lib.rs",
            "use std::os::unix::fs::PermissionsExt as _;\n",
        );
        assert!(!check_packages(temp.path(), &["demo"]).unwrap().is_empty());
    }

    #[test]
    fn good_example() {
        let temp = TempDir::new("wasm-good");
        temp.write("Cargo.toml", MANIFEST);
        temp.write(
            "src/lib.rs",
            "pub fn add(a: u32, b: u32) -> u32 {\n    a + b\n}\n",
        );
        assert!(check_packages(temp.path(), &["demo"]).unwrap().is_empty());
    }
}

//! ARCH-WASM (minimal form): the crates shared with the browser still build for WebAssembly
//! (ADR-0001, ADR-0005, ADR-0011).
//!
//! `bb-core`, `bb-searchlang` and `bb-markdown` do no I/O, so a clock, a random source or a file
//! read that sneaks in breaks the build for `wasm32-unknown-unknown`. This check only builds them.
//! The size budget (PERF-04, `wasm-opt`) comes with `bb-wasm` and is not checked yet.

use std::path::Path;

use super::{Check, Context, Finding};
use crate::tools;

/// The crates that must build for the browser.
const CRATES: [&str; 3] = ["bb-core", "bb-searchlang", "bb-markdown"];

const TARGET: &str = "wasm32-unknown-unknown";

pub(crate) struct ArchWasm;

impl Check for ArchWasm {
    fn id(&self) -> &'static str {
        "ARCH-WASM"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        check_packages(context.repo.root(), &CRATES)
    }
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

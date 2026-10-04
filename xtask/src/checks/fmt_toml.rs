//! FMT-TOML: TOML files are formatted (coding-standards §2, `taplo.toml`).

use std::path::Path;

use super::{Check, Context, Finding};
use crate::tools;

pub(crate) struct FmtToml;

impl Check for FmtToml {
    fn id(&self) -> &'static str {
        "FMT-TOML"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        check_files(
            context.repo.root(),
            context.files_for(self.id())?.as_deref(),
        )
    }
}

fn check_files(root: &Path, files: Option<&[String]>) -> anyhow::Result<Vec<Finding>> {
    let mut args = vec![
        "--colors",
        "never",
        "fmt",
        "--check",
        "--config",
        "taplo.toml",
    ];
    match files {
        Some([]) => return Ok(Vec::new()),
        Some(files) => args.extend(files.iter().map(String::as_str)),
        None => {}
    }
    let output = tools::run_installed("taplo", &args, root)?.map(|mut output| {
        // Taplo logs its progress as INFO lines on the same stream as its findings.
        output.text = output
            .text
            .lines()
            .filter(|line| !line.trim_start().starts_with("INFO "))
            .collect::<Vec<_>>()
            .join("\n");
        output
    });
    Ok(tools::findings_from(
        output,
        "TOML files are not formatted. Run `.tools/bin/taplo fmt --config taplo.toml`.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;

    #[test]
    fn bad_example() {
        let temp = TempDir::new("fmt-toml-bad");
        temp.write(
            "taplo.toml",
            "# Test configuration.
",
        );
        temp.write("a.toml", "a   =   1\n[ t ]\nb=2\n");
        crate::testutil::assert_real_findings(&check_files(temp.path(), None).unwrap());
    }

    #[test]
    fn good_example() {
        let temp = TempDir::new("fmt-toml-good");
        temp.write(
            "taplo.toml",
            "# Test configuration.
",
        );
        temp.write("a.toml", "a = 1\n\n[t]\nb = 2\n");
        assert!(check_files(temp.path(), None).unwrap().is_empty());
    }
}

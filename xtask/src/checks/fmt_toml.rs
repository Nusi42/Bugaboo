//! FMT-TOML: TOML files are formatted (coding-standards §2, `taplo.toml`).

use super::{Check, Context, Finding};
use crate::tools::{FileTool, TOOLS_DIR};

pub(crate) struct FmtToml;

impl Check for FmtToml {
    fn id(&self) -> &'static str {
        "FMT-TOML"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        tool().check(
            context.repo.root(),
            context.files_for(self.id())?.as_deref(),
        )
    }
}

fn tool() -> FileTool {
    FileTool {
        program: "taplo",
        args: &[
            "--colors",
            "never",
            "fmt",
            "--check",
            "--config",
            "taplo.toml",
        ],
        whole: &[],
        fix: format!(
            "TOML files are not formatted. Run `{TOOLS_DIR}/bin/taplo fmt --config taplo.toml`."
        ),
        clean: Some(without_progress),
    }
}

/// Taplo logs its progress as INFO lines on the same stream as its findings.
fn without_progress(text: &str) -> String {
    text.lines()
        .filter(|line| !line.trim_start().starts_with("INFO "))
        .collect::<Vec<_>>()
        .join("\n")
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
        crate::testutil::assert_real_findings(&tool().check(temp.path(), None).unwrap());
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
        assert!(tool().check(temp.path(), None).unwrap().is_empty());
    }
}

//! FMT-MD: Markdown docs follow one style (coding-standards §2, `.rumdl.toml`).

use super::{Check, Context, Finding};
use crate::tools::{FileTool, TOOLS_DIR};

pub(crate) struct FmtMd;

impl Check for FmtMd {
    fn id(&self) -> &'static str {
        "FMT-MD"
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
        program: "rumdl",
        args: &["check", "--no-cache", "--config", ".rumdl.toml"],
        whole: &["."],
        fix: format!(
            "Markdown style problems. Fix them in the file (`{TOOLS_DIR}/bin/rumdl check --fix` fixes many)."
        ),
        clean: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;

    #[test]
    fn bad_example() {
        let temp = TempDir::new("fmt-md-bad");
        temp.write(".rumdl.toml", "");
        temp.write("a.md", "# Title\n### Skipped a level\n\ntext   \n");
        crate::testutil::assert_real_findings(&tool().check(temp.path(), None).unwrap());
    }

    #[test]
    fn good_example() {
        let temp = TempDir::new("fmt-md-good");
        temp.write(".rumdl.toml", "");
        temp.write("a.md", "# Title\n\nSome text.\n\n## Part\n\n- one\n- two\n");
        assert!(tool().check(temp.path(), None).unwrap().is_empty());
    }
}

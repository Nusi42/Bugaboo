//! FMT-MD: Markdown docs follow one style (coding-standards §2, `.rumdl.toml`).

use std::path::Path;

use super::{Check, Context, Finding};
use crate::tools::{self, TOOLS_DIR};

pub(crate) struct FmtMd;

impl Check for FmtMd {
    fn id(&self) -> &'static str {
        "FMT-MD"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        check_files(
            context.repo.root(),
            context.files_for(self.id())?.as_deref(),
        )
    }
}

fn check_files(root: &Path, files: Option<&[String]>) -> anyhow::Result<Vec<Finding>> {
    let mut args = vec!["check", "--no-cache", "--config", ".rumdl.toml"];
    match files {
        Some([]) => return Ok(Vec::new()),
        Some(files) => args.extend(files.iter().map(String::as_str)),
        None => args.push("."),
    }
    let output = tools::run_installed("rumdl", &args, root)?;
    Ok(tools::findings_from(
        output,
        &format!(
            "Markdown style problems. Fix them in the file (`{TOOLS_DIR}/bin/rumdl check --fix` fixes many)."
        ),
    ))
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
        crate::testutil::assert_real_findings(&check_files(temp.path(), None).unwrap());
    }

    #[test]
    fn good_example() {
        let temp = TempDir::new("fmt-md-good");
        temp.write(".rumdl.toml", "");
        temp.write("a.md", "# Title\n\nSome text.\n\n## Part\n\n- one\n- two\n");
        assert!(check_files(temp.path(), None).unwrap().is_empty());
    }
}

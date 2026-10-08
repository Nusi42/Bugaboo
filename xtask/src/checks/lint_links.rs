//! LINT-LINKS: links in the docs point somewhere real (coding-standards §2, `lychee.toml`).
//!
//! Offline: only links to files and headings inside the repository are checked, so the check is
//! fast and never fails because a website is down.

use super::{Check, Context, Finding};
use crate::tools::FileTool;

pub(crate) struct LintLinks;

impl Check for LintLinks {
    fn id(&self) -> &'static str {
        "LINT-LINKS"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        tool().check(context.repo.root(), Some(&context.selected(self.id())?))
    }
}

fn tool() -> FileTool {
    FileTool {
        program: "lychee",
        args: &["--offline", "--no-progress", "--config", "lychee.toml"],
        whole: &[],
        fix: "Broken links. Fix the path or the heading name in the link.".to_owned(),
        clean: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;

    fn bad(name: &str, text: &str) {
        let temp = TempDir::new(name);
        temp.write(
            "lychee.toml",
            "include_fragments = \"anchor-only\"
",
        );
        temp.write("a.md", text);
        temp.write(
            "b.md", "# B
",
        );
        crate::testutil::assert_real_findings(
            &tool().check(temp.path(), Some(&["a.md".into()])).unwrap(),
        );
    }

    #[test]
    fn bad_example_missing_file() {
        bad(
            "lint-links-bad-file",
            "# A

See [missing](missing.md).
",
        );
    }

    #[test]
    fn bad_example_missing_anchor() {
        bad(
            "lint-links-bad-anchor",
            "# A

See [part](b.md#nowhere).
",
        );
    }

    #[test]
    fn good_example() {
        let temp = TempDir::new("lint-links-good");
        temp.write("lychee.toml", "include_fragments = \"anchor-only\"\n");
        temp.write("a.md", "# A\n\nSee [b](b.md) and [part](b.md#b).\n");
        temp.write("b.md", "# B\n");
        assert!(
            tool()
                .check(temp.path(), Some(&["a.md".into()]))
                .unwrap()
                .is_empty()
        );
    }
}

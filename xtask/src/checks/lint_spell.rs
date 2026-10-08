//! LINT-SPELL: no spelling mistakes in code, UI text and docs (coding-standards §2, `typos.toml`).

use super::{Check, Context, Finding};
use crate::tools::FileTool;

pub(crate) struct LintSpell;

impl Check for LintSpell {
    fn id(&self) -> &'static str {
        "LINT-SPELL"
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
        program: "typos",
        // --force-exclude: the excludes in typos.toml also apply to files named explicitly.
        args: &[
            "--force-exclude",
            "--config",
            "typos.toml",
            "--format",
            "brief",
        ],
        whole: &[],
        fix: "Spelling mistakes. Fix the words; for a real word the tool does not know, add it to `typos.toml`.".to_owned(),
        clean: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;

    #[test]
    fn bad_example() {
        let temp = TempDir::new("lint-spell-bad");
        temp.write("typos.toml", "");
        // Assembled at run time so that this file itself has no misspelling.
        let wrong = ["rec", "ieve"].concat();
        temp.write("a.md", format!("This is a {wrong} mistake.\n"));
        crate::testutil::assert_real_findings(&tool().check(temp.path(), None).unwrap());
    }

    #[test]
    fn good_example() {
        let temp = TempDir::new("lint-spell-good");
        temp.write("typos.toml", "");
        temp.write("a.md", "This text is spelled correctly.\n");
        assert!(tool().check(temp.path(), None).unwrap().is_empty());
    }
}

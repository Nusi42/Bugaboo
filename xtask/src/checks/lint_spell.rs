//! LINT-SPELL: no spelling mistakes in code, UI text and docs (coding-standards §2, `typos.toml`).

use std::path::Path;

use super::{Check, Context, Finding};
use crate::tools;

pub(crate) struct LintSpell;

impl Check for LintSpell {
    fn id(&self) -> &'static str {
        "LINT-SPELL"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        check_files(
            context.repo.root(),
            context.files_for(self.id())?.as_deref(),
        )
    }
}

fn check_files(root: &Path, files: Option<&[String]>) -> anyhow::Result<Vec<Finding>> {
    let mut args = vec!["--config", "typos.toml", "--format", "brief"];
    match files {
        Some([]) => return Ok(Vec::new()),
        Some(files) => args.extend(files.iter().map(String::as_str)),
        None => {}
    }
    let output = tools::run_installed("typos", &args, root)?;
    Ok(tools::findings_from(
        output,
        "Spelling mistakes. Fix the words; for a real word the tool does not know, add it to `typos.toml`.",
    ))
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
        crate::testutil::assert_real_findings(&check_files(temp.path(), None).unwrap());
    }

    #[test]
    fn good_example() {
        let temp = TempDir::new("lint-spell-good");
        temp.write("typos.toml", "");
        temp.write("a.md", "This text is spelled correctly.\n");
        assert!(check_files(temp.path(), None).unwrap().is_empty());
    }
}

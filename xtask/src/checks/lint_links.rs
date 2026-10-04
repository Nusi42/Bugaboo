//! LINT-LINKS: links in the docs point somewhere real (coding-standards §2, `lychee.toml`).
//!
//! Offline: only links to files and headings inside the repository are checked, so the check is
//! fast and never fails because a website is down.

use std::path::Path;

use super::{Check, Context, Finding};
use crate::repo::Repo;
use crate::tools;

pub(crate) struct LintLinks;

impl Check for LintLinks {
    fn id(&self) -> &'static str {
        "LINT-LINKS"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let files = match context.files_for(self.id())? {
            Some(files) => files,
            None => markdown_files(context.repo)?,
        };
        check_files(context.repo.root(), &files)
    }
}

fn markdown_files(repo: &Repo) -> anyhow::Result<Vec<String>> {
    let mut files = repo.all_files()?;
    files.retain(|file| {
        Path::new(file)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
            && repo.path(file).is_file()
    });
    Ok(files)
}

fn check_files(root: &Path, files: &[String]) -> anyhow::Result<Vec<Finding>> {
    if files.is_empty() {
        return Ok(Vec::new());
    }
    let mut args = vec!["--offline", "--no-progress", "--config", "lychee.toml"];
    args.extend(files.iter().map(String::as_str));
    let output = tools::run_installed("lychee", &args, root)?;
    Ok(tools::findings_from(
        output,
        "Broken links. Fix the path or the heading name in the link.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;

    #[test]
    fn bad_example() {
        let temp = TempDir::new("lint-links-bad");
        temp.write("lychee.toml", "include_fragments = \"anchor-only\"\n");
        temp.write(
            "a.md",
            "# A\n\nSee [missing](missing.md) and [part](b.md#nowhere).\n",
        );
        temp.write("b.md", "# B\n");
        crate::testutil::assert_real_findings(&check_files(temp.path(), &["a.md".into()]).unwrap());
    }

    #[test]
    fn good_example() {
        let temp = TempDir::new("lint-links-good");
        temp.write("lychee.toml", "include_fragments = \"anchor-only\"\n");
        temp.write("a.md", "# A\n\nSee [b](b.md) and [part](b.md#b).\n");
        temp.write("b.md", "# B\n");
        assert!(
            check_files(temp.path(), &["a.md".into()])
                .unwrap()
                .is_empty()
        );
    }
}

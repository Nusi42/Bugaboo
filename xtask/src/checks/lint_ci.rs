//! LINT-CI: CI workflow files are valid (ADR-0014). Passes when there are no workflow files.

use std::path::Path;

use super::{Check, Context, Finding};
use crate::tools;

pub(crate) struct LintCi;

impl Check for LintCi {
    fn id(&self) -> &'static str {
        "LINT-CI"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let files = match context.files_for(self.id())? {
            Some(files) => files,
            None => context.repo.all_files()?,
        };
        check_files(context.repo.root(), &files)
    }
}

fn is_workflow(file: &str) -> bool {
    file.starts_with(".github/workflows/")
        && Path::new(file)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("yml") || ext.eq_ignore_ascii_case("yaml"))
}

fn check_files(root: &Path, files: &[String]) -> anyhow::Result<Vec<Finding>> {
    let workflows: Vec<&str> = files
        .iter()
        .map(String::as_str)
        .filter(|file| is_workflow(file) && root.join(file).is_file())
        .collect();
    if workflows.is_empty() {
        return Ok(Vec::new());
    }
    let mut args = vec!["-no-color"];
    args.extend(workflows);
    let output = tools::run_installed("actionlint", &args, root)?;
    Ok(tools::findings_from(
        output,
        "The workflow files have problems. Fix them as described below.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;

    const FILE: &str = ".github/workflows/ci.yml";

    #[test]
    fn bad_example() {
        let temp = TempDir::new("lint-ci-bad");
        temp.write(
            FILE,
            "on: push\njobs:\n  build:\n    steps:\n      - run: echo hi\n",
        );
        crate::testutil::assert_real_findings(&check_files(temp.path(), &[FILE.into()]).unwrap());
    }

    #[test]
    fn good_example() {
        let temp = TempDir::new("lint-ci-good");
        temp.write(
            FILE,
            "on: push\njobs:\n  build:\n    runs-on: ubuntu-latest\n    steps:\n      - run: echo hi\n",
        );
        assert!(check_files(temp.path(), &[FILE.into()]).unwrap().is_empty());
    }

    #[test]
    fn no_workflow_files_passes() {
        let temp = TempDir::new("lint-ci-none");
        assert!(
            check_files(temp.path(), &["README.md".into()])
                .unwrap()
                .is_empty()
        );
    }
}

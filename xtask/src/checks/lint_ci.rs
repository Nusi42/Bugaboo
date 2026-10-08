//! LINT-CI: CI workflow files are valid (ADR-0014). Passes when there are no workflow files.

use super::{Check, Context, Finding};
use crate::tools::FileTool;

pub(crate) struct LintCi;

impl Check for LintCi {
    fn id(&self) -> &'static str {
        "LINT-CI"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        tool().check(context.repo.root(), Some(&context.selected(self.id())?))
    }
}

fn tool() -> FileTool {
    FileTool {
        program: "actionlint",
        args: &["-no-color"],
        whole: &[],
        fix: "The workflow files have problems. Fix them as described below.".to_owned(),
        clean: None,
    }
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
        crate::testutil::assert_real_findings(
            &tool().check(temp.path(), Some(&[FILE.into()])).unwrap(),
        );
    }

    #[test]
    fn good_example() {
        let temp = TempDir::new("lint-ci-good");
        temp.write(
            FILE,
            "on: push\njobs:\n  build:\n    runs-on: ubuntu-latest\n    steps:\n      - run: echo hi\n",
        );
        assert!(
            tool()
                .check(temp.path(), Some(&[FILE.into()]))
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn no_workflow_files_passes() {
        let temp = TempDir::new("lint-ci-none");
        assert!(tool().check(temp.path(), Some(&[])).unwrap().is_empty());
    }
}

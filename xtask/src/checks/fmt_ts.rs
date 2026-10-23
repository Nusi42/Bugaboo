//! FMT-TS: TypeScript, JavaScript and JSON are formatted by oxfmt (coding-standards §4).

use super::node_tool::unformatted;
use super::{Check, Context, Finding};

pub(crate) struct FmtTs;

impl Check for FmtTs {
    fn id(&self) -> &'static str {
        "FMT-TS"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        unformatted(context.repo.root(), &context.selected(self.id())?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repo::Repo;

    fn findings(file: &str) -> Vec<Finding> {
        let repo = Repo::discover().unwrap();
        unformatted(
            repo.root(),
            &[format!("tools/checks/fixtures/FMT-TS/{file}")],
        )
        .unwrap()
    }

    #[test]
    fn bad_example() {
        assert!(!findings("bad/messy.ts").is_empty());
    }

    #[test]
    fn good_example() {
        assert!(findings("good/tidy.ts").is_empty());
    }

    #[test]
    fn good_example_files_that_are_all_on_the_ignore_list_are_no_error() {
        // The frozen copy of the API is on the ignore list of oxfmt; a commit that holds only it must
        // not fail with "expected at least one target file".
        let repo = Repo::discover().unwrap();
        let found = unformatted(repo.root(), &["tools/checks/api-frozen.json".to_owned()]).unwrap();
        assert!(
            found.is_empty(),
            "{:?}",
            found.first().map(|each| &each.message)
        );
    }
}

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
}

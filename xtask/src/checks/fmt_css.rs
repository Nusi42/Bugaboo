//! FMT-CSS: CSS is formatted by oxfmt, which covers CSS, so Biome is not needed (ADR-0014).

use super::node_tool::unformatted;
use super::{Check, Context, Finding};

pub(crate) struct FmtCss;

impl Check for FmtCss {
    fn id(&self) -> &'static str {
        "FMT-CSS"
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
            &[format!("tools/checks/fixtures/FMT-CSS/{file}")],
        )
        .unwrap()
    }

    #[test]
    fn bad_example() {
        assert!(!findings("bad/messy.css").is_empty());
    }

    #[test]
    fn good_example() {
        assert!(findings("good/tidy.css").is_empty());
    }
}

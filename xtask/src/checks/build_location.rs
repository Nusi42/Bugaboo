//! BUILD-LOCATION as a catalogue check, so hooks and CI report it like every other rule.

use super::{Check, Context, Finding};
use crate::location;

pub(crate) struct BuildLocation;

impl Check for BuildLocation {
    fn id(&self) -> &'static str {
        "BUILD-LOCATION"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let root = context.repo.root();
        Ok(location::check(root, &location::onedrive_roots())
            .map(Finding::general)
            .into_iter()
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use crate::location;

    #[test]
    fn bad_example() {
        let roots = [PathBuf::from(r"C:\Users\bo\OneDrive")];
        let message = location::check(Path::new(r"C:\Users\bo\OneDrive\code\bugaboo"), &roots);
        assert!(message.is_some());
    }

    #[test]
    fn good_example() {
        let roots = [PathBuf::from(r"C:\Users\bo\OneDrive")];
        let message = location::check(
            Path::new(r"C:\Users\bo\the work root\bugaboo.win4"),
            &roots,
        );
        assert!(message.is_none());
    }
}

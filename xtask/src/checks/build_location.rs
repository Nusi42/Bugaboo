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

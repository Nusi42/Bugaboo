//! BUILD-LOCATION as a catalogue check, so hooks and the full check report it like every other
//! rule.

use super::{Check, Context, Finding};
use crate::location;

pub(crate) struct BuildLocation;

impl Check for BuildLocation {
    fn id(&self) -> &'static str {
        "BUILD-LOCATION"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let root = context.repo.root();
        Ok(location::check(root, location::role(root))
            .map(Finding::general)
            .into_iter()
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use crate::location::{self, ENV_FILE, Role};
    use crate::testutil::TempDir;

    #[test]
    fn bad_example() {
        let temp = TempDir::new("build-location-bad");
        temp.write(ENV_FILE, "BUGABOO_CHECKOUT_ROLE=master\n");
        let role = location::role_of(temp.path(), |_| None);
        assert!(location::check(temp.path(), role).is_some());
    }

    #[test]
    fn good_example() {
        let temp = TempDir::new("build-location-good");
        let role = location::role_of(temp.path(), |_| None);
        assert_eq!(role, Role::Work);
        assert!(location::check(temp.path(), role).is_none());
    }
}

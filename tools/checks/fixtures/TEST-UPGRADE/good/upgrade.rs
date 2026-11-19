// The upgrade test asks Git for the previous milestone.
pub(crate) const MILESTONE_TAGS: &str = "m*-done";

pub(crate) fn previous_tag() -> String {
    git(&["describe", "--tags", "--abbrev=0", "--match", MILESTONE_TAGS, "HEAD^"])
}

//! The names of the members of an own field in its events, which *History* and the conflict
//! answers read: defined once, here, in the style of the field codes (kebab case).

pub(crate) const NAME: &str = "name";
pub(crate) const HELP: &str = "help";
pub(crate) const KIND: &str = "kind";
pub(crate) const APPLIES_TO: &str = "applies-to";
pub(crate) const RULES: &str = "rules";
pub(crate) const DEFAULT: &str = "default";
pub(crate) const LOOK: &str = "look";
pub(crate) const NEVER_TO_ASSISTANT: &str = "never-to-assistant";
pub(crate) const RETIRED: &str = "retired";
pub(crate) const PROJECTS: &str = "projects";
pub(crate) const ISSUE_TYPES: &str = "issue-types";
/// The choices a field was made with.
pub(crate) const CHOICES: &str = "choices";
/// The order of the choices.
pub(crate) const CHOICES_ORDER: &str = "choices-order";
/// The choice that was added (its name).
pub(crate) const CHOICE_ADDED: &str = "choice-added";
/// The choice that was changed (the member says which).
pub(crate) const CHOICE_CHANGED: &str = "choice-changed";
/// The choice that went into another one.
pub(crate) const CHOICE_COMBINED: &str = "choice-combined";

/// The names of the members of a choice that changed.
pub(crate) mod choice {
    pub(crate) const NAME: &str = "name";
    pub(crate) const COLOR: &str = "color";
    pub(crate) const ICON: &str = "icon";
    pub(crate) const RETIRED: &str = "retired";
}

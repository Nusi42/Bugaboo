//! The settings of *Remove a person* (PRIV-04, ADR-0044, decision 7): what the window offers first
//! for the comments of a removed person. They are part of the registry ([`super::registry`] lists
//! them).

use super::{ChangedBy, Choice, Format, Level, Setting, SettingDefinition, Text, settings};
use crate::message;

words! {
    /// What *Remove a person* does with the comments the person wrote (ADR-0044, decision 7).
    pub enum RemovedTexts {
        /// The comments stay, shown as *Former user*.
        Keep => "keep",
        /// The comments go, with their versions and events.
        Delete => "delete",
    }
}

crate::words_in_api!(
    RemovedTexts,
    "What Remove a person does with the comments the person wrote: keep them, shown as Former user, or delete them."
);

/// The choices of [`PRIVACY_REMOVED_TEXTS`].
const REMOVED_TEXTS_CHOICES: &[Choice] = &[
    Choice::new(
        RemovedTexts::Keep.code(),
        message::SETTING_PRIVACY_REMOVED_TEXTS_KEEP,
    ),
    Choice::new(
        RemovedTexts::Delete.code(),
        message::SETTING_PRIVACY_REMOVED_TEXTS_DELETE,
    ),
];

settings! {
    /// What the window of *Remove a person* offers first for the person's comments: keep them,
    /// shown as *Former user* (at the start), or delete them.
    PRIVACY_REMOVED_TEXTS: Text = {
        key: "privacy.removed-texts",
        name: message::SETTING_PRIVACY_REMOVED_TEXTS_NAME,
        default: RemovedTexts::Keep.code(),
        levels: [Organization],
        changed_by: Administrators,
        help: message::SETTING_PRIVACY_REMOVED_TEXTS_HELP,
        lockable: false,
        format: Format::Choices(REMOVED_TEXTS_CHOICES),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn priv_04_the_choices_of_the_texts_are_the_words_of_removed_texts() {
        let codes: Vec<&str> = REMOVED_TEXTS_CHOICES.iter().map(Choice::value).collect();
        let words: Vec<&str> = RemovedTexts::ALL.iter().map(|word| word.code()).collect();
        assert_eq!(codes, words);
    }
}

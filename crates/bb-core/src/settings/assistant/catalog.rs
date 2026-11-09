//! The look-ups of the assistant (AI-10, ADR-0013 amendment M7, WP-15): how many tools it may run to
//! answer one question. An administrator sets the most for the organization (with *No limit*), and
//! each person may choose a lower number for themselves, never a higher one. They stand in a file
//! of their own like the settings of agents; the numbers they hold are read through
//! [`crate::ai::LookUps`]. The file is called `catalog.rs` so that the keys, choices and defaults for
//! the browser are made from it too.

use super::super::{ChangedBy, Choice, Format, Level, Setting, SettingDefinition, Text, settings};
use crate::message;

/// The choices of the most for the organization. Written out like the other lists of choices (the
/// registry is read as text by the tools that make the constants of the browser) and held to the
/// words of `LookUps` by the test below.
const MOST_CHOICES: &[Choice] = &[
    Choice::new("2", message::SETTING_ASSISTANT_LOOK_UPS_2),
    Choice::new("4", message::SETTING_ASSISTANT_LOOK_UPS_4),
    Choice::new("8", message::SETTING_ASSISTANT_LOOK_UPS_8),
    Choice::new("12", message::SETTING_ASSISTANT_LOOK_UPS_12),
    Choice::new("20", message::SETTING_ASSISTANT_LOOK_UPS_20),
    Choice::new("50", message::SETTING_ASSISTANT_LOOK_UPS_50),
    Choice::new("no-limit", message::SETTING_ASSISTANT_LOOK_UPS_NO_LIMIT),
];

/// The choices of a person: the same numbers, and the first choice, which follows the organization.
const OWN_CHOICES: &[Choice] = &[
    Choice::new(
        "as-organization",
        message::SETTING_ASSISTANT_LOOK_UPS_AS_ORGANIZATION,
    ),
    Choice::new("2", message::SETTING_ASSISTANT_LOOK_UPS_2),
    Choice::new("4", message::SETTING_ASSISTANT_LOOK_UPS_4),
    Choice::new("8", message::SETTING_ASSISTANT_LOOK_UPS_8),
    Choice::new("12", message::SETTING_ASSISTANT_LOOK_UPS_12),
    Choice::new("20", message::SETTING_ASSISTANT_LOOK_UPS_20),
    Choice::new("50", message::SETTING_ASSISTANT_LOOK_UPS_50),
];

/// The word of a person who chose no number of their own (written out in the choices above, which the
/// tools that make the constants of the browser read as text, and held to this word here).
pub const AS_ORGANIZATION: &str = "as-organization";

const _: () = assert!(crate::words::same_text(AS_ORGANIZATION, "as-organization"));

settings! {
    /// The most tools the assistant may run to answer one question, for everybody: 8 at the start.
    ASSISTANT_LOOK_UPS_MOST: Text = {
        key: "assistant.look-ups-most",
        name: message::SETTING_ASSISTANT_LOOK_UPS_MOST_NAME,
        default: "8",
        levels: [Organization],
        changed_by: Administrators,
        help: message::SETTING_ASSISTANT_LOOK_UPS_MOST_HELP,
        lockable: false,
        format: Format::Choices(MOST_CHOICES),
    }

    /// The number a person chooses for their own questions: it counts when it is lower than the most
    /// of the organization, else the most of the organization counts.
    ASSISTANT_LOOK_UPS: Text = {
        key: "assistant.look-ups",
        name: message::SETTING_ASSISTANT_LOOK_UPS_NAME,
        default: AS_ORGANIZATION,
        levels: [Person],
        changed_by: EachPerson,
        help: message::SETTING_ASSISTANT_LOOK_UPS_HELP,
        lockable: false,
        format: Format::Choices(OWN_CHOICES),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::LookUps;

    #[test]
    fn m7_wp15_every_choice_is_a_look_up_the_code_reads() {
        for choice in MOST_CHOICES {
            assert!(LookUps::of_code(choice.value()).is_some(), "{choice:?}");
        }
        for choice in OWN_CHOICES {
            assert!(
                choice.value() == AS_ORGANIZATION || LookUps::of_code(choice.value()).is_some(),
                "{choice:?}"
            );
        }
        let start = ASSISTANT_LOOK_UPS_MOST.default_value().unwrap_or_default();
        assert_eq!(LookUps::of_code(&start), Some(LookUps::default()));
    }
}

//! The table of *Keep and delete rules* for the user docs (`userDocs/privacy/keep-and-delete.md`,
//! ADR-0045, decision 2): written from [`super::KEEP_KINDS`] with the starting periods and the words
//! of the message catalog, by `cargo xtask gen keep-rules`; DOC-GENERATED fails when the file
//! differs. Nothing in it is written by hand.

use std::fmt::Write as _;

use super::{KEEP_KINDS, KeepGroup, KeepRule, Length};
use crate::message::{Message, Value, keep_rules_no_fixed_end};
use crate::settings::Layers;

impl Length {
    /// The period in words: "7 days", "1 year".
    #[must_use]
    pub fn words(self) -> Message {
        Message::with_arguments(self.unit.name(), vec![("count", Value::Number(self.count))])
    }
}

impl KeepRule {
    /// How long the kind is kept, in words: its period, or that it has no fixed end and how far back
    /// the period reaches ([`Self::fixed_end`]).
    #[must_use]
    pub fn kept_for(&self, layers: &Layers) -> Message {
        let length = self.length(layers).words();
        if self.fixed_end() {
            return length;
        }
        keep_rules_no_fixed_end(length.render())
    }
}

/// The page of the user docs, as it should be.
#[must_use]
pub fn keep_table() -> String {
    let layers = Layers::default();
    let mut page = String::from(concat!(
        "<!-- Written by `cargo xtask gen keep-rules` from `bb_core::keeping::KEEP_KINDS`. ",
        "Do not edit. -->\n\n",
        "# Keep and delete rules\n\n",
        "How long Bugaboo keeps each kind of data at least, and who deletes it then. These are the\n",
        "periods a new organization starts with. A period that has a setting is changed by an\n",
        "administrator on *Organization settings › Privacy › Keep and delete rules*, never below\n",
        "the shortest period that kind allows.\n",
    ));
    for group in KeepGroup::ALL {
        let _ = write!(
            page,
            "\n## {}\n\n| Kind of data | Kept for | Why |\n|---|---|---|\n",
            Message::from(group.name()).render()
        );
        for row in KEEP_KINDS.iter().filter(|row| row.group == *group) {
            let _ = writeln!(
                page,
                "| {} | {} | {} |",
                Message::from(row.kind.name()).render(),
                row.kept_for(&layers).render(),
                Message::from(row.kind.why()).render()
            );
        }
    }
    page
}

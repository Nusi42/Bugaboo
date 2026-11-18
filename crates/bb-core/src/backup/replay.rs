//! Reading and merging the replay list (ADR-0044 decision 8, ADR-0046 decision 4): what a restore
//! must not undo. One line for each removal (with its choice about comments), switch-off and
//! key, AI app grant, passkey or support access taken back:
//! `<moment> <kind> <id>[ <choice>]`, with no name and no other value.
//!
//! The list is kept in three places (the data folder, every backup, the copy beside the keys of
//! `bugaboo keys export`); a restore reads every copy it has, merges them here and runs every line
//! newer than the backup. The writer of the list comes with *Remove a person*.

use core::fmt;

use jiff::Timestamp;

use crate::id::{AccessKeyId, GrantId, PasskeyId, UserId};

words! {
    /// What a line of the replay list stands for.
    pub enum ReplayKind {
        /// A person was removed; the line carries the choice about their comments.
        Removal => "removal",
        /// A person was switched off.
        SwitchOff => "switch-off",
        /// An access key was taken back.
        AccessKey => "access-key",
        /// The grant of an AI app was taken back.
        AiAppGrant => "ai-app-grant",
        /// A passkey was taken back.
        Passkey => "passkey",
        /// A support access was taken back.
        SupportAccess => "support-access",
    }
}

/// One line of the replay list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplayLine {
    /// When it happened.
    pub moment: Timestamp,
    /// What happened.
    pub kind: ReplayKind,
    /// The ID of what it happened to, as the ID writes itself.
    pub id: String,
    /// The choice of a removal about the person's comments.
    pub choice: Option<String>,
}

/// Whether `text` is one word of an ID or a choice: letters, digits and `-`.
fn is_word(text: &str) -> bool {
    !text.is_empty() && text.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

impl ReplayKind {
    /// Whether `id` is the ID of what a line of this kind names: a person for a removal and a
    /// switch-off, the key, grant or passkey taken back. A support access has no ID of its own
    /// yet, so any word is read.
    fn reads_id(self, id: &str) -> bool {
        match self {
            Self::Removal | Self::SwitchOff => id.parse::<UserId>().is_ok(),
            Self::AccessKey => id.parse::<AccessKeyId>().is_ok(),
            Self::AiAppGrant => id.parse::<GrantId>().is_ok(),
            Self::Passkey => id.parse::<PasskeyId>().is_ok(),
            Self::SupportAccess => is_word(id),
        }
    }
}

impl ReplayLine {
    /// Reads one line; `None` when it is not one, also when its ID is not the ID of what its kind
    /// names, so a restore never meets a line it cannot read.
    #[must_use]
    pub fn parse(line: &str) -> Option<Self> {
        let mut parts = line.split(' ');
        let moment = parts.next()?.parse().ok()?;
        let kind = ReplayKind::from_code(parts.next()?)?;
        let id = parts.next().filter(|id| kind.reads_id(id))?.to_owned();
        let choice = match parts.next() {
            Some(choice) if is_word(choice) => Some(choice.to_owned()),
            Some(_) => return None,
            None => None,
        };
        if parts.next().is_some() {
            return None;
        }
        Some(Self {
            moment,
            kind,
            id,
            choice,
        })
    }
}

impl fmt::Display for ReplayLine {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} {} {}", self.moment, self.kind, self.id)?;
        match &self.choice {
            Some(choice) => write!(formatter, " {choice}"),
            None => Ok(()),
        }
    }
}

/// The lines of one copy of the list, and how many of its lines could not be read.
#[must_use]
pub fn read(text: &str) -> (Vec<ReplayLine>, usize) {
    let mut lines = Vec::new();
    let mut unreadable = 0;
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        match ReplayLine::parse(line.trim_end()) {
            Some(read) => lines.push(read),
            None => unreadable += 1,
        }
    }
    (lines, unreadable)
}

/// Every line of every copy, each once, oldest first.
#[must_use]
pub fn merge(copies: impl IntoIterator<Item = Vec<ReplayLine>>) -> Vec<ReplayLine> {
    let mut merged: Vec<ReplayLine> = Vec::new();
    for line in copies.into_iter().flatten() {
        if !merged.contains(&line) {
            merged.push(line);
        }
    }
    merged.sort_by_key(|line| line.moment);
    merged
}

/// The text of a list.
#[must_use]
pub fn write(lines: &[ReplayLine]) -> String {
    lines.iter().fold(String::new(), |mut text, line| {
        text.push_str(&line.to_string());
        text.push('\n');
        text
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(text: &str) -> ReplayLine {
        ReplayLine::parse(text).unwrap()
    }

    #[test]
    fn ops_07_a_restore_merges_the_replay_list_of_the_folder_the_backup_and_keys_export() {
        let folder = "2026-11-02T10:00:00Z switch-off 0195f7a0-0000-4000-8000-000000000001\n\
                      2026-11-03T10:00:00Z access-key 0195f7a0-0000-4000-8000-000000000002\n";
        let backup = "2026-11-01T10:00:00Z passkey 0195f7a0-0000-4000-8000-000000000003\n\
                      2026-11-02T10:00:00Z switch-off 0195f7a0-0000-4000-8000-000000000001\n";
        let exported = "2026-11-04T10:00:00Z removal 0195f7a0-0000-4000-8000-000000000004 keep\n\
                        not a line\n";
        let copies: Vec<(Vec<ReplayLine>, usize)> =
            [folder, backup, exported].into_iter().map(read).collect();
        assert_eq!(copies[2].1, 1);
        let merged = merge(copies.into_iter().map(|(lines, _)| lines));
        let kinds: Vec<ReplayKind> = merged.iter().map(|line| line.kind).collect();
        assert_eq!(
            kinds,
            [
                ReplayKind::Passkey,
                ReplayKind::SwitchOff,
                ReplayKind::AccessKey,
                ReplayKind::Removal
            ]
        );
        assert_eq!(merged[3].choice.as_deref(), Some("keep"));
        let (again, unreadable) = read(&write(&merged));
        assert_eq!((again, unreadable), (merged, 0));
    }

    const PERSON: &str = "0195f7a0-0000-4000-8000-000000000001";

    #[test]
    fn a_line_with_more_than_kind_id_and_choice_is_not_read() {
        for bad in [
            "2026-11-02T10:00:00Z switch-off".to_owned(),
            format!("yesterday switch-off {PERSON}"),
            format!("2026-11-02T10:00:00Z unknown {PERSON}"),
            format!("2026-11-02T10:00:00Z removal {PERSON} b c"),
            "2026-11-02T10:00:00Z removal a/b".to_owned(),
        ] {
            assert_eq!(ReplayLine::parse(&bad), None, "{bad}");
        }
        let removal = format!("2026-11-02T10:00:00Z removal {PERSON} delete");
        assert_eq!(line(&removal).to_string(), removal);
    }

    #[test]
    fn a_line_whose_id_is_not_of_its_kind_is_not_read() {
        for bad in [
            "2026-11-02T10:00:00Z removal a",
            "2026-11-02T10:00:00Z switch-off not-an-id",
            "2026-11-02T10:00:00Z access-key 0195f7a0",
            "2026-11-02T10:00:00Z ai-app-grant x",
            "2026-11-02T10:00:00Z passkey ffff",
        ] {
            assert_eq!(ReplayLine::parse(bad), None, "{bad}");
        }
        let (lines, unreadable) = read(&format!(
            "2026-11-02T10:00:00Z access-key nope
2026-11-02T10:00:00Z switch-off {PERSON}
"
        ));
        assert_eq!((lines.len(), unreadable), (1, 1));
    }
}

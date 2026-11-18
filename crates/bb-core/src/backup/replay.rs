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

impl ReplayLine {
    /// Reads one line; `None` when it is not one.
    #[must_use]
    pub fn parse(line: &str) -> Option<Self> {
        let mut parts = line.split(' ');
        let moment = parts.next()?.parse().ok()?;
        let kind = ReplayKind::from_code(parts.next()?)?;
        let id = parts.next().filter(|id| is_word(id))?.to_owned();
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
        let folder = "2026-11-02T10:00:00Z switch-off 0195f7a0-0000-7000-8000-000000000001\n\
                      2026-11-03T10:00:00Z access-key 0195f7a0-0000-4000-8000-000000000002\n";
        let backup = "2026-11-01T10:00:00Z passkey 0195f7a0-0000-4000-8000-000000000003\n\
                      2026-11-02T10:00:00Z switch-off 0195f7a0-0000-7000-8000-000000000001\n";
        let exported = "2026-11-04T10:00:00Z removal 0195f7a0-0000-7000-8000-000000000004 keep\n\
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

    #[test]
    fn a_line_with_more_than_kind_id_and_choice_is_not_read() {
        for bad in [
            "2026-11-02T10:00:00Z switch-off",
            "yesterday switch-off a",
            "2026-11-02T10:00:00Z unknown a",
            "2026-11-02T10:00:00Z removal a b c",
            "2026-11-02T10:00:00Z removal a/b",
        ] {
            assert_eq!(ReplayLine::parse(bad), None, "{bad}");
        }
        assert_eq!(
            line("2026-11-02T10:00:00Z removal a delete").to_string(),
            "2026-11-02T10:00:00Z removal a delete"
        );
    }
}

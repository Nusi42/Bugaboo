//! The exemption files of the checks (`tools/checks/*-exemptions.toml`): one way to read them and
//! the rules every file shares. An entry names one thing that a rule would refuse and says why it
//! may stand; an entry without a reason, or one that nothing needs any more, fails, so a file
//! shrinks as the code is fixed and cannot go stale.

use std::collections::BTreeSet;

use anyhow::Context as _;
use serde::de::DeserializeOwned;

use super::Finding;
use crate::repo::Repo;
use crate::source::read_text;

/// The shortest reason that can say anything.
pub(super) const REASON_MIN: usize = 20;

/// Reads and parses one exemption file.
pub(super) fn load<T: DeserializeOwned>(repo: &Repo, path: &str) -> anyhow::Result<T> {
    parse(&read_text(repo, path)?, path)
}

/// Parses the text of the exemption file at `path`.
pub(super) fn parse<T: DeserializeOwned>(text: &str, path: &str) -> anyhow::Result<T> {
    toml::from_str(text).with_context(|| format!("{path} is not valid"))
}

/// One entry of an exemption file, as the rules every file shares see it.
pub(super) trait Exemption {
    /// What the entry names, as a person reads it.
    fn name(&self) -> String;
    /// Why the thing may stand.
    fn reason(&self) -> &str;
    /// What is said about an entry that nothing needs any more.
    fn unused(&self) -> String {
        "matches nothing that needs it any more. Remove it.".to_owned()
    }
}

/// The entries (by their place in the file) that a check needed.
#[derive(Default)]
pub(super) struct Used(BTreeSet<usize>);

impl Used {
    pub(super) fn mark(&mut self, index: usize) {
        self.0.insert(index);
    }

    fn contains(&self, index: usize) -> bool {
        self.0.contains(&index)
    }
}

/// The entries without a reason that says something.
pub(super) fn reason_findings<E: Exemption>(path: &str, entries: &[E]) -> Vec<Finding> {
    entries
        .iter()
        .filter(|entry| entry.reason().trim().len() < REASON_MIN)
        .map(|entry| {
            Finding::at(
                path,
                format!(
                    "The entry `{}` needs a reason that says why it may.",
                    entry.name()
                ),
            )
        })
        .collect()
}

/// The rules every file shares: each entry has a reason, and each was needed.
pub(super) fn judge<E: Exemption>(path: &str, entries: &[E], used: &Used) -> Vec<Finding> {
    let mut findings = reason_findings(path, entries);
    for (index, entry) in entries.iter().enumerate() {
        if !used.contains(index) {
            findings.push(Finding::at(
                path,
                format!("The entry `{}` {}", entry.name(), entry.unused()),
            ));
        }
    }
    findings
}

/// Whether the reason names a follow-up like `M1-F44`.
pub(super) fn names_a_follow_up(reason: &str) -> bool {
    reason
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '-')
        .any(is_follow_up)
}

fn is_follow_up(word: &str) -> bool {
    let mut parts = word.split('-');
    let (Some(milestone), Some(number), None) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
    let milestone_ok = milestone.len() >= 2
        && milestone.starts_with(['M', 'V'])
        && milestone[1..].chars().all(|c| c.is_ascii_digit());
    let number_ok = number.len() >= 2
        && number.starts_with('F')
        && number[1..].chars().all(|c| c.is_ascii_digit());
    milestone_ok && number_ok
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Entry(&'static str, &'static str);

    impl Exemption for Entry {
        fn name(&self) -> String {
            self.0.to_owned()
        }

        fn reason(&self) -> &str {
            self.1
        }
    }

    const LONG: &str = "A reason that says enough about why.";

    #[test]
    fn good_example_an_entry_with_a_reason_that_was_needed_passes() {
        let mut used = Used::default();
        used.mark(0);
        assert!(judge("x.toml", &[Entry("a", LONG)], &used).is_empty());
    }

    #[test]
    fn bad_example_an_entry_without_a_reason_or_without_need_fails() {
        let entries = [Entry("short", "ok"), Entry("stale", LONG)];
        let mut used = Used::default();
        used.mark(0);
        let messages: Vec<String> = judge("x.toml", &entries, &used)
            .into_iter()
            .map(|finding| finding.message)
            .collect();
        assert_eq!(messages.len(), 2, "{messages:?}");
        assert!(messages[0].contains("`short` needs a reason"));
        assert!(messages[1].contains("`stale` matches nothing"));
    }

    #[test]
    fn a_follow_up_is_a_milestone_and_a_number() {
        for good in ["see M1-F44.", "V2-F10 tracks it", "(M2-F60)"] {
            assert!(names_a_follow_up(good), "{good}");
        }
        for bad in ["", "M1", "F44", "M1-44", "ADR-0021", "nothing"] {
            assert!(!names_a_follow_up(bad), "{bad}");
        }
    }

    #[test]
    fn a_file_that_is_not_toml_names_itself() {
        let error = parse::<toml::Table>("not = [", "x.toml").unwrap_err();
        assert!(format!("{error:#}").contains("x.toml is not valid"));
    }
}

//! When two names are the same name (ISS-02, TAG-01, TAG-02, CF-02, ISS-04, WF-01): one key and one
//! question for everything that has a name a person types.
//!
//! Two names are the same when they differ only in capitals and in the spaces between and around
//! the words, so *Bug report*, *bug  report* and *BUG REPORT* are one name. Issue types, groups of
//! labels, labels, tags, fields, choices, kinds of relation and the statuses of a workflow all ask
//! here; the places that must keep a name unique (the use cases, and again inside the change after
//! the lock) differ only in which names they compare it with.

use crate::id::ProjectId;
use crate::offer::can_meet;

/// The name by which names are told apart: lower case, with the spaces in it squeezed. Two names
/// with the same key are the same name.
#[must_use]
pub fn name_key(name: &str) -> String {
    name.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Whether two names are the same name.
#[must_use]
pub fn same_name(one: &str, other: &str) -> bool {
    name_key(one) == name_key(other)
}

/// Whether a name must be judged: it is new (`before` is none) or it is not spelled as it was
/// stored. A name stored before names were told apart by one rule stays as it is, so a type, a
/// field or a workflow can still be given another icon or be retired. Every place that keeps names
/// unique asks this before it asks whether the name is taken.
#[must_use]
pub fn is_new_or_changed(before: Option<&str>, name: &str) -> bool {
    before.is_none_or(|stored| stored != name)
}

/// Whether one of `names` is the same name as `name`.
#[must_use]
pub fn name_among<'a>(names: impl IntoIterator<Item = &'a str>, name: &str) -> bool {
    let key = name_key(name);
    names.into_iter().any(|other| name_key(other) == key)
}

/// A subject (an issue type, a group of labels) as the question of [`taken_where_they_meet`] sees
/// it.
#[derive(Clone, Copy, Debug)]
pub struct Placed<'a> {
    /// Its name.
    pub name: &'a str,
    /// The project it was made in; none: for the whole organization.
    pub project: Option<ProjectId>,
    /// Whether it is the one that is being renamed, which cannot clash with itself.
    pub is_the_one_changed: bool,
}

/// Whether one of `others` that can meet a subject made in `owner` already has the name `name`
/// (`can_meet`: made for the whole organization, or in the same project or one inside the other).
/// `is_within(project, outside)` says whether a project is inside another, at any depth.
#[must_use]
pub fn taken_where_they_meet<T>(
    others: &[T],
    name: &str,
    owner: Option<ProjectId>,
    is_within: impl Fn(ProjectId, ProjectId) -> bool,
    placed: impl for<'a> Fn(&'a T) -> Placed<'a>,
) -> bool {
    let key = name_key(name);
    others.iter().any(|other| {
        let other = placed(other);
        !other.is_the_one_changed
            && name_key(other.name) == key
            && can_meet(owner, other.project, &is_within)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::SequentialIds;

    #[test]
    fn tag_01_two_spellings_of_a_name_have_one_key() {
        assert_eq!(name_key("  Login  Page "), name_key("login page"));
        assert_ne!(name_key("login"), name_key("logins"));
    }

    #[test]
    fn iss_02_names_differ_only_in_capitals_and_spaces_are_one_name() {
        assert!(same_name("Bug report", "bug  report"));
        assert!(same_name(" BUG REPORT ", "Bug report"));
        assert!(!same_name("Bug report", "Bug reports"));
        assert!(name_among(["Task", "Bug  report"], "bug report"));
        assert!(!name_among(["Task"], "bug report"));
    }

    #[test]
    fn iss_02_a_name_is_judged_only_when_it_is_new_or_changed() {
        assert!(is_new_or_changed(None, "Bug"));
        assert!(is_new_or_changed(Some("Bug"), "Bugs"));
        assert!(is_new_or_changed(Some("Bug  report"), "Bug report"));
        assert!(is_new_or_changed(Some("Bug"), "bug"));
        assert!(!is_new_or_changed(Some("Bug"), "Bug"));
    }

    struct Subject(&'static str, Option<ProjectId>, bool);

    fn placed(subject: &Subject) -> Placed<'_> {
        Placed {
            name: subject.0,
            project: subject.1,
            is_the_one_changed: subject.2,
        }
    }

    #[test]
    fn iss_02_a_name_is_taken_only_where_the_subjects_can_meet() -> Result<(), crate::id::IdError> {
        let mut ids = SequentialIds::default();
        let (parent, child, sibling) = (
            ProjectId::generate(&mut ids)?,
            ProjectId::generate(&mut ids)?,
            ProjectId::generate(&mut ids)?,
        );
        let within = |inside: ProjectId, outside: ProjectId| inside == child && outside == parent;
        let all = [
            Subject("Bug report", Some(parent), false),
            Subject("Task", None, false),
        ];
        let taken = |name, owner| taken_where_they_meet(&all, name, owner, within, placed);
        // The same name in the same project, below it, and for the whole organization.
        assert!(taken("bug  REPORT", Some(parent)));
        assert!(taken("bug report", Some(child)));
        assert!(taken("bug report", None));
        // A sibling project never meets it, and a name nobody has is free.
        assert!(!taken("bug report", Some(sibling)));
        assert!(!taken("Epic", Some(parent)));
        // An organization-wide subject meets every project.
        assert!(taken("task", Some(sibling)));
        // The one that is renamed does not clash with itself.
        let itself = [Subject("Bug report", Some(parent), true)];
        assert!(!taken_where_they_meet(
            &itself,
            "bug report",
            Some(parent),
            within,
            placed
        ));
        Ok(())
    }
}

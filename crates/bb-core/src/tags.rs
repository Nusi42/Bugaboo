//! Tags and labels (TAG-01, TAG-02, TAG-03): the free words anyone who may change an issue adds, and
//! the organization's own list of labels, which may be kept in groups.
//!
//! A tag is found by its name without regard to capitals, so *Login* and *login* are one tag. It
//! has a color of the category colors (the same twelve as a choice of an own field) and may be
//! limited to some projects. A label is made by people with *Manage labels and tags*; a group of
//! labels may be a *Pick one* group, in which an issue has at most one label.

use jiff::Timestamp;

use crate::id::{IssueId, IssueTypeId, LabelGroupId, LabelId, ProjectId, TagId, UserId};
use crate::own_fields::ChoiceColor;
use crate::personal::Entity;

/// The longest name of a tag or a label, in characters.
pub const TAG_NAME_MAX: usize = 40;

/// The most tags one issue has, so a row of them stays a row.
pub const TAGS_PER_ISSUE_MAX: usize = 30;

/// The most labels one issue has.
pub const LABELS_PER_ISSUE_MAX: usize = 30;

/// The most tags in one list of suggestions.
pub const TAG_SUGGESTIONS_MAX: usize = 10;

/// The most groups of labels one place has: the whole organization, and each project, counted on
/// its own (the groups created there).
pub const LABEL_GROUPS_MAX: usize = 100;

/// The most labels one place has: the labels of the groups created in a project count against that
/// project, all the others against the whole organization.
pub const LABELS_MAX: usize = 1000;

/// The most projects one tag is limited to.
pub const TAG_PROJECTS_MAX: usize = 200;

/// The name by which tags and labels are told apart: lower case, with the spaces in it squeezed.
/// Two names with the same key are the same name.
#[must_use]
pub fn name_key(name: &str) -> String {
    name.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Which issues a viewer reads when tags are looked up for them (SEC-06): every issue of the
/// projects in `all`, and only the issues `person` reported in the projects in `own`. A project the
/// viewer does not see, or where the field is hidden from them, is in neither list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IssueSight {
    /// The projects where the viewer sees every issue.
    pub all: Vec<ProjectId>,
    /// The projects where the viewer sees only the issues they reported.
    pub own: Vec<ProjectId>,
    /// The viewer.
    pub person: UserId,
    /// The issues of these types in these projects are not read: the role hides the field the
    /// lookup is about (*Tags*) for the type there.
    pub hidden: Vec<(ProjectId, IssueTypeId)>,
    /// Also count the issues in the trash (to ask whether a tag is used at all, never to show).
    pub with_trashed: bool,
}

/// A free word anyone who may change an issue puts on it (TAG-01).
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct Tag {
    /// The ID of the tag.
    #[not_personal]
    pub id: TagId,
    /// What it is called, as it was first written. Free text that could name a person.
    #[personal(category = "content", basis = "legitimate_interests")]
    pub name: String,
    /// Its color.
    #[not_personal]
    pub color: ChoiceColor,
    /// When it was made.
    #[not_personal]
    pub created_at: Timestamp,
    /// Counts up with every change, for the check that nobody changed it meanwhile (ADR-0019).
    #[not_personal]
    pub version: i64,
}

/// A project a tag is limited to (with its sub-projects). A tag without any is for every project.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct TagProject {
    /// The tag.
    #[not_personal]
    pub tag: TagId,
    /// The project.
    #[not_personal]
    pub project: ProjectId,
}

/// Whether a tag is allowed in a project whose projects from the top down are `chain`: a tag
/// without a limit is allowed everywhere, a limited one only in (the sub-projects of) its projects.
#[must_use]
pub fn tag_allowed_in(tag: TagId, limits: &[TagProject], chain: &[ProjectId]) -> bool {
    let mut own = limits.iter().filter(|limit| limit.tag == tag).peekable();
    own.peek().is_none() || own.any(|limit| chain.contains(&limit.project))
}

/// A tag on an issue.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct IssueTag {
    /// The issue.
    #[not_personal]
    pub issue: IssueId,
    /// The tag.
    #[not_personal]
    pub tag: TagId,
    /// Who put it there.
    #[personal(category = "activity", basis = "contract")]
    pub added_by: UserId,
    /// When.
    #[not_personal]
    pub added_at: Timestamp,
}

/// A group of labels (TAG-02). In a *Pick one* group an issue has at most one label.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct LabelGroup {
    /// The ID of the group.
    #[not_personal]
    pub id: LabelGroupId,
    /// What it is called. Free text that could name a person.
    #[personal(category = "content", basis = "legitimate_interests")]
    pub name: String,
    /// Whether an issue has at most one label of the group.
    #[not_personal]
    pub pick_one: bool,
    /// The project it was made in (offered there and below until a place says otherwise, and
    /// looked after by whoever manages labels there); none for the whole organization.
    #[not_personal]
    pub project: Option<ProjectId>,
    /// The place in the order the groups are offered in.
    #[not_personal]
    pub position: i64,
    /// When it was retired: its labels are not offered for new issues, while the old ones stay.
    #[not_personal]
    pub retired_at: Option<Timestamp>,
    /// Counts up with every change.
    #[not_personal]
    pub version: i64,
}

/// A label of the organization's list (TAG-02).
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct Label {
    /// The ID of the label.
    #[not_personal]
    pub id: LabelId,
    /// The group it is in, if any.
    #[not_personal]
    pub group: Option<LabelGroupId>,
    /// What it is called. Free text that could name a person.
    #[personal(category = "content", basis = "legitimate_interests")]
    pub name: String,
    /// Its color.
    #[not_personal]
    pub color: ChoiceColor,
    /// The place in the order the labels are offered in.
    #[not_personal]
    pub position: i64,
    /// When it was retired: it is not offered for new issues, while the old ones keep it.
    #[not_personal]
    pub retired_at: Option<Timestamp>,
    /// Counts up with every change.
    #[not_personal]
    pub version: i64,
}

/// A label on an issue.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct IssueLabel {
    /// The issue.
    #[not_personal]
    pub issue: IssueId,
    /// The label.
    #[not_personal]
    pub label: LabelId,
    /// Who put it there.
    #[personal(category = "activity", basis = "contract")]
    pub added_by: UserId,
    /// When.
    #[not_personal]
    pub added_at: Timestamp,
}

#[cfg(test)]
mod tests {
    use super::{TagProject, name_key, tag_allowed_in};
    use crate::id::{ProjectId, SequentialIds, TagId};

    #[test]
    fn tag_01_two_spellings_of_a_name_have_one_key() {
        assert_eq!(name_key("  Login  Page "), name_key("login page"));
        assert_ne!(name_key("login"), name_key("logins"));
    }

    #[test]
    fn tag_01_a_limited_tag_is_allowed_only_in_its_projects() -> Result<(), crate::id::IdError> {
        let mut ids = SequentialIds::default();
        let (tag, other) = (TagId::generate(&mut ids)?, TagId::generate(&mut ids)?);
        let (here, there) = (
            ProjectId::generate(&mut ids)?,
            ProjectId::generate(&mut ids)?,
        );
        let limits = [TagProject { tag, project: here }];
        assert!(tag_allowed_in(tag, &limits, &[there, here]));
        assert!(!tag_allowed_in(tag, &limits, &[there]));
        assert!(tag_allowed_in(other, &limits, &[there]));
        Ok(())
    }
}

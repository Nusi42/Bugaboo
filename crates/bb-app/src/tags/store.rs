//! The port of the tag and label use cases: what they need from storage beyond the issues port.
//!
//! Like the other ports, reads take `&self` and **writes** are associated functions that run inside
//! `record_change`, so a change and its domain event are committed together (ARCH-EVENTS).

use std::future::Future;

use bb_core::id::{IssueId, LabelGroupId, LabelId, ProjectId, TagId};
use bb_core::tags::{IssueLabel, IssueSight, IssueTag, Label, LabelGroup, Tag, TagProject};
use jiff::Timestamp;

use crate::issues::IssueStore;

/// A tag with what decides whether a person may know it (`Ctx::known_tag_names`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TagUse {
    /// The tag.
    pub tag: Tag,
    /// Whether an issue the person reads has it.
    pub seen: bool,
    /// Whether any issue has it, the trash and the issues of others too.
    pub anywhere: bool,
    /// Whether it is limited to some projects.
    pub limited: bool,
    /// Whether it is limited to a project the person sees.
    pub limited_to_visible: bool,
}
use crate::store_macros::{queries, reads, writes};

/// What the tag and label use cases need from storage.
pub trait TagStore: IssueStore + 'static {
    queries! {
        /// Every tag, by name.
        fn tags() -> Vec<Tag> = tags_in;
        /// The projects the tags are limited to.
        fn tag_projects() -> Vec<TagProject> = tag_projects_in;
        /// The tags on an issue, in the order they were put there.
        fn issue_tags(issue: IssueId) -> Vec<IssueTag> = issue_tags_in;
        /// Every group of labels, in the order they are offered in (then by ID).
        fn label_groups() -> Vec<LabelGroup> = label_groups_in;
        /// Every label, in the order they are offered in (then by ID).
        fn labels() -> Vec<Label> = labels_in;
        /// The labels on an issue, in the order they were put there.
        fn issue_labels(issue: IssueId) -> Vec<IssueLabel> = issue_labels_in;
        /// The issues (also those in the trash) that have a tag, by ID: the ones after `after`,
        /// at most `limit`.
        fn issues_with_tag(tag: TagId, after: Option<IssueId>, limit: u64) -> Vec<IssueId>
            = issues_with_tag_in;
        /// The issues (also those in the trash) that have a label, by ID: the ones after `after`,
        /// at most `limit`.
        fn issues_with_label(label: LabelId, after: Option<IssueId>, limit: u64) -> Vec<IssueId>
            = issues_with_label_in;
        /// The tags on these issues.
        fn tags_of_issues(issues: Vec<IssueId>) -> Vec<IssueTag> = tags_of_issues_in;
        /// The labels on these issues.
        fn labels_of_issues(issues: Vec<IssueId>) -> Vec<IssueLabel> = labels_of_issues_in;
    }

    reads! {
        /// The tags used on the issues that are not in the trash and that the viewer reads, whose key holds
        /// `part`, by key, at most `limit`.
        fn used_tags(sight: &IssueSight, part: &str, limit: u64) -> Vec<Tag>;
        /// Every tag with what the rule of what a person may know of a tag needs, by key, in one
        /// statement: whether it is used on an issue of `seen` and on an issue of `anywhere`, and
        /// whether it is limited to projects and to one of `visible`.
        fn tags_with_use(
            seen: &IssueSight,
            anywhere: &IssueSight,
            visible: &[ProjectId]
        ) -> Vec<TagUse>;
    }

    writes! {
        /// Adds a tag. A name that is used already (without regard to capitals) is a duplicate key.
        fn insert_tag(tag: &Tag) -> ();
        /// Changes the name and the color of a tag, counts its version up, and says whether the
        /// tag was still at `version`.
        fn update_tag(
            id: TagId,
            name: &str,
            color: bb_core::own_fields::ChoiceColor,
            version: i64
        ) -> bool;
        /// Replaces the projects a tag is limited to.
        fn replace_tag_projects(tag: TagId, projects: &[ProjectId]) -> ();
        /// Takes a tag away, with its links to issues and projects; `false` when it was not there.
        fn delete_tag(id: TagId) -> bool;
        /// Puts a tag on an issue; a duplicate key when it is there already.
        fn insert_issue_tag(link: &IssueTag) -> ();
        /// Takes a tag off an issue; `false` when it was not on it.
        fn delete_issue_tag(issue: IssueId, tag: TagId) -> bool;
        /// Puts every link to the tag `from` onto the tag `into`; an issue that has both keeps one
        /// link. The tag `from` stays, without links.
        fn retarget_issue_tags(from: TagId, into: TagId) -> ();
        /// Adds a group of labels.
        fn insert_label_group(group: &LabelGroup) -> ();
        /// Changes the name, the *Pick one* choice and the retired date of a group, counts its
        /// version up, and says whether the group was still at `version`.
        fn update_label_group(
            id: LabelGroupId,
            name: &str,
            pick_one: bool,
            retired_at: Option<Timestamp>,
            version: i64
        ) -> bool;
        /// Adds a label. A name used in the same group (without regard to capitals) is a duplicate
        /// key.
        fn insert_label(label: &Label) -> ();
        /// Changes the name, color, group and retired date of a label, counts its version up, and
        /// says whether the label was still at `version`.
        fn update_label(
            id: LabelId,
            name: &str,
            color: bb_core::own_fields::ChoiceColor,
            group: Option<LabelGroupId>,
            retired_at: Option<Timestamp>,
            version: i64
        ) -> bool;
        /// Puts a label on an issue; a duplicate key when it is there already.
        fn insert_issue_label(link: &IssueLabel) -> ();
        /// Takes a label off an issue; `false` when it was not on it.
        fn delete_issue_label(issue: IssueId, label: LabelId) -> bool;
        /// Puts every link to the label `from` onto the label `into`; an issue that has both keeps
        /// one link. The label `from` stays, without links.
        fn retarget_issue_labels(from: LabelId, into: LabelId) -> ();
        /// Takes a label away; `false` when it was not there. Its links to issues must be gone.
        fn delete_label(id: LabelId) -> bool;
        /// Waits for every other change of the names and the places of tags and labels, until the
        /// end of this change: names are judged and written by one change at a time.
        fn lock_tags_and_labels() -> ();
    }
}

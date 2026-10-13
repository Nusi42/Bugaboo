//! The port of the tag and label use cases: what they need from storage beyond the issues port.
//!
//! Like the other ports, reads take `&self` and **writes** are associated functions that run inside
//! `record_change`, so a change and its domain event are committed together (ARCH-EVENTS).

use std::future::Future;

use bb_core::id::{IssueId, LabelGroupId, LabelId, ProjectId, TagId};
use bb_core::tags::{IssueLabel, IssueTag, Label, LabelGroup, Tag, TagProject};
use jiff::Timestamp;

use crate::issues::IssueStore;
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
    }

    reads! {
        /// The tags on these issues.
        fn tags_of_issues(issues: &[IssueId]) -> Vec<IssueTag>;
        /// The labels on these issues.
        fn labels_of_issues(issues: &[IssueId]) -> Vec<IssueLabel>;
        /// The tags used on issues that are not in the trash, in these projects, whose key holds
        /// `part`, by key, at most `limit`.
        fn used_tags(projects: &[ProjectId], part: &str, limit: u64) -> Vec<Tag>;
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

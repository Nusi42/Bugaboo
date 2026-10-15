//! What the tag and label use cases take and give back (TAG-01 … TAG-03). Inputs that change
//! something are named after what they do; inputs that only read end in `Query` (ARCH-EVENTS).
//! Everything that is given back is wrapped in `Authorized<T>` by the use case.

use bb_core::id::{IssueId, IssueTypeId, LabelGroupId, LabelId, ProjectId, TagId};
use bb_core::own_fields::{ChoiceColor, Edit};
use bb_core::tags::{Label, LabelGroup, Tag};
use bb_policy::Masked;
use serde::Serialize;

use crate::issues::IssueTypeRef;
use crate::preview::PreviewSummary;
use crate::projects::ProjectRef;
use crate::relations::IssueLine;

/// A tag as a small reference: what a chip needs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TagRef {
    /// The ID of the tag.
    pub id: TagId,
    /// What it is called.
    pub name: String,
    /// Its color.
    pub color: ChoiceColor,
}

impl From<&Tag> for TagRef {
    fn from(tag: &Tag) -> Self {
        Self {
            id: tag.id,
            name: tag.name.clone(),
            color: tag.color,
        }
    }
}

/// A label as a small reference: what a chip needs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LabelRef {
    /// The ID of the label.
    pub id: LabelId,
    /// What it is called.
    pub name: String,
    /// Its color.
    pub color: ChoiceColor,
    /// The group it is in, if any.
    pub group: Option<LabelGroupId>,
}

impl From<&Label> for LabelRef {
    fn from(label: &Label) -> Self {
        Self {
            id: label.id,
            name: label.name.clone(),
            color: label.color,
            group: label.group,
        }
    }
}

/// The tags and labels of one issue, and what the person may do with them. A member that holds a
/// field of the catalogue is dropped when the viewer's level for the field is *Hidden*.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Masked)]
pub struct IssueSorting {
    /// The issue.
    #[masked(open)]
    pub issue: IssueId,
    /// The version of the issue now. A change of a tag or a label counts it up, so the page
    /// writes it into what it knows about the issue and a change right after does not conflict
    /// with the person's own (ADR-0019).
    #[masked(open)]
    pub version: i64,
    /// Its issue type. The levels set for it count for the fields of this issue (ADR-0027).
    #[masked(nested, item_type)]
    pub issue_type: IssueTypeRef,
    /// The tags on it, in the order they were put there.
    #[masked(field = "tags")]
    pub tags: Vec<TagRef>,
    /// The labels on it, in the order they were put there.
    #[masked(field = "labels")]
    pub labels: Vec<LabelRef>,
    /// Whether the person may put tags on the issue and take them off.
    #[masked(open)]
    pub may_change_tags: bool,
    /// Whether the person may put labels on the issue and take them off.
    #[masked(open)]
    pub may_change_labels: bool,
}

/// Which issue and what is asked of it.
pub struct IssueSortingQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The issue, by its readable ID (also an old one) or its internal ID.
    pub issue: String,
}

/// Put a tag on an issue, by its name: a name that is new makes the tag.
pub struct AddTag {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The issue, by its readable ID (also an old one) or its internal ID.
    pub issue: String,
    /// The name of the tag, as typed.
    pub name: String,
}

/// Take a tag off an issue.
pub struct RemoveTag {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The issue, by its readable ID (also an old one) or its internal ID.
    pub issue: String,
    /// The tag.
    pub tag: TagId,
}

/// Put a label on an issue. In a *Pick one* group the label takes the place of the one the issue
/// has from that group.
pub struct AddLabel {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The issue, by its readable ID (also an old one) or its internal ID.
    pub issue: String,
    /// The label.
    pub label: LabelId,
}

/// Take a label off an issue.
pub struct RemoveLabel {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The issue, by its readable ID (also an old one) or its internal ID.
    pub issue: String,
    /// The label.
    pub label: LabelId,
}

/// The tags to offer while a person types in a project.
pub struct TagSuggestionsQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The project of the issue the tag goes on, by its ID or its short name.
    pub project: String,
    /// What has been typed.
    pub text: String,
    /// The issue type of the issue the tag goes on, when it is known (the issue page, the new-issue
    /// form): its level for *Tags* decides. Without it the lowest level across types counts.
    pub issue_type: Option<IssueTypeId>,
}

/// The tags to offer: the ones used in projects the person sees that fit the project and the text.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TagSuggestions {
    /// The tags, by name.
    pub tags: Vec<TagRef>,
}

impl Masked for TagSuggestions {}

/// A tag as the people who look after tags see it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TagView {
    /// The ID of the tag.
    pub id: TagId,
    /// What it is called.
    pub name: String,
    /// Its color.
    pub color: ChoiceColor,
    /// Whether it is for every project.
    pub whole_organization: bool,
    /// The projects it is limited to, as far as the person sees them.
    pub projects: Vec<ProjectRef>,
    /// Counts up with every change. A change is made on the version that was looked at.
    pub version: i64,
}

impl Masked for TagView {}

/// The tags of the organization, a page at a time, for the people who look after them.
pub struct TagsQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// Where the page starts: the `next_cursor` of the page before.
    pub cursor: Option<String>,
    /// How many tags a page holds at most.
    pub limit: u32,
}

/// A page of tags, by name.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TagPage {
    /// The tags.
    pub items: Vec<TagView>,
    /// The cursor of the next page, if there is one.
    pub next_cursor: Option<String>,
}

impl Masked for TagPage {}

/// Change a tag: what is `Some` is changed. A new name is a change of every issue that has the
/// tag.
pub struct ChangeTag {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The tag.
    pub tag: TagId,
    /// The version the person looked at.
    pub version: i64,
    /// A new name.
    pub name: Option<String>,
    /// A new color.
    pub color: Option<ChoiceColor>,
    /// The projects the tag is limited to from now on; none for every project.
    pub projects: Option<Vec<ProjectId>>,
}

/// Move every issue from one tag to another, then take the first tag away.
pub struct CombineTags {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The tag that goes away.
    pub tag: TagId,
    /// The version of that tag the person looked at.
    pub version: i64,
    /// The tag that stays.
    pub into: TagId,
    /// The version of that tag the person looked at.
    pub into_version: i64,
}

/// What renaming or combining a tag or a label would do.
pub struct TagChangePreviewQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The tag.
    pub tag: TagId,
    /// The new name, when it is renamed.
    pub name: Option<String>,
    /// The tag it would be combined with, when it is.
    pub into: Option<TagId>,
}

/// One issue that a rename or a combine changes, with its tags or labels before and after.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ChangedIssue {
    /// The issue, as the person sees it.
    pub issue: IssueLine,
    /// The names it has now.
    pub before: Vec<String>,
    /// The names it will have.
    pub after: Vec<String>,
}

/// The most issues a preview of a rename or a combine lists.
pub const CHANGE_PREVIEW_ISSUES_MAX: usize = 100;

/// What renaming or combining a tag or a label would do to the issues the person sees.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TagChangePreview {
    /// How many issues change, as far as the person sees them.
    pub summary: PreviewSummary,
    /// The first of them.
    pub issues: Vec<ChangedIssue>,
    /// The list is cut at [`CHANGE_PREVIEW_ISSUES_MAX`] issues.
    pub truncated: bool,
}

impl Masked for TagChangePreview {}

/// A group of labels.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LabelGroupView {
    /// The ID of the group.
    pub id: LabelGroupId,
    /// What it is called.
    pub name: String,
    /// Whether an issue has at most one label of the group.
    pub pick_one: bool,
    /// Whether it is retired: old issues keep its labels, new ones are not offered them.
    pub retired: bool,
    /// Counts up with every change.
    pub version: i64,
}

impl From<&LabelGroup> for LabelGroupView {
    fn from(group: &LabelGroup) -> Self {
        Self {
            id: group.id,
            name: group.name.clone(),
            pick_one: group.pick_one,
            retired: group.retired_at.is_some(),
            version: group.version,
        }
    }
}

/// A label as the list of the organization shows it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LabelView {
    /// The ID of the label.
    pub id: LabelId,
    /// The group it is in, if any.
    pub group: Option<LabelGroupId>,
    /// What it is called.
    pub name: String,
    /// Its color.
    pub color: ChoiceColor,
    /// Whether it is retired: old issues keep it, new ones are not offered it.
    pub retired: bool,
    /// Counts up with every change.
    pub version: i64,
}

impl From<&Label> for LabelView {
    fn from(label: &Label) -> Self {
        Self {
            id: label.id,
            group: label.group,
            name: label.name.clone(),
            color: label.color,
            retired: label.retired_at.is_some(),
            version: label.version,
        }
    }
}

/// The labels of the organization with their groups, in the order they are offered in.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Labels {
    /// The groups.
    pub groups: Vec<LabelGroupView>,
    /// The labels, those in a group and those that are in none.
    pub labels: Vec<LabelView>,
    /// Whether the person looks after labels and tags.
    pub may_manage: bool,
}

impl Masked for Labels {}

/// The labels of the organization.
pub struct LabelsQuery {
    /// The secret of the session of the request.
    pub session_token: String,
}

/// Make a group of labels, at the end of the order.
pub struct MakeLabelGroup {
    /// The secret of the session of the request.
    pub session_token: String,
    /// What it is called.
    pub name: String,
    /// Whether an issue has at most one label of the group.
    pub pick_one: bool,
}

/// Change a group of labels: what is `Some` is changed.
pub struct ChangeLabelGroup {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The group.
    pub group: LabelGroupId,
    /// The version the person looked at.
    pub version: i64,
    /// A new name.
    pub name: Option<String>,
    /// Whether an issue has at most one label of the group from now on.
    pub pick_one: Option<bool>,
    /// `true` retires the group, `false` offers it again.
    pub retired: Option<bool>,
}

/// Make a label, at the end of the order.
pub struct MakeLabel {
    /// The secret of the session of the request.
    pub session_token: String,
    /// What it is called.
    pub name: String,
    /// Its color; the next in turn when left out.
    pub color: Option<ChoiceColor>,
    /// The group it goes into.
    pub group: Option<LabelGroupId>,
}

/// Change a label: what is `Some` is changed. A new name or a new group is a change of every issue
/// that has the label.
pub struct ChangeLabel {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The label.
    pub label: LabelId,
    /// The version the person looked at.
    pub version: i64,
    /// A new name.
    pub name: Option<String>,
    /// A new color.
    pub color: Option<ChoiceColor>,
    /// A new group, or none to take the label out of its group.
    pub group: Edit<LabelGroupId>,
    /// `true` retires the label, `false` offers it again.
    pub retired: Option<bool>,
}

/// Move every issue from one label to another, then take the first label away.
pub struct CombineLabels {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The label that goes away.
    pub label: LabelId,
    /// The version of that label the person looked at.
    pub version: i64,
    /// The label that stays.
    pub into: LabelId,
    /// The version of that label the person looked at.
    pub into_version: i64,
}

/// What renaming or combining a label would do.
pub struct LabelChangePreviewQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The label.
    pub label: LabelId,
    /// The new name, when it is renamed.
    pub name: Option<String>,
    /// The label it would be combined with, when it is.
    pub into: Option<LabelId>,
}

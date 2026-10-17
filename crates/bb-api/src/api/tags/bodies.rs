//! The bodies and queries of the routes of labels and tags.

use super::{
    ChangedIssue, ChoiceColor, Deserialize, Edit, IssueLabelsAndTags, IssueRef, IssueTypeRef,
    LabelGroupView, LabelRef, LabelView, Labels, Masked, PlaceRef, PreviewSummaryBody, ProjectRef,
    Serialize, TagChangePreview, TagRef, TagView, ToSchema,
};

/// A tag as a small reference: what a chip needs.
#[derive(Serialize, ToSchema)]
pub(crate) struct TagRefBody {
    /// The ID of the tag.
    pub(super) id: String,
    /// What it is called.
    pub(super) name: String,
    /// Its color, as a category token of the theme.
    pub(super) color: ChoiceColor,
}

impl From<&TagRef> for TagRefBody {
    fn from(tag: &TagRef) -> Self {
        Self {
            id: tag.id.to_string(),
            name: tag.name.clone(),
            color: tag.color,
        }
    }
}

/// A label as a small reference: what a chip needs.
#[derive(Serialize, ToSchema)]
pub(crate) struct LabelRefBody {
    /// The ID of the label.
    pub(super) id: String,
    /// What it is called.
    pub(super) name: String,
    /// Its color, as a category token of the theme.
    pub(super) color: ChoiceColor,
    /// The group it is in, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) group: Option<String>,
}

impl From<&LabelRef> for LabelRefBody {
    fn from(label: &LabelRef) -> Self {
        Self {
            id: label.id.to_string(),
            name: label.name.clone(),
            color: label.color,
            group: label.group.map(|group| group.to_string()),
        }
    }
}

/// The tags and labels of one issue, and what the person may do with them. A member that holds a
/// field of the catalogue is left out when the person's level for the field is *Hidden*.
#[derive(Serialize, ToSchema, Masked)]
pub(crate) struct IssueLabelsAndTagsBody {
    /// The internal ID of the issue.
    #[masked(open)]
    pub(super) issue: String,
    /// The version of the issue now. A change of a tag or a label counts it up.
    #[masked(open)]
    pub(super) version: i64,
    /// Its issue type. The levels set for it count for the fields of this issue.
    #[masked(nested, item_type)]
    pub(super) issue_type: IssueTypeRef,
    /// The tags on it, in the order they were put there.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(field = "tags")]
    pub(super) tags: Option<Vec<TagRefBody>>,
    /// The labels on it, in the order they were put there.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(field = "labels")]
    pub(super) labels: Option<Vec<LabelRefBody>>,
    /// Whether the person may put tags on the issue and take them off.
    #[masked(open)]
    pub(super) may_change_tags: bool,
    /// Whether the person may put labels on the issue and take them off.
    #[masked(open)]
    pub(super) may_change_labels: bool,
}

impl From<IssueLabelsAndTags> for IssueLabelsAndTagsBody {
    fn from(labels_and_tags: IssueLabelsAndTags) -> Self {
        Self {
            issue: labels_and_tags.issue.to_string(),
            version: labels_and_tags.version,
            issue_type: IssueTypeRef::from(&labels_and_tags.issue_type),
            tags: Some(labels_and_tags.tags.iter().map(TagRefBody::from).collect()),
            labels: Some(
                labels_and_tags
                    .labels
                    .iter()
                    .map(LabelRefBody::from)
                    .collect(),
            ),
            may_change_tags: labels_and_tags.may_change_tags,
            may_change_labels: labels_and_tags.may_change_labels,
        }
    }
}

/// The tags to offer while a person types.
#[derive(Serialize, ToSchema)]
pub(crate) struct TagSuggestionsBody {
    /// The tags, by name.
    pub(super) tags: Vec<TagRefBody>,
}

impl Masked for TagSuggestionsBody {}

/// A tag as the people who look after tags see it: the fields that can be asked for.
#[derive(Serialize, ToSchema)]
pub(crate) struct TagItem {
    /// The ID of the tag.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) id: Option<String>,
    /// What it is called.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) name: Option<String>,
    /// Its color, as a category token of the theme.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) color: Option<ChoiceColor>,
    /// Whether it is for every project.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) whole_organization: Option<bool>,
    /// The projects it is limited to, as far as the person sees them.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) projects: Option<Vec<ProjectRef>>,
    /// Counts up with every change. A change is made on the version that was looked at.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) version: Option<i64>,
}

impl Masked for TagItem {}

impl From<&TagView> for TagItem {
    fn from(tag: &TagView) -> Self {
        Self {
            id: Some(tag.id.to_string()),
            name: Some(tag.name.clone()),
            color: Some(tag.color),
            whole_organization: Some(tag.whole_organization),
            projects: Some(tag.projects.iter().map(ProjectRef::from).collect()),
            version: Some(tag.version),
        }
    }
}

/// One issue that a rename or a combine changes.
#[derive(Serialize, ToSchema)]
pub(crate) struct ChangedIssueBody {
    /// The issue, as the person sees it.
    pub(super) issue: IssueRef,
    /// The names it has now.
    pub(super) before: Vec<String>,
    /// The names it will have.
    pub(super) after: Vec<String>,
}

impl From<&ChangedIssue> for ChangedIssueBody {
    fn from(changed: &ChangedIssue) -> Self {
        Self {
            issue: IssueRef::from(&changed.issue),
            before: changed.before.clone(),
            after: changed.after.clone(),
        }
    }
}

/// What renaming or combining a tag or a label would do to the issues the person sees.
#[derive(Serialize, ToSchema)]
pub(crate) struct ChangePreviewBody {
    /// How many issues change.
    pub(super) summary: PreviewSummaryBody,
    /// The first of them.
    pub(super) issues: Vec<ChangedIssueBody>,
    /// The list is cut: there are more issues than it names.
    pub(super) truncated: bool,
}

impl Masked for ChangePreviewBody {}

impl From<TagChangePreview> for ChangePreviewBody {
    fn from(preview: TagChangePreview) -> Self {
        Self {
            summary: PreviewSummaryBody::from(preview.summary),
            issues: preview.issues.iter().map(ChangedIssueBody::from).collect(),
            truncated: preview.truncated,
        }
    }
}

/// A group of labels.
#[derive(Serialize, ToSchema)]
pub(crate) struct LabelGroupBody {
    /// The ID of the group.
    pub(super) id: String,
    /// What it is called.
    pub(super) name: String,
    /// Whether an issue has at most one label of the group.
    pub(super) pick_one: bool,
    /// Whether it is retired: old issues keep its labels, new ones are not offered them.
    pub(super) retired: bool,
    /// Where it was made: the whole organization or a project. It is looked after where it was
    /// made.
    pub(super) created_in: PlaceRef,
    /// Whether the person who asks looks after it.
    pub(super) may_manage: bool,
    /// Counts up with every change.
    pub(super) version: i64,
    /// Whether the project the list is asked about offers it. Only when the request names a
    /// `project`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) offered_here: Option<bool>,
    /// The row that says so at exactly that project; left out while the project follows the place
    /// above it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) own_offer: Option<bool>,
    /// Whether the person who asks may change that for the project.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) may_change_offer: Option<bool>,
}

impl Masked for LabelGroupBody {}

impl From<&LabelGroupView> for LabelGroupBody {
    fn from(group: &LabelGroupView) -> Self {
        Self {
            id: group.id.to_string(),
            name: group.name.clone(),
            pick_one: group.pick_one,
            retired: group.retired,
            created_in: PlaceRef::from(group.created_in.as_ref()),
            may_manage: group.may_manage,
            version: group.version,
            offered_here: group.here.as_ref().map(|here| here.offered),
            own_offer: group.here.as_ref().and_then(|here| here.own),
            may_change_offer: group.here.as_ref().map(|here| here.may_change),
        }
    }
}

/// A label of the organization's list.
#[derive(Serialize, ToSchema)]
pub(crate) struct LabelBody {
    /// The ID of the label.
    pub(super) id: String,
    /// The group it is in, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) group: Option<String>,
    /// What it is called.
    pub(super) name: String,
    /// Its color, as a category token of the theme.
    pub(super) color: ChoiceColor,
    /// Whether it is retired: old issues keep it, new ones are not offered it.
    pub(super) retired: bool,
    /// Counts up with every change.
    pub(super) version: i64,
}

impl Masked for LabelBody {}

impl From<&LabelView> for LabelBody {
    fn from(label: &LabelView) -> Self {
        Self {
            id: label.id.to_string(),
            group: label.group.map(|group| group.to_string()),
            name: label.name.clone(),
            color: label.color,
            retired: label.retired,
            version: label.version,
        }
    }
}

/// The labels of the organization with their groups.
#[derive(Serialize, ToSchema)]
pub(crate) struct LabelsBody {
    /// The groups, in the order they are offered in.
    pub(super) groups: Vec<LabelGroupBody>,
    /// The labels, those in a group and those in none.
    pub(super) labels: Vec<LabelBody>,
    /// Whether the person looks after labels and tags.
    pub(super) may_manage: bool,
}

impl Masked for LabelsBody {}

impl From<Labels> for LabelsBody {
    fn from(labels: Labels) -> Self {
        Self {
            groups: labels.groups.iter().map(LabelGroupBody::from).collect(),
            labels: labels.labels.iter().map(LabelBody::from).collect(),
            may_manage: labels.may_manage,
        }
    }
}

/// A tag to put on an issue.
#[derive(Deserialize, ToSchema)]
pub(crate) struct NewTagBody {
    /// The name of the tag. A name nobody used makes the tag.
    pub(super) name: String,
}

/// A label to put on an issue.
#[derive(Deserialize, ToSchema)]
pub(crate) struct NewIssueLabelBody {
    /// The ID of the label.
    pub(super) label: String,
}

/// What to change about a tag: what is left out stays as it is.
#[derive(Deserialize, ToSchema)]
pub(crate) struct ChangeTagBody {
    /// The version the person looked at.
    pub(super) version: i64,
    /// A new name. Every issue that has the tag records it.
    pub(super) name: Option<String>,
    /// A new color.
    pub(super) color: Option<ChoiceColor>,
    /// The IDs of the projects the tag is limited to from now on; an empty list makes it a tag for
    /// every project.
    pub(super) projects: Option<Vec<String>>,
}

/// The tag that takes the issues of another.
#[derive(Deserialize, ToSchema)]
pub(crate) struct CombineBody {
    /// The ID of the tag or label that stays.
    pub(super) into: String,
    /// The version of the tag or label that goes away, as the person looked at it.
    pub(super) version: i64,
    /// The version of the one that stays, as the person looked at it.
    pub(super) into_version: i64,
}

/// What to preview: a new name, or the one it would be combined with.
#[derive(Deserialize, ToSchema)]
pub(crate) struct PreviewChangeBody {
    /// The new name, when it is renamed.
    pub(super) name: Option<String>,
    /// The ID of the tag or label it would be combined with.
    pub(super) into: Option<String>,
}

/// A group of labels to make.
#[derive(Deserialize, ToSchema)]
pub(crate) struct NewLabelGroupBody {
    /// The project it is made in, by its ID or short name; left out for the whole organization.
    pub(super) project: Option<String>,
    /// What it is called.
    pub(super) name: String,
    /// Whether an issue has at most one label of the group.
    pub(super) pick_one: bool,
}

/// What to change about a group of labels: what is left out stays as it is.
#[derive(Deserialize, ToSchema)]
pub(crate) struct ChangeLabelGroupBody {
    /// The version the person looked at.
    pub(super) version: i64,
    /// A new name.
    pub(super) name: Option<String>,
    /// Whether an issue has at most one label of the group from now on.
    pub(super) pick_one: Option<bool>,
    /// `true` retires the group, `false` offers it again.
    pub(super) retired: Option<bool>,
}

/// A label to make.
#[derive(Deserialize, ToSchema)]
pub(crate) struct NewLabelBody {
    /// What it is called.
    pub(super) name: String,
    /// Its color; the next in turn when left out.
    pub(super) color: Option<ChoiceColor>,
    /// The ID of the group it goes into.
    pub(super) group: Option<String>,
}

/// What to change about a label: what is left out stays as it is.
#[derive(Deserialize, ToSchema)]
pub(crate) struct ChangeLabelBody {
    /// The version the person looked at.
    pub(super) version: i64,
    /// A new name. Every issue that has the label records it.
    pub(super) name: Option<String>,
    /// A new color.
    pub(super) color: Option<ChoiceColor>,
    /// The ID of a new group; `null` takes the label out of its group.
    #[serde(default, deserialize_with = "crate::api::null_or_value")]
    #[schema(value_type = Option<String>)]
    pub(super) group: Edit<String>,
    /// `true` retires the label, `false` offers it again.
    pub(super) retired: Option<bool>,
}

//! What the workflow routes take and give back (WF-01…WF-04, ADR-0031): the rules of a status
//! change, the statuses and status changes an editor sends, and what the editor shows.

use bb_app::workflows::{
    EditorChange, EditorGroup, EditorRole, PublishPreview, RuleField, Tried, TriedChange,
    TriedVerdict, WorkflowEditor, WorkflowList, WorkflowListItem, WorkflowSlot,
};
use bb_core::ErrorCode;
use bb_core::fields::FieldKey;
use bb_core::id::{GroupId, RoleId};
use bb_core::issue::StatusGroup;
use bb_core::workflow::ask_may_name;
use bb_core::workflow::judge::Why;
use bb_core::workflow::{
    ChangeDraft, Condition, StatusDraft, StatusKey, StatusRules, Test, WhoCan,
};
use bb_policy::Masked;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::{StatusRef, status_in};
use crate::api::issues::IssueTypeRef;
use crate::api::layouts::InheritedFromBody;
use crate::api::projects::ProjectRef;
use crate::api::relations::IssueRef;
use crate::api::{body_id, path_id};
use crate::errors::ApiError;

/// *Who can do it*: the people who may make a status change. Naming nobody means everybody who may
/// change the issue; a rule never gives a right *Change anybody's issues* does not.
#[derive(Clone, Serialize, Deserialize, ToSchema)]
pub(crate) struct WhoCanBody {
    /// The IDs of the roles.
    #[serde(default)]
    roles: Vec<String>,
    /// The IDs of the groups.
    #[serde(default)]
    groups: Vec<String>,
    /// The people the issue is assigned to.
    #[serde(default)]
    assigned: bool,
    /// The person who reported the issue.
    #[serde(default)]
    reported_by: bool,
}

/// One condition of *Only if …*, in the same words as *Must be filled in*.
#[derive(Clone, Serialize, Deserialize, ToSchema)]
pub(crate) struct OnlyIfRuleBody {
    /// The key of the field: the word of a built-in field or the ID of an own field.
    field: String,
    /// What it must be like.
    test: Test,
}

/// The rules of a status change.
#[derive(Clone, Serialize, Deserialize, ToSchema)]
pub(crate) struct StatusRulesBody {
    /// *Who can do it*.
    #[serde(default)]
    who: WhoCanBody,
    /// *Only if …*: every condition must hold.
    #[serde(default)]
    only_if: Vec<OnlyIfRuleBody>,
    /// *Must be filled in*: the keys of the fields that need a value first.
    #[serde(default)]
    must_fill: Vec<String>,
    /// *Ask for*: the keys of the fields shown in a small form during the change.
    #[serde(default)]
    ask_for: Vec<String>,
}

impl Default for WhoCanBody {
    fn default() -> Self {
        Self::from(&WhoCan::default())
    }
}

impl From<&WhoCan> for WhoCanBody {
    fn from(who: &WhoCan) -> Self {
        Self {
            roles: who.roles.iter().map(ToString::to_string).collect(),
            groups: who.groups.iter().map(ToString::to_string).collect(),
            assigned: who.assigned,
            reported_by: who.reporter,
        }
    }
}

impl From<&StatusRules> for StatusRulesBody {
    fn from(rules: &StatusRules) -> Self {
        Self {
            who: WhoCanBody::from(&rules.who),
            only_if: rules
                .only_if
                .iter()
                .map(|condition| OnlyIfRuleBody {
                    field: condition.field.code().into_owned(),
                    test: condition.test,
                })
                .collect(),
            must_fill: rules.must_fill.iter().map(ToString::to_string).collect(),
            ask_for: rules.ask_for.iter().map(ToString::to_string).collect(),
        }
    }
}

fn field_in(text: &str) -> Result<FieldKey, ApiError> {
    FieldKey::from_code(text).ok_or_else(|| ApiError::new(ErrorCode::InvalidRequest))
}

impl StatusRulesBody {
    /// The rules as the use cases take them.
    fn rules(&self) -> Result<StatusRules, ApiError> {
        Ok(StatusRules {
            who: WhoCan {
                roles: self
                    .who
                    .roles
                    .iter()
                    .map(|text| body_id::<RoleId>(text))
                    .collect::<Result<_, _>>()?,
                groups: self
                    .who
                    .groups
                    .iter()
                    .map(|text| body_id::<GroupId>(text))
                    .collect::<Result<_, _>>()?,
                assigned: self.who.assigned,
                reporter: self.who.reported_by,
            },
            only_if: self
                .only_if
                .iter()
                .map(|condition| {
                    Ok(Condition {
                        field: field_in(&condition.field)?,
                        test: condition.test,
                    })
                })
                .collect::<Result<_, ApiError>>()?,
            must_fill: self
                .must_fill
                .iter()
                .map(|text| field_in(text))
                .collect::<Result<_, _>>()?,
            ask_for: self
                .ask_for
                .iter()
                .map(|text| field_in(text))
                .collect::<Result<_, _>>()?,
        })
    }
}

/// A status as the editor sends it.
#[derive(Deserialize, ToSchema)]
pub(crate) struct StatusDraftBody {
    /// The key of a status the workflow has. Left out for a new status, which gets a key made from
    /// its name; a status change names a new status by its name.
    key: Option<String>,
    /// What it is called.
    name: String,
    /// The status group it belongs to.
    group: StatusGroup,
}

impl StatusDraftBody {
    pub(super) fn draft(&self) -> Result<StatusDraft, ApiError> {
        Ok(StatusDraft {
            key: status_in(self.key.as_deref())?,
            name: self.name.clone(),
            group: self.group,
        })
    }
}

/// A status change as the editor sends it.
#[derive(Deserialize, ToSchema)]
pub(crate) struct ChangeDraftBody {
    /// The status it starts from: a key, or the name of a new status.
    from: String,
    /// The status it ends in: a key, or the name of a new status.
    to: String,
    /// The text of the button.
    button: String,
    /// Its rules.
    #[serde(default)]
    rules: StatusRulesBody,
}

impl Default for StatusRulesBody {
    fn default() -> Self {
        Self::from(&StatusRules::default())
    }
}

impl ChangeDraftBody {
    pub(super) fn draft(&self) -> Result<ChangeDraft, ApiError> {
        Ok(ChangeDraft {
            from: self.from.clone(),
            to: self.to.clone(),
            button: self.button.clone(),
            rules: self.rules.rules()?,
        })
    }
}

/// Where the issues of a dropped status go.
#[derive(Deserialize, ToSchema)]
pub(crate) struct StatusMoveBody {
    /// The key of the status that is dropped.
    from: String,
    /// The status of the new workflow the issues go to: its key, or its name when it is new.
    to: String,
}

impl StatusMoveBody {
    pub(super) fn parts(&self) -> Result<(StatusKey, String), ApiError> {
        Ok((body_id(&self.from)?, self.to.clone()))
    }
}

/// The workflow to publish.
#[derive(Deserialize, ToSchema)]
pub(crate) struct PublishWorkflowBody {
    /// The version the person looked at.
    pub(super) version: i64,
    /// The token the edit lock was taken with.
    pub(super) lock_token: String,
    /// What the workflow is called.
    pub(super) name: String,
    /// All the statuses, in order. The first is where a new issue starts.
    pub(super) statuses: Vec<StatusDraftBody>,
    /// All the status changes. They replace the ones that are stored.
    pub(super) changes: Vec<ChangeDraftBody>,
    /// Where the issues of a dropped status go. Needed for each dropped status that issues are
    /// still in; nothing is published without it.
    #[serde(default)]
    pub(super) moves: Vec<StatusMoveBody>,
}

/// What to preview: the workflow as *Publish* would send it.
#[derive(Deserialize, ToSchema)]
pub(crate) struct PublishPreviewBody {
    /// All the statuses, in order.
    pub(super) statuses: Vec<StatusDraftBody>,
    /// All the status changes.
    pub(super) changes: Vec<ChangeDraftBody>,
}

/// A status that publishing would drop, and the issues that are in it.
#[derive(Serialize, ToSchema)]
pub(crate) struct DroppedStatusBody {
    /// The status, as the stored workflow names it.
    status: StatusRef,
    /// How many live issues the workflow governs that are in it, also the ones the person does not
    /// see.
    issues: usize,
    /// Some of the issues the person sees, as a list shows them.
    named: Vec<IssueRef>,
    /// How many of the issues the person does not see. They are counted, never named.
    unseen: usize,
    /// How many of the issues the person could not move: those they do not see, and those they may
    /// not change. A publish that has to move them is refused.
    blocked: usize,
}

/// What publishing a workflow would drop.
#[derive(Serialize, ToSchema)]
pub(crate) struct PublishPreviewAnswer {
    /// The statuses the stored workflow has and the new one lacks.
    dropped: Vec<DroppedStatusBody>,
    /// The most issues one publish moves; a publish that would move more is refused.
    limit: usize,
    /// There are more issues than that in the statuses that would go. They are then not judged or
    /// named, and the counts are not complete.
    too_many: bool,
}

impl Masked for PublishPreviewAnswer {}

impl From<&PublishPreview> for PublishPreviewAnswer {
    fn from(preview: &PublishPreview) -> Self {
        Self {
            limit: preview.limit,
            too_many: preview.too_many,
            dropped: preview
                .dropped
                .iter()
                .map(|each| DroppedStatusBody {
                    status: StatusRef::from(&each.status),
                    issues: each.issues,
                    named: each.named.iter().map(IssueRef::from).collect(),
                    unseen: each.unseen,
                    blocked: each.blocked,
                })
                .collect(),
        }
    }
}

/// The workflow set at exactly the place that was asked about.
#[derive(Serialize, ToSchema)]
pub(crate) struct WorkflowSlotBody {
    /// The ID of the workflow, which the edit lock is about.
    id: String,
    /// Counts up with every change. A change is made on the version that was looked at.
    version: i64,
}

impl From<&WorkflowSlot> for WorkflowSlotBody {
    fn from(slot: &WorkflowSlot) -> Self {
        Self {
            id: slot.id.to_string(),
            version: slot.version,
        }
    }
}

/// A status change in the editor.
#[derive(Serialize, ToSchema)]
pub(crate) struct EditorChangeBody {
    /// The key of the status it starts from.
    from: String,
    /// The key of the status it ends in.
    to: String,
    /// The text of the button.
    button: String,
    /// Its rules.
    rules: StatusRulesBody,
}

impl From<&EditorChange> for EditorChangeBody {
    fn from(change: &EditorChange) -> Self {
        Self {
            from: change.from.to_string(),
            to: change.to.to_string(),
            button: change.button.clone(),
            rules: StatusRulesBody::from(&change.rules),
        }
    }
}

/// A role or a group a rule can name.
#[derive(Serialize, ToSchema)]
pub(crate) struct NamedBody {
    /// The ID.
    id: String,
    /// The name.
    name: String,
}

impl From<&EditorRole> for NamedBody {
    fn from(role: &EditorRole) -> Self {
        Self {
            id: role.id.to_string(),
            name: role.name.clone(),
        }
    }
}

impl From<&EditorGroup> for NamedBody {
    fn from(group: &EditorGroup) -> Self {
        Self {
            id: group.id.to_string(),
            name: group.name.clone(),
        }
    }
}

/// A field a rule can name.
#[derive(Serialize, ToSchema)]
pub(crate) struct RuleFieldBody {
    /// The key of the field: the word of a built-in field or the ID of an own field.
    field: String,
    /// The name of an own field; a built-in field is named by its word.
    name: Option<String>,
    /// Whether a rule may ask for the field in the small form of a status change; the description
    /// can only be required to be filled in.
    askable: bool,
}

impl From<&RuleField> for RuleFieldBody {
    fn from(field: &RuleField) -> Self {
        Self {
            field: field.field.code().into_owned(),
            name: field.name.clone(),
            askable: ask_may_name(field.field),
        }
    }
}

/// What the editor shows for a place.
#[derive(Serialize, ToSchema)]
pub(crate) struct WorkflowEditorBody {
    /// The project of the place; none for the whole organization.
    project: Option<ProjectRef>,
    /// The issue type of the place; none for all types.
    issue_type: Option<IssueTypeRef>,
    /// The workflow set at this place. Without it the place uses the workflow it inherits (or the
    /// starting workflow), and *Create* makes one.
    workflow: Option<WorkflowSlotBody>,
    /// Where the workflow that counts is set, when it is another place.
    inherited_from: Option<InheritedFromBody>,
    /// What the workflow is called.
    name: String,
    /// Its statuses, in order.
    statuses: Vec<StatusRef>,
    /// Its status changes.
    changes: Vec<EditorChangeBody>,
    /// The roles *Who can do it* can name.
    roles: Vec<NamedBody>,
    /// The groups *Who can do it* can name.
    groups: Vec<NamedBody>,
    /// The fields the rules can name.
    fields: Vec<RuleFieldBody>,
}

impl Masked for WorkflowEditorBody {}

impl From<&WorkflowEditor> for WorkflowEditorBody {
    fn from(editor: &WorkflowEditor) -> Self {
        Self {
            project: editor.project.as_ref().map(ProjectRef::from),
            issue_type: editor.issue_type.as_ref().map(IssueTypeRef::from),
            workflow: editor.workflow.as_ref().map(WorkflowSlotBody::from),
            inherited_from: editor.inherited_from.as_ref().map(InheritedFromBody::from),
            name: editor.name.clone(),
            statuses: editor.statuses.iter().map(StatusRef::from).collect(),
            changes: editor.changes.iter().map(EditorChangeBody::from).collect(),
            roles: editor.roles.iter().map(NamedBody::from).collect(),
            groups: editor.groups.iter().map(NamedBody::from).collect(),
            fields: editor.fields.iter().map(RuleFieldBody::from).collect(),
        }
    }
}

/// A workflow in the list.
#[derive(Serialize, ToSchema)]
pub(crate) struct WorkflowChoiceBody {
    /// The ID of the workflow.
    id: String,
    /// What it is called.
    name: String,
    /// Counts up with every change.
    version: i64,
}

/// The workflows there are.
#[derive(Serialize, ToSchema)]
pub(crate) struct WorkflowListBody {
    /// The workflows, by name.
    workflows: Vec<WorkflowChoiceBody>,
    /// The list was cut off at its cap.
    truncated: bool,
}

impl Masked for WorkflowListBody {}

impl From<&WorkflowList> for WorkflowListBody {
    fn from(list: &WorkflowList) -> Self {
        Self {
            truncated: list.truncated,
            workflows: list
                .workflows
                .iter()
                .map(
                    |WorkflowListItem { id, name, version }| WorkflowChoiceBody {
                        id: id.to_string(),
                        name: name.clone(),
                        version: *version,
                    },
                )
                .collect(),
        }
    }
}

/// What *Publish* did.
#[derive(Serialize, ToSchema)]
pub(crate) struct PublishedBody {
    /// The workflow, with its new version.
    workflow: WorkflowSlotBody,
}

impl Masked for PublishedBody {}

impl From<&bb_app::workflows::Published> for PublishedBody {
    fn from(published: &bb_app::workflows::Published) -> Self {
        Self {
            workflow: WorkflowSlotBody::from(&published.workflow),
        }
    }
}

/// What the judge says about one status change.
#[derive(Serialize, ToSchema)]
#[serde(tag = "verdict", rename_all = "snake_case")]
pub(crate) enum TriedVerdictBody {
    /// The person may make it.
    Allowed,
    /// The person may make it once these fields have a value.
    Needs {
        /// The keys of the fields.
        fields: Vec<String>,
    },
    /// The person may not make it.
    NotAllowed {
        /// Why.
        why: Why,
        /// The key of the field it is about, when the person who tries may see it too.
        field: Option<String>,
    },
}

impl From<&TriedVerdict> for TriedVerdictBody {
    fn from(verdict: &TriedVerdict) -> Self {
        match verdict {
            TriedVerdict::Allowed => Self::Allowed,
            TriedVerdict::Needs { fields } => Self::Needs {
                fields: fields.iter().map(ToString::to_string).collect(),
            },
            TriedVerdict::NotAllowed { why, field } => Self::NotAllowed {
                why: *why,
                field: field.as_ref().map(ToString::to_string),
            },
        }
    }
}

/// One status change *Try it* looked at.
#[derive(Serialize, ToSchema)]
pub(crate) struct TriedChangeBody {
    /// The status it ends in.
    to: StatusRef,
    /// The text of the button.
    button: String,
    /// What the judge says.
    #[serde(flatten)]
    verdict: TriedVerdictBody,
}

impl From<&TriedChange> for TriedChangeBody {
    fn from(change: &TriedChange) -> Self {
        Self {
            to: StatusRef::from(&change.to),
            button: change.button.clone(),
            verdict: TriedVerdictBody::from(&change.verdict),
        }
    }
}

/// What *Try it* found.
#[derive(Serialize, ToSchema)]
pub(crate) struct TriedBody {
    /// The status the issue is in.
    status: StatusRef,
    /// The status changes from there, each with the answer for the person.
    changes: Vec<TriedChangeBody>,
}

impl Masked for TriedBody {}

impl From<&Tried> for TriedBody {
    fn from(tried: &Tried) -> Self {
        Self {
            status: StatusRef::from(&tried.status),
            changes: tried.changes.iter().map(TriedChangeBody::from).collect(),
        }
    }
}

/// The workflow in a path.
pub(super) fn workflow_in_path(text: &str) -> Result<bb_core::id::WorkflowId, ApiError> {
    path_id(text)
}

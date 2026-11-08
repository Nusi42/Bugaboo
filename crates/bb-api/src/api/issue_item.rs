//! An issue as an answer holds it (ISS-01, PERF-02): the fields a list can ask for and the item
//! with every field the person may see, shared by the issue routes, the search and the fields.

use bb_app::issues::{AgentMark, IssueTypeRef as AppIssueTypeRef, IssueView};
use bb_core::agent_work::AgentWorkState;
use bb_core::issue::{IssueIcon, Priority};
use bb_core::own_fields_value::OwnValues;
use bb_core::time::{Minutes, TimeLeft};
use bb_policy::{Masked, PersonRef};
use serde::Serialize;
use utoipa::ToSchema;

use super::projects::ProjectRef;
use super::tags::LabelRefBody;
use super::workflows::{StatusChangeRef, StatusRef};
use crate::lean::{FieldSpec, ListParams};

/// The fields of an issue in a list that can be asked for, and which of them a list holds when the
/// request names none: what a row of a list needs.
pub(crate) const ISSUE_FIELDS: [FieldSpec; 22] = [
    FieldSpec::by_default("id"),
    FieldSpec::by_default("readable_id"),
    FieldSpec::on_request("project"),
    FieldSpec::by_default("issue_type"),
    FieldSpec::by_default("title"),
    FieldSpec::on_request("description"),
    FieldSpec::by_default("status"),
    FieldSpec::on_request("status_changes"),
    FieldSpec::by_default("priority"),
    FieldSpec::on_request("estimated_time"),
    FieldSpec::on_request("time_left"),
    FieldSpec::on_request("time_left_worked_out"),
    FieldSpec::on_request("time_spent"),
    FieldSpec::on_request("reported_by"),
    FieldSpec::on_request("assigned_to"),
    FieldSpec::on_request("created_at"),
    FieldSpec::on_request("updated_at"),
    FieldSpec::on_request("closed_at"),
    FieldSpec::on_request("own_fields"),
    FieldSpec::on_request("labels"),
    FieldSpec::on_request("agent_work"),
    FieldSpec::on_request("version"),
];
/// An issue type as a small reference: what the line of an issue needs.
#[derive(Serialize, ToSchema, Masked)]
pub(crate) struct IssueTypeRef {
    /// The ID of the type.
    #[masked(open)]
    pub(crate) id: String,
    /// What it is called.
    #[masked(open)]
    pub(crate) name: String,
    /// Its icon.
    #[masked(open)]
    pub(crate) icon: IssueIcon,
    /// Whether it is retired: old issues keep it, new ones cannot have it.
    #[masked(open)]
    pub(crate) retired: bool,
}

impl From<&AppIssueTypeRef> for IssueTypeRef {
    fn from(issue_type: &AppIssueTypeRef) -> Self {
        Self {
            id: issue_type.id.to_string(),
            name: issue_type.name.clone(),
            icon: issue_type.icon,
            retired: issue_type.retired,
        }
    }
}

/// The agent that works on an issue and where its work is: the mark of a row of a list.
#[derive(Serialize, ToSchema, Masked)]
pub(crate) struct AgentMarkBody {
    /// The name of the agent.
    #[masked(open)]
    pub(crate) name: String,
    /// Where its work is. It is not the status of the issue.
    #[masked(open)]
    pub(crate) state: AgentWorkState,
}

impl From<&AgentMark> for AgentMarkBody {
    fn from(mark: &AgentMark) -> Self {
        Self {
            name: mark.name.clone(),
            state: mark.state,
        }
    }
}

/// An issue: the fields that can be asked for. An answer holds only those that were asked for (a
/// list holds a few by default; one issue holds all) and that the person may see: a field that is
/// hidden from them is left out, and asking for it fails like asking for one that does not exist.
#[derive(Serialize, ToSchema, Masked)]
pub(crate) struct IssueItem {
    /// The internal ID, which never changes.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(open)]
    id: Option<String>,
    /// The readable ID now (`WEB-142`, `WEB-K7Q2X`).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(open)]
    readable_id: Option<String>,
    /// The project it is in.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(nested)]
    project: Option<ProjectRef>,
    /// Its issue type. The levels set for it count for the fields of this issue.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(nested, item_type)]
    issue_type: Option<IssueTypeRef>,
    /// What it is about.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(field = "title")]
    title: Option<String>,
    /// The long text, in Markdown.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(field = "description")]
    description: Option<String>,
    /// Where it stands in its workflow: the key, the name and the status group.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(field = "status")]
    status: Option<StatusRef>,
    /// The status changes the person may make now, with the text of each button and the fields it
    /// still needs. Empty when they may not change the issue. Hidden with the status.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(field = "status")]
    status_changes: Option<Vec<StatusChangeRef>>,
    /// How important it is.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(field = "priority")]
    priority: Option<Priority>,
    /// *Estimated time*, in whole minutes; left out while it has none.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(field = "estimated-time")]
    #[schema(value_type = Option<u32>)]
    estimated_time: Option<Minutes>,
    /// *Time left* in whole minutes, typed or worked out from the estimate and the time spent;
    /// left out without either. Worked out only in a list that asks for it: the answer of one issue
    /// holds only a typed value, and one issue reads its worked-out one through `/time`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(field = "time-left")]
    #[schema(value_type = Option<u32>)]
    time_left: Option<Minutes>,
    /// Whether *Time left* is worked out rather than typed (shown in the quiet style of a value
    /// worked out); left out with it. Only in a list; one issue reads it through `/time`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(field = "time-left")]
    time_left_worked_out: Option<bool>,
    /// *Time spent*: the total of the issue's time entries, in whole minutes. Only in a list that
    /// asks for it; one issue reads it through `/time`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(field = "time-spent")]
    #[schema(value_type = Option<u32>)]
    time_spent: Option<Minutes>,
    /// Who made it, as the viewer may see people of this kind.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(field = "reported-by", nested)]
    reported_by: Option<PersonRef>,
    /// Who works on it, as the viewer may see people of this kind.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(field = "assigned-to", nested)]
    assigned_to: Option<Vec<PersonRef>>,
    /// When it was made.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(open)]
    created_at: Option<String>,
    /// When it was last changed.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(open)]
    updated_at: Option<String>,
    /// When it entered the group *Done*, while it is in it. Hidden with the status.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(field = "status")]
    closed_at: Option<String>,
    /// The values of its own fields, by the key of the field. Only the fields the person may see
    /// on an issue of this type.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(own_fields)]
    own_fields: Option<OwnValues>,
    /// The labels on it, in the order they were put there. Hidden from a reader whose level for
    /// *Labels* is *Hidden*.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(field = "labels")]
    labels: Option<Vec<LabelRefBody>>,
    /// The agent that works on it now and where its work is; left out while no agent works on it.
    /// Only a list that asks for it reads it.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(nested, item_type)]
    agent_work: Option<AgentMarkBody>,
    /// Counts up with every change. A change is made on the version that was looked at.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[masked(open)]
    version: Option<i64>,
}

impl From<&IssueView> for IssueItem {
    fn from(issue: &IssueView) -> Self {
        Self {
            id: Some(issue.id.to_string()),
            readable_id: Some(issue.key.clone()),
            project: Some(ProjectRef::from(&issue.project)),
            issue_type: Some(IssueTypeRef::from(&issue.issue_type)),
            title: Some(issue.title.clone()),
            description: Some(issue.description.clone()),
            status: Some(StatusRef::from(&issue.status)),
            status_changes: Some(
                issue
                    .status_changes
                    .iter()
                    .map(StatusChangeRef::from)
                    .collect(),
            ),
            priority: Some(issue.priority),
            estimated_time: issue.estimated_time,
            time_left: issue.time_left.map(TimeLeft::minutes),
            time_left_worked_out: issue
                .time_left
                .map(|left| matches!(left, TimeLeft::WorkedOut(_))),
            time_spent: issue.time_spent,
            reported_by: Some(issue.reported_by.clone()),
            assigned_to: Some(issue.assigned_to.clone()),
            created_at: Some(issue.created_at.to_string()),
            updated_at: Some(issue.updated_at.to_string()),
            closed_at: issue.closed_at.map(|at| at.to_string()),
            own_fields: Some(issue.own_fields.clone()),
            labels: issue
                .labels
                .as_ref()
                .map(|labels| labels.iter().map(LabelRefBody::from).collect()),
            agent_work: issue.agent_work.as_ref().map(AgentMarkBody::from),
            version: Some(issue.version),
        }
    }
}

/// The members of an item that need the minutes spent on each issue of the page.
const TIME_MEMBERS: [&str; 3] = ["time_spent", "time_left", "time_left_worked_out"];

/// Whether a list asks for a member that needs the minutes spent on each issue of the page:
/// *Time spent*, *Time left* or whether it is worked out.
pub(crate) fn asks_for_time(params: &ListParams) -> bool {
    TIME_MEMBERS
        .iter()
        .any(|member| params.asks_for(&ISSUE_FIELDS, member))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_03_each_member_of_time_asks_for_the_minutes_spent() {
        let asked = |fields: &str| -> ListParams {
            serde_json::from_value(serde_json::json!({ "fields": fields })).unwrap()
        };
        for member in TIME_MEMBERS {
            assert!(asks_for_time(&asked(&format!("id,{member}"))), "{member}");
        }
        assert!(!asks_for_time(&asked("id,title")));
    }

    #[test]
    fn perf_02_the_field_table_names_exactly_the_fields_of_an_issue() {
        let item = IssueItem {
            id: Some(String::new()),
            readable_id: Some(String::new()),
            project: Some(ProjectRef {
                id: String::new(),
                short_name: String::new(),
                name: String::new(),
            }),
            issue_type: Some(IssueTypeRef {
                id: String::new(),
                name: String::new(),
                icon: IssueIcon::Bug,
                retired: false,
            }),
            title: Some(String::new()),
            description: Some(String::new()),
            status: Some(StatusRef::blank()),
            status_changes: Some(Vec::new()),
            priority: Some(Priority::Normal),
            estimated_time: Some(Minutes::new(0)),
            time_left: Some(Minutes::new(0)),
            time_left_worked_out: Some(false),
            time_spent: Some(Minutes::new(0)),
            reported_by: Some(PersonRef::Someone),
            assigned_to: Some(Vec::new()),
            created_at: Some(String::new()),
            updated_at: Some(String::new()),
            closed_at: Some(String::new()),
            own_fields: Some(OwnValues::default()),
            labels: Some(Vec::new()),
            agent_work: Some(AgentMarkBody {
                name: String::new(),
                state: AgentWorkState::AgentWorking,
            }),
            version: Some(1),
        };
        crate::lean::assert_fields_are_the_keys_of(&ISSUE_FIELDS, &item);
    }
}

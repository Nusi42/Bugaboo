//! What it takes to put the fields of an issue back: the change of a single issue that has the
//! values a run found, which `change_issue` then judges again as the person who undoes the run.

use std::collections::BTreeMap;

use bb_core::given::SetTo;
use bb_core::id::{IssueId, IssueTypeId, UserId};
use bb_core::issue::Priority;
use bb_core::time::length::Minutes;
use bb_core::workflow::StatusKey;
use serde_json::Value;

use super::weigh::Net;
use crate::issues::{ChangeIssue, member};

/// Whether the field is one that a change of an issue puts back.
pub(super) fn restorable(net: &Net) -> bool {
    let known = [
        member::TITLE,
        member::STATUS,
        member::PRIORITY,
        member::ESTIMATED_TIME,
        member::TIME_LEFT,
        member::ASSIGNED_TO,
        member::ISSUE_TYPE,
    ];
    let own = net.field.parse::<bb_core::id::FieldId>().is_ok();
    // A built-in field that had no value before cannot be put back to none; an own field can.
    (known.contains(&net.field.as_str()) && net.before.is_some()) || own
}

/// The change of the issue `issue` that puts `nets` back, on `version`: the version the plan looked
/// at, so a change made after the plan is a conflict and not overwritten. `None` when a value cannot
/// be read as what the field holds.
pub(super) fn issue_change(
    token: &str,
    issue: IssueId,
    version: i64,
    nets: &[Net],
) -> Option<ChangeIssue> {
    let mut input = ChangeIssue {
        session_token: token.to_owned(),
        issue: issue.to_string(),
        version,
        issue_type: None,
        title: None,
        status: None,
        priority: None,
        estimated_time: None,
        time_left: None,
        assigned_to: None,
        fields: BTreeMap::new(),
    };
    for net in nets {
        let before = net.before.clone();
        match net.field.as_str() {
            member::TITLE => input.title = Some(before?.as_str()?.to_owned()),
            member::STATUS => input.status = Some(before?.as_str()?.parse::<StatusKey>().ok()?),
            member::PRIORITY => {
                input.priority = Some(serde_json::from_value::<Priority>(before?).ok()?);
            }
            member::ESTIMATED_TIME => input.estimated_time = Some(length(before)?),
            member::TIME_LEFT => input.time_left = Some(length(before)?),
            member::ASSIGNED_TO => {
                let people = before?
                    .as_array()?
                    .iter()
                    .map(|each| each.as_str()?.parse::<UserId>().ok())
                    .collect::<Option<Vec<_>>>()?;
                input.assigned_to = Some(people);
            }
            member::ISSUE_TYPE => {
                input.issue_type = Some(before?.as_str()?.parse::<IssueTypeId>().ok()?);
            }
            own => {
                input
                    .fields
                    .insert(own.to_owned(), before.unwrap_or(Value::Null));
            }
        }
    }
    Some(input)
}

/// A length of time as an event holds it: minutes, or none.
fn length(value: Option<Value>) -> Option<SetTo<Minutes>> {
    match value {
        None | Some(Value::Null) => Some(SetTo::Nothing),
        Some(number) => Some(SetTo::Value(serde_json::from_value(number).ok()?)),
    }
}

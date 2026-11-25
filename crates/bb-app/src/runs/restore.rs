//! What it takes to put the fields of an issue back: the change of a single issue that has the
//! values a run found, which `change_issue` then judges again as the person who undoes the run.

use bb_core::given::SetTo;
use bb_core::id::{IssueId, IssueTypeId, ReleaseId, UserId};
use bb_core::issue::Priority;
use bb_core::time::length::Minutes;
use bb_core::workflow::StatusKey;
use jiff::civil::Date;
use serde_json::Value;

use super::weigh::Net;
use crate::issues::{AskedBy, ChangeIssue, member};

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
    let date = [
        member::START_DATE,
        member::DUE_DATE,
        member::FOUND_IN_RELEASE,
        member::PLANNED_FOR_RELEASE,
    ]
    .contains(&net.field.as_str());
    // A built-in field that had no value before cannot be put back to none; an own field, a date
    // and a release can (a date or a release a person set on an issue that had none is taken away
    // again).
    (known.contains(&net.field.as_str()) && net.before.is_some()) || own || date
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
        asked_by: AskedBy::Undo,
        ..ChangeIssue::of(token, issue.to_string(), version)
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
            member::START_DATE => input.start_date = Some(day(before)?),
            member::DUE_DATE => input.due_date = Some(day(before)?),
            member::FOUND_IN_RELEASE => input.found_in_release = Some(releases(before)?),
            member::PLANNED_FOR_RELEASE => input.planned_for_release = Some(release(before)?),
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

/// A day as an event holds it: its text (`2026-11-14`), or none.
fn day(value: Option<Value>) -> Option<SetTo<Date>> {
    match value {
        None | Some(Value::Null) => Some(SetTo::Nothing),
        Some(text) => Some(SetTo::Value(text.as_str()?.parse().ok()?)),
    }
}

/// The releases of *Found in release* as an event holds them (`{id, name, project}` each), or none.
fn releases(value: Option<Value>) -> Option<Vec<ReleaseId>> {
    match value {
        None | Some(Value::Null) => Some(Vec::new()),
        Some(items) => items
            .as_array()?
            .iter()
            .map(|item| item.get("id")?.as_str()?.parse().ok())
            .collect(),
    }
}

/// The release of *Planned for release* as an event holds it, or none.
fn release(value: Option<Value>) -> Option<SetTo<ReleaseId>> {
    match value {
        None | Some(Value::Null) => Some(SetTo::Nothing),
        Some(item) => Some(SetTo::Value(item.get("id")?.as_str()?.parse().ok()?)),
    }
}

/// A length of time as an event holds it: minutes, or none.
fn length(value: Option<Value>) -> Option<SetTo<Minutes>> {
    match value {
        None | Some(Value::Null) => Some(SetTo::Nothing),
        Some(number) => Some(SetTo::Value(serde_json::from_value(number).ok()?)),
    }
}

#[cfg(test)]
mod tests {
    use bb_core::event::{EntityRef, RecordKind};
    use bb_core::id::{IssueId, ReleaseId, SequentialIds};
    use serde_json::json;

    use super::*;

    #[test]
    fn iss_08_undo_of_a_run_puts_both_release_fields_back() {
        let mut ids = SequentialIds::default();
        let issue = IssueId::generate(&mut ids).unwrap();
        let (one, two) = (
            ReleaseId::generate(&mut ids).unwrap(),
            ReleaseId::generate(&mut ids).unwrap(),
        );
        let net = |field: &str, before: Option<Value>| Net {
            entity: EntityRef {
                kind: RecordKind::Issue.into(),
                id: issue.into(),
            },
            field: field.to_owned(),
            place: 0,
            before,
            after: Some(json!({"id": two.to_string()})),
        };
        let item = |id: ReleaseId| json!({"id": id.to_string(), "name": "2.4", "project": ""});
        let nets = [
            net(
                member::FOUND_IN_RELEASE,
                Some(json!([item(one), item(two)])),
            ),
            net(member::PLANNED_FOR_RELEASE, None),
        ];
        assert!(nets.iter().all(restorable));
        let change = issue_change("token", issue, 3, &nets).unwrap();
        assert_eq!(change.found_in_release, Some(vec![one, two]));
        assert_eq!(change.planned_for_release, Some(SetTo::Nothing));
        let back = issue_change(
            "token",
            issue,
            3,
            &[net(member::PLANNED_FOR_RELEASE, Some(item(one)))],
        )
        .unwrap();
        assert_eq!(back.planned_for_release, Some(SetTo::Value(one)));
    }
}

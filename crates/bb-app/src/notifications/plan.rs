//! What an event means for notifications, as far as that can be said without asking the store
//! (NTF-01, ADR-0033 §1): who was newly assigned, who is told of an edit lock, which kind of event
//! a change of a followed issue is. Pure; the reader (`build.rs`) asks the store and the policy
//! engine for the rest.

use bb_core::edit::NoticeKind;
use bb_core::event::FieldChange;
use bb_core::fields::Field;
use bb_core::id::UserId;
use bb_core::many_changes::ManyChangeStatus;
use bb_core::notification::{NotificationKind, Via};
use bb_core::settings::Layers;
use serde_json::Value;

use crate::issues::member as issue_member;
use crate::member::{edit_lock, run};

/// Whether a person in the layers they have wants to hear of `kind` this way. The organization's
/// value is the default of everybody and a locked one counts for everybody.
pub(crate) fn wants(layers: &Layers, kind: NotificationKind, via: Via) -> bool {
    layers.yes(kind.setting(via))
}

/// Where the event of a run of many changes says the run is now, if it says.
pub(crate) fn run_status_of(changes: &[FieldChange]) -> Option<ManyChangeStatus> {
    changes
        .iter()
        .find(|change| change.field == run::STATUS)
        .and_then(|change| change.after.as_ref()?.as_str())
        .and_then(ManyChangeStatus::from_code)
}

/// Whether the event says that giving a project's issues their new IDs gave up.
pub(crate) fn renewal_failed(changes: &[FieldChange]) -> bool {
    changes
        .iter()
        .any(|change| change.field == issue_member::ISSUE_IDS_FAILED)
}

/// The people an event assigned the issue to: in the members after and not in those before.
pub(crate) fn newly_assigned(changes: &[FieldChange]) -> Vec<UserId> {
    let people = |value: &Option<Value>| -> Vec<UserId> {
        value
            .as_ref()
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.as_str()?.parse().ok())
                    .collect()
            })
            .unwrap_or_default()
    };
    let Some(change) = changes
        .iter()
        .find(|change| change.field == Field::AssignedTo.code())
    else {
        return Vec::new();
    };
    let before = people(&change.before);
    people(&change.after)
        .into_iter()
        .filter(|person| !before.contains(person))
        .collect()
}

/// What an event of an edit lock says: what happened, to which part, and to whom.
pub(crate) struct EditNote {
    pub(crate) notice: NoticeKind,
    pub(crate) part: String,
    pub(crate) person: UserId,
}

/// The note of an event of an edit lock, if the event is one that tells a person (somebody was
/// asked to finish, or lost the editing to somebody).
pub(crate) fn edit_note_of(changes: &[FieldChange]) -> Option<EditNote> {
    let change = changes
        .iter()
        .find(|change| change.field == edit_lock::FIELD)?;
    let after = change.after.as_ref()?;
    let person = after.get(edit_lock::PERSON)?.as_str()?.parse().ok()?;
    let (notice, part) = [
        (NoticeKind::FinishAsked, edit_lock::ASKED_TO_FINISH),
        (NoticeKind::TakenOver, edit_lock::TAKEN_OVER),
    ]
    .into_iter()
    .find_map(|(notice, word)| Some((notice, after.get(word)?.as_str()?.to_owned())))?;
    Some(EditNote {
        notice,
        part,
        person,
    })
}

/// Which kind of notification a change to an issue that a person follows is, given the changes
/// they may see: a change of the status is *activity*, any other change is *fields*, and nothing
/// visible is no notification at all.
pub(crate) fn followed_kind(visible: &[&FieldChange]) -> Option<NotificationKind> {
    if visible.is_empty() {
        None
    } else if visible
        .iter()
        .any(|change| change.field == Field::Status.code())
    {
        Some(NotificationKind::FollowedActivity)
    } else {
        Some(NotificationKind::FollowedFields)
    }
}

/// The kinds a follower could be told a change as, before the fields they may see are known: the
/// kind of all the changes, and the kind of the changes without the status (a reader who may not
/// see the status). The one place that works out what [`followed_kind`] could say.
pub(crate) fn possible_followed_kinds(changes: &[FieldChange]) -> Vec<NotificationKind> {
    let all: Vec<&FieldChange> = changes.iter().collect();
    let without_status: Vec<&FieldChange> = changes
        .iter()
        .filter(|change| change.field != Field::Status.code())
        .collect();
    let mut kinds: Vec<NotificationKind> = [followed_kind(&all), followed_kind(&without_status)]
        .into_iter()
        .flatten()
        .collect();
    kinds.dedup();
    kinds
}

#[cfg(test)]
mod tests {
    use bb_core::id::SequentialIds;
    use serde_json::json;

    use super::*;

    fn change(field: &str, before: Option<Value>, after: Option<Value>) -> FieldChange {
        FieldChange {
            field: field.to_owned(),
            before,
            after,
        }
    }

    #[test]
    fn m3_f125_the_status_of_a_run_is_read_from_its_event_and_only_the_last_three_end_it() {
        let status = |word: &str| run_status_of(&[change("status", None, Some(json!(word)))]);
        assert_eq!(status("done"), Some(ManyChangeStatus::Done));
        for over in ["done", "failed", "stopped"] {
            assert!(
                status(over).is_some_and(ManyChangeStatus::is_over),
                "{over}"
            );
        }
        for going in ["queued", "running"] {
            assert!(
                !status(going).is_some_and(ManyChangeStatus::is_over),
                "{going}"
            );
        }
        assert_eq!(status("nonsense"), None);
        assert_eq!(
            run_status_of(&[change("title", None, Some(json!("done")))]),
            None
        );
    }

    #[test]
    fn ntf_01_only_the_people_added_to_assigned_to_are_newly_assigned() {
        let mut ids = SequentialIds::default();
        let (ana, ben) = (UserId::generate(&mut ids), UserId::generate(&mut ids));
        let both = json!([ana.to_string(), ben.to_string()]);
        assert_eq!(
            newly_assigned(&[change(
                "assigned-to",
                Some(json!([ana.to_string()])),
                Some(both.clone())
            )]),
            [ben]
        );
        // A new issue has no value before.
        assert_eq!(
            newly_assigned(&[change("assigned-to", None, Some(both))]),
            [ana, ben]
        );
        assert!(newly_assigned(&[change("status", None, Some(json!("done")))]).is_empty());
        assert!(
            newly_assigned(&[change(
                "assigned-to",
                Some(json!([ana.to_string()])),
                Some(json!([]))
            )])
            .is_empty()
        );
    }

    #[test]
    fn ntf_01_a_change_of_the_status_is_activity_and_any_other_change_is_fields() {
        let status = change("status", None, Some(json!("done")));
        let title = change("title", None, Some(json!("A")));
        assert_eq!(
            followed_kind(&[&title, &status]),
            Some(NotificationKind::FollowedActivity)
        );
        assert_eq!(
            followed_kind(&[&title]),
            Some(NotificationKind::FollowedFields)
        );
        assert_eq!(followed_kind(&[]), None);
    }

    #[test]
    fn ntf_01_the_person_of_an_edit_lock_event_is_the_one_told() {
        let person = UserId::generate(&mut SequentialIds::default());
        let asked = change(
            edit_lock::FIELD,
            None,
            Some(json!({
                edit_lock::ASKED_TO_FINISH: "description",
                edit_lock::PERSON: person.to_string(),
            })),
        );
        let note = edit_note_of(&[asked]).unwrap();
        assert_eq!(
            (note.notice, note.part.as_str(), note.person),
            (NoticeKind::FinishAsked, "description", person)
        );
        let taken = change(
            edit_lock::FIELD,
            None,
            Some(json!({ edit_lock::TAKEN_OVER: "text", edit_lock::PERSON: person.to_string() })),
        );
        assert_eq!(
            edit_note_of(&[taken]).unwrap().notice,
            NoticeKind::TakenOver
        );
        // Nobody was told: the lock was free.
        let nobody = change(
            edit_lock::FIELD,
            None,
            Some(json!({ edit_lock::TAKEN_OVER: "text" })),
        );
        assert!(edit_note_of(&[nobody]).is_none());
    }
}

//! The members of the events that the large dataset writes without the use cases (architecture §11, *Large
//! dataset*). The dataset does not go through the use cases, so it cannot ask them for an event;
//! the use cases build their events with these same functions, so an event of the dataset says what
//! the event of a person says (ADR-0007), and the history and the notifications read both alike.

use bb_core::access::{Assignment, Group};
use bb_core::event::FieldChange;
use bb_core::fields::FieldKey;
use bb_core::files::{Attachment, StoredFile};
use bb_core::id::FieldId;
use bb_core::id::{IssueId, UserId};
use bb_core::issue::Issue;
use bb_core::own_fields::FieldType;
use bb_core::own_fields_value::FieldValue;
use bb_core::project::Project;
use bb_core::relation::Relation;
use bb_core::version::counted_up;
use bb_core::workflow::StatusKey;
use serde_json::{Value, json};

use crate::auth::set_to;
use crate::files::member as file_member;
use crate::issues::member as issue_member;
use crate::member::comment as comment_member;
use crate::tags::NameRef;
use crate::tags::labels_changed;
use bb_core::tags::Label;

/// The change of a member of a record from one value to another; none when it stayed.
#[must_use]
pub(crate) fn difference(name: &str, was: Value, now: Value) -> Option<FieldChange> {
    (was != now).then(|| FieldChange {
        field: name.to_owned(),
        before: Some(was),
        after: Some(now),
    })
}

/// A new issue: its project, type, title, description, status, priority, who reported it and the
/// people it is assigned to, then the values of its own fields (`own`).
#[must_use]
pub fn issue_created(
    issue: &Issue,
    assigned: &[UserId],
    own: Vec<FieldChange>,
) -> Vec<FieldChange> {
    let mut changes = vec![
        set_to(issue_member::PROJECT, json!(issue.project.to_string())),
        set_to(
            issue_member::ISSUE_TYPE,
            json!(issue.issue_type.to_string()),
        ),
        set_to(issue_member::TITLE, json!(issue.title)),
        set_to(issue_member::DESCRIPTION, json!(issue.description)),
        set_to(issue_member::STATUS, json!(issue.status.as_str())),
        set_to(issue_member::PRIORITY, json!(issue.priority.code())),
        set_to(
            issue_member::REPORTED_BY,
            json!(issue.reported_by.to_string()),
        ),
        set_to(
            issue_member::ASSIGNED_TO,
            json!(assigned.iter().map(ToString::to_string).collect::<Vec<_>>()),
        ),
    ];
    changes.extend(own);
    changes
}

/// A status change of an issue that had `version_before`.
#[must_use]
pub fn status_changed(
    before: &StatusKey,
    after: &StatusKey,
    version_before: i64,
) -> Vec<FieldChange> {
    difference(
        issue_member::STATUS,
        json!(before.as_str()),
        json!(after.as_str()),
    )
    .into_iter()
    .chain(std::iter::once(counted_up(version_before)))
    .collect()
}

/// A new comment under an issue.
#[must_use]
pub fn comment_written(issue: IssueId, text: &str) -> Vec<FieldChange> {
    vec![
        set_to(comment_member::ISSUE, json!(issue.to_string())),
        set_to(comment_member::TEXT, json!(text)),
    ]
}

/// A tag put on an issue that had `version_before`, with the names of the tags before and after.
#[must_use]
pub fn tag_put_on(before: &[String], after: &[String], version_before: i64) -> Vec<FieldChange> {
    vec![
        crate::tags::names_changed(issue_member::TAGS, before, after),
        counted_up(version_before),
    ]
}

/// A label put on an issue that had `version_before`, with the labels before and after.
#[must_use]
pub fn label_put_on(before: &[Label], after: &[Label], version_before: i64) -> Vec<FieldChange> {
    let refs = |labels: &[Label]| labels.iter().map(NameRef::from).collect::<Vec<_>>();
    vec![
        labels_changed(&refs(before), &refs(after)),
        counted_up(version_before),
    ]
}

/// An issue put under another one, taken out from under it or moved, when it had
/// `version_before`.
#[must_use]
pub fn parent_changed(
    before: Option<IssueId>,
    after: Option<IssueId>,
    version_before: i64,
) -> Vec<FieldChange> {
    let shown = |id: Option<IssueId>| id.map(|id| json!(id.to_string()));
    vec![
        FieldChange {
            field: issue_member::PARENT.to_owned(),
            before: shown(before),
            after: shown(after),
        },
        counted_up(version_before),
    ]
}

/// A relation between two issues, as it was added.
#[must_use]
pub fn relation_added(relation: &Relation) -> Vec<FieldChange> {
    crate::relations::relation_changes(relation, true)
}

/// A file put on an issue, or on a comment of it.
#[must_use]
pub fn attachment_added(attachment: &Attachment, file: &StoredFile) -> Vec<FieldChange> {
    let mut changes = vec![
        set_to(file_member::ISSUE, json!(attachment.issue.to_string())),
        set_to(file_member::NAME, json!(attachment.name)),
        set_to(
            file_member::FILE,
            json!(bb_core::hex::encode(file.hash.as_bytes())),
        ),
        set_to(file_member::MEDIA_TYPE, json!(file.media_type.as_str())),
    ];
    if let Some(comment) = attachment.comment {
        changes.push(set_to(file_member::COMMENT, json!(comment.to_string())));
    }
    changes
}

/// Gives a person the handle that others mention them by, as the making of an account does
/// (`reserved` are the handles of groups and roles, which a person does not get).
///
/// # Errors
/// The error of the store, or its "aborted" error when no free handle was found.
pub async fn give_handle<S: crate::auth::AuthStore>(
    transaction: &mut S::Transaction,
    person: &bb_core::person::Person,
    at: jiff::Timestamp,
    reserved: &[String],
) -> Result<(), S::Error> {
    crate::notifications::give_handle::<S>(transaction, person, at, reserved).await
}

/// The change of an own field from `old` to `new` (none is no value), as the event keeps it.
#[must_use]
pub(crate) fn own_change(
    field: FieldId,
    kind: FieldType,
    old: Option<&FieldValue>,
    new: Option<&FieldValue>,
) -> FieldChange {
    FieldChange {
        field: FieldKey::Own(field).code().into_owned(),
        before: old.and_then(|value| value.to_json(kind)),
        after: new.and_then(|value| value.to_json(kind)),
    }
}

/// The value an own field got when its issue was made.
#[must_use]
pub fn own_value_set(field: FieldId, kind: FieldType, value: &FieldValue) -> FieldChange {
    own_change(field, kind, None, Some(value))
}

/// A person who joined: the account was made.
#[must_use]
pub fn person_joined() -> Vec<FieldChange> {
    vec![crate::auth::changed("account")]
}

/// A new project.
#[must_use]
pub fn project_created(project: &Project) -> Vec<FieldChange> {
    vec![
        set_to(
            crate::member::project::SHORT_NAME,
            json!(project.short_name.as_str()),
        ),
        set_to(crate::member::project::NAME, json!(project.name)),
        set_to(
            "parent",
            json!(project.parent.map(|parent| parent.to_string())),
        ),
    ]
}

/// A new group.
#[must_use]
pub fn group_created(group: &Group) -> Vec<FieldChange> {
    vec![set_to(crate::member::group::NAME, json!(group.name))]
}

/// A person put into a group.
#[must_use]
pub fn group_member_added() -> Vec<FieldChange> {
    vec![crate::auth::changed("member")]
}

/// A role given to a person or a group, with where it counts.
#[must_use]
pub fn assignment_given(row: &Assignment) -> Vec<FieldChange> {
    vec![
        set_to("role", json!(row.role.map(|role| role.to_string()))),
        set_to("applies_to", json!(row.applies_to.code())),
    ]
}

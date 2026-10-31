//! The members of the events of saved searches (ADR-0034, decision 7): made, changed, shared, no
//! longer shared, deleted. History and the security log read them by these names.

use bb_core::event::FieldChange;
use bb_core::id::SavedSearchId;
use bb_core::saved_search::{SavedSearch, SearchColumn, ShareTarget};
use bb_core::version::counted_up;
use serde_json::{Value, json};

use crate::auth::set_to;
use crate::event_members::difference;
use crate::member::saved_search as member;

/// The columns as one value of an event: each with its field and width, in order.
fn columns_json(columns: &[SearchColumn]) -> Value {
    json!(
        columns
            .iter()
            .map(|column| json!({ "field": column.field, "width": column.width }))
            .collect::<Vec<_>>()
    )
}

/// The target of a share as one value of an event.
pub(super) fn target_json(target: ShareTarget) -> Value {
    json!({ "kind": target.kind().code(), "id": target.id_text() })
}

/// The change of the list of a project, for the event about the project: the search it showed and
/// the one it shows from now on.
pub(super) fn default_list_change(
    before: Option<SavedSearchId>,
    after: Option<SavedSearchId>,
) -> FieldChange {
    let text = |search: Option<SavedSearchId>| search.map(|id| json!(id.to_string()));
    FieldChange {
        field: member::DEFAULT_LIST.to_owned(),
        before: text(before),
        after: text(after),
    }
}

/// A new saved search: everything it holds.
pub(super) fn made(search: &SavedSearch, columns: &[SearchColumn]) -> Vec<FieldChange> {
    let mut changes = vec![
        set_to(member::NAME, json!(search.name)),
        set_to(member::TEXT, json!(search.text)),
        set_to(member::VIEW, json!(search.view.code())),
        set_to(member::COLUMNS, columns_json(columns)),
    ];
    if let Some(field) = &search.group_by {
        changes.push(set_to(member::GROUP_BY, json!(field)));
    }
    changes
}

/// What changed from `before` to `after`, then the version counted up; empty when nothing did.
pub(super) fn changed(
    before: (&SavedSearch, &[SearchColumn]),
    after: (&SavedSearch, &[SearchColumn]),
) -> Vec<FieldChange> {
    let ((was, was_columns), (now, now_columns)) = (before, after);
    let mut changes: Vec<FieldChange> = [
        difference(member::NAME, json!(was.name), json!(now.name)),
        difference(member::TEXT, json!(was.text), json!(now.text)),
        difference(member::VIEW, json!(was.view.code()), json!(now.view.code())),
        difference(
            member::COLUMNS,
            columns_json(was_columns),
            columns_json(now_columns),
        ),
        difference(member::GROUP_BY, json!(was.group_by), json!(now.group_by)),
    ]
    .into_iter()
    .flatten()
    .collect();
    if !changes.is_empty() {
        changes.push(counted_up(was.version));
    }
    changes
}

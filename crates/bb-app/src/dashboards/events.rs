//! The members of the events of dashboards (ADR-0050, decision 9): made, changed, shared, no
//! longer shared, deleted. History and the security log read them by these names.

use bb_core::dashboard::{BoxSource, ChartShape, Dashboard, DashboardBox, TimeSpan};
use bb_core::event::FieldChange;
use bb_core::saved_search::ShareTarget;
use bb_core::version::counted_up;
use serde_json::{Value, json};

use crate::auth::set_to;
use crate::event_members::difference;
use crate::member::dashboard as member;

/// The boxes as one value of an event: each with its ID, kind, place, title, source and options,
/// in order.
fn boxes_json(boxes: &[DashboardBox]) -> Value {
    json!(
        boxes
            .iter()
            .map(|each| {
                let spec = &each.spec;
                let (text, search) = match &spec.source {
                    Some(BoxSource::Text(text)) => (Some(text.clone()), None),
                    Some(BoxSource::Saved(search)) => (None, Some(search.to_string())),
                    None => (None, None),
                };
                json!({
                    "id": each.id.to_string(),
                    "kind": spec.kind.code(),
                    "column": spec.place.column,
                    "row": spec.place.row,
                    "width": spec.place.width,
                    "height": spec.place.height,
                    "title": spec.title,
                    "search_text": text,
                    "saved_search": search,
                    "by": spec.options.by,
                    "shape": spec.options.shape.map(ChartShape::code),
                    "span": spec.options.span.map(TimeSpan::code),
                })
            })
            .collect::<Vec<_>>()
    )
}

/// The change of a share: the group or project the dashboard is shared with, before and after.
pub(super) fn share_change(target: ShareTarget, shared: bool) -> FieldChange {
    let value = Some(json!({ "kind": target.kind().code(), "id": target.id_text() }));
    let (before, after) = if shared { (None, value) } else { (value, None) };
    FieldChange {
        field: member::SHARED_WITH.to_owned(),
        before,
        after,
    }
}

/// A new dashboard: everything it holds.
pub(super) fn made(dashboard: &Dashboard, boxes: &[DashboardBox]) -> Vec<FieldChange> {
    vec![
        set_to(member::NAME, json!(dashboard.name)),
        set_to(member::HOME, json!(dashboard.home)),
        set_to(member::BOXES, boxes_json(boxes)),
    ]
}

/// What changed from `before` to `after`, then the version counted up; empty when nothing did.
pub(super) fn changed(
    before: (&Dashboard, &[DashboardBox]),
    after: (&Dashboard, &[DashboardBox]),
) -> Vec<FieldChange> {
    let ((was, was_boxes), (now, now_boxes)) = (before, after);
    let mut changes: Vec<FieldChange> = [
        difference(member::NAME, json!(was.name), json!(now.name)),
        difference(member::BOXES, boxes_json(was_boxes), boxes_json(now_boxes)),
    ]
    .into_iter()
    .flatten()
    .collect();
    if !changes.is_empty() {
        changes.push(counted_up(was.version));
    }
    changes
}

/// Into the trash or out of it, then the version counted up.
pub(super) fn trashed(was: &Dashboard, now: &Dashboard) -> Vec<FieldChange> {
    let moment = |dashboard: &Dashboard| dashboard.deleted_at.map(|at| json!(at.to_string()));
    vec![
        FieldChange {
            field: member::DELETED_AT.to_owned(),
            before: moment(was),
            after: moment(now),
        },
        counted_up(was.version),
    ]
}

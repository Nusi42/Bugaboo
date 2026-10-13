//! The history of an issue (ISS-06, SEC-06): `/api/v1/issues/{issue}/history`, with the tabs
//! *Everything*, *Comments* and *Changes*.
//!
//! The history is made from the domain events and filtered for the reader: a change to a field the
//! reader may not see is not in it. A row is flat, so a list of them names related things by ID or
//! by reference only (ARCH-LEAN): a comment holds its text and its HTML, a change of an issue is
//! one item for each field that changed, and the items of one event share the `event`.

use axum::extract::State;
use axum::response::Response;
use bb_app::history::{ChangeValue, HistoryItem, HistoryQuery, HistoryWhat};
use bb_core::history::HistoryTab;
use bb_core::own_fields::FieldType;
use bb_core::relation::Direction;
use bb_policy::{Masked, PersonRef};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::access_bodies::cursor_error;
use super::auth::{AuthHandle, Info, NotFoundAnswer, NotSignedInAnswer, Session, answer};
use super::comments::CommentItem;
use super::relations::IssueRef;
use crate::errors::{ApiError, ApiPath, ApiQuery};
use crate::lean::{FieldSpec, ListParams, Listed, Page};

const HISTORY_LIST: &str = "history";

/// The fields of an item of the history that can be asked for. A row holds only the ones that
/// belong to its kind.
const HISTORY_FIELDS: [FieldSpec; 25] = [
    FieldSpec::by_default("event"),
    FieldSpec::by_default("at"),
    FieldSpec::by_default("by"),
    FieldSpec::by_default("kind"),
    FieldSpec::by_default("comment"),
    FieldSpec::by_default("text"),
    FieldSpec::by_default("html"),
    FieldSpec::by_default("edited_at"),
    FieldSpec::by_default("version"),
    FieldSpec::by_default("may_edit"),
    FieldSpec::by_default("may_delete"),
    FieldSpec::by_default("field"),
    FieldSpec::by_default("field_name"),
    FieldSpec::by_default("field_kind"),
    FieldSpec::by_default("before_text"),
    FieldSpec::by_default("after_text"),
    FieldSpec::by_default("before_value"),
    FieldSpec::by_default("after_value"),
    FieldSpec::by_default("before_people"),
    FieldSpec::by_default("after_people"),
    FieldSpec::by_default("before_issue"),
    FieldSpec::by_default("after_issue"),
    FieldSpec::by_default("relation"),
    FieldSpec::by_default("direction"),
    FieldSpec::by_default("issue"),
];

/// What an item of the history is.
#[derive(Clone, Copy, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum HistoryEntryKind {
    /// A comment.
    Comment,
    /// One field of the issue that changed.
    Change,
    /// A relation to another issue that was added or taken away.
    Relation,
}

/// One item of the history of an issue.
#[derive(Serialize, ToSchema)]
pub(crate) struct HistoryEntryItem {
    /// The event the item was made from; the items of one event share it.
    event: String,
    /// When it happened.
    at: String,
    /// Who did it, as the reader may see people of the kind *Authors*; left out for Bugaboo itself.
    #[serde(skip_serializing_if = "Option::is_none")]
    by: Option<PersonRef>,
    /// What it is.
    kind: HistoryEntryKind,
    /// A comment: its ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    comment: Option<String>,
    /// A comment: the text, in Markdown.
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    /// A comment: the text as safe HTML, made on the server for this reader.
    #[serde(skip_serializing_if = "Option::is_none")]
    html: Option<String>,
    /// A comment: when its text was last changed.
    #[serde(skip_serializing_if = "Option::is_none")]
    edited_at: Option<String>,
    /// A comment: the version, which a change of it is made on.
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<i64>,
    /// A comment: whether the reader may change its text.
    #[serde(skip_serializing_if = "Option::is_none")]
    may_edit: Option<bool>,
    /// A comment: whether the reader may put it into the trash.
    #[serde(skip_serializing_if = "Option::is_none")]
    may_delete: Option<bool>,
    /// A change: the key of the field.
    #[serde(skip_serializing_if = "Option::is_none")]
    field: Option<String>,
    /// A change of an own field: its name.
    #[serde(skip_serializing_if = "Option::is_none")]
    field_name: Option<String>,
    /// A change of an own field: its kind, which says how `before_value` and `after_value` are
    /// worded.
    #[serde(skip_serializing_if = "Option::is_none")]
    field_kind: Option<FieldType>,
    /// A change: what the field was, when it is a word or a name.
    #[serde(skip_serializing_if = "Option::is_none")]
    before_text: Option<String>,
    /// A change: what the field is now, when it is a word or a name.
    #[serde(skip_serializing_if = "Option::is_none")]
    after_text: Option<String>,
    /// A change of an own field that a screen words itself (yes or no, a length of time, a number,
    /// a day): what the field was, as the JSON of its kind.
    #[serde(skip_serializing_if = "Option::is_none")]
    before_value: Option<Value>,
    /// The same for what the field is now.
    #[serde(skip_serializing_if = "Option::is_none")]
    after_value: Option<Value>,
    /// A change: the people the field held, as the kind of the field says.
    #[serde(skip_serializing_if = "Option::is_none")]
    before_people: Option<Vec<PersonRef>>,
    /// A change: the people the field holds now.
    #[serde(skip_serializing_if = "Option::is_none")]
    after_people: Option<Vec<PersonRef>>,
    /// A change: the issue the field held (the parent), when the reader may see it.
    #[serde(skip_serializing_if = "Option::is_none")]
    before_issue: Option<IssueRef>,
    /// A change: the issue the field holds now.
    #[serde(skip_serializing_if = "Option::is_none")]
    after_issue: Option<IssueRef>,
    /// A relation: `added` or `removed`.
    #[serde(skip_serializing_if = "Option::is_none")]
    relation: Option<String>,
    /// A relation: which end this issue is.
    #[serde(skip_serializing_if = "Option::is_none")]
    direction: Option<Direction>,
    /// A relation: the issue at the other end.
    #[serde(skip_serializing_if = "Option::is_none")]
    issue: Option<IssueRef>,
}

impl Masked for HistoryEntryItem {}

/// The first part of every item of an event.
fn start(item: &HistoryItem, kind: HistoryEntryKind) -> HistoryEntryItem {
    HistoryEntryItem {
        event: item.event.to_string(),
        at: item.at.to_string(),
        by: item.by.clone(),
        kind,
        comment: None,
        text: None,
        html: None,
        edited_at: None,
        version: None,
        may_edit: None,
        may_delete: None,
        field: None,
        field_name: None,
        field_kind: None,
        before_text: None,
        after_text: None,
        before_value: None,
        after_value: None,
        before_people: None,
        after_people: None,
        before_issue: None,
        after_issue: None,
        relation: None,
        direction: None,
        issue: None,
    }
}

/// The places in an item where one side of a change goes.
#[derive(Default)]
struct Side {
    text: Option<String>,
    value: Option<Value>,
    people: Option<Vec<PersonRef>>,
    issue: Option<IssueRef>,
    kind: Option<FieldType>,
}

impl Side {
    /// Where a value of a change goes in an item.
    fn of(value: Option<&ChangeValue>) -> Self {
        let mut side = Self::default();
        match value {
            Some(ChangeValue::Text { text }) => side.text = Some(text.clone()),
            Some(ChangeValue::Typed { kind, value }) => {
                side.kind = Some(*kind);
                side.value = Some(value.clone());
            }
            Some(ChangeValue::People { people }) => side.people = Some(people.clone()),
            Some(ChangeValue::Issue { issue }) => side.issue = Some(IssueRef::from(issue)),
            None => {}
        }
        side
    }
}

/// The items of one row of the history: a comment and a relation are one each, a change one for
/// each field.
fn items_of(row: &HistoryItem) -> Vec<HistoryEntryItem> {
    match &row.what {
        HistoryWhat::Comment { comment } => {
            let shown = CommentItem::from(comment);
            let mut item = start(row, HistoryEntryKind::Comment);
            item.comment = Some(shown.id);
            item.text = Some(shown.text);
            item.html = Some(shown.html);
            item.edited_at = shown.edited_at;
            item.version = Some(shown.version);
            item.may_edit = Some(shown.may_edit);
            item.may_delete = Some(shown.may_delete);
            vec![item]
        }
        HistoryWhat::Changes { fields } => fields
            .iter()
            .map(|change| {
                let mut item = start(row, HistoryEntryKind::Change);
                item.field = Some(change.field.clone());
                item.field_name.clone_from(&change.name);
                let (before, after) = (
                    Side::of(change.before.as_ref()),
                    Side::of(change.after.as_ref()),
                );
                item.field_kind = before.kind.or(after.kind);
                (item.before_text, item.before_value) = (before.text, before.value);
                (item.before_people, item.before_issue) = (before.people, before.issue);
                (item.after_text, item.after_value) = (after.text, after.value);
                (item.after_people, item.after_issue) = (after.people, after.issue);
                item
            })
            .collect(),
        HistoryWhat::Relation {
            added,
            direction,
            name,
            issue,
        } => {
            let mut item = start(row, HistoryEntryKind::Relation);
            item.relation = Some(if *added { "added" } else { "removed" }.to_owned());
            item.direction = Some(*direction);
            item.field = Some(name.clone());
            item.issue = Some(IssueRef::from(issue));
            vec![item]
        }
    }
}

/// Which rows of the history to list.
#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct TabParams {
    /// Everything (the default), only the comments, or only the changes.
    #[param(inline)]
    tab: Option<HistoryTab>,
}

/// The history of an issue, newest first, a page at a time: the comments and the changes, in three
/// tabs. A change to a field the person may not see is not in it. A row's changes stay together, so
/// a page may hold a few items more than `limit`.
#[utoipa::path(
    get,
    path = "/api/v1/issues/{issue}/history",
    operation_id = "listIssueHistory",
    tag = "history",
    params(("issue" = String, Path, description = "The readable ID (WEB-142) or the internal ID of the issue."), ListParams, TabParams),
    responses((status = 200, description = "The history.", body = Page<HistoryEntryItem>), NotSignedInAnswer, NotFoundAnswer)
)]
async fn list_issue_history(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(issue): ApiPath<String>,
    ApiQuery(params): ApiQuery<ListParams>,
    ApiQuery(tab): ApiQuery<TabParams>,
) -> Result<Response, ApiError> {
    let position = params.position(HISTORY_LIST);
    let cursor = match &position {
        Ok(position) => position.clone(),
        Err(_) => Some(String::new()),
    };
    let page = auth
        .api()?
        .history(
            info,
            HistoryQuery {
                session_token,
                issue,
                tab: tab.tab.unwrap_or(HistoryTab::Everything),
                cursor,
                limit: params.limit(),
            },
        )
        .await
        .map_err(cursor_error)?;
    position?;
    let list = page.map(|page| Listed {
        items: page.items.iter().flat_map(items_of).collect(),
        next: page.next_cursor,
    });
    let fields = params.fields_readable(&HISTORY_FIELDS, &list.hidden_properties())?;
    Ok(answer(Page::of_authorized(HISTORY_LIST, &list, &fields)?))
}

/// The history route.
pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new().routes(routes!(list_issue_history))
}

#[cfg(test)]
mod tests {
    use bb_core::issue::{IssueIcon, Status, StatusGroup};

    use super::*;
    use crate::api::issues::IssueTypeRef;
    use crate::api::projects::ProjectRef;

    #[test]
    fn perf_02_the_field_table_names_exactly_the_fields_of_an_item() {
        let line = || IssueRef {
            id: String::new(),
            key: String::new(),
            project: ProjectRef {
                id: String::new(),
                short_name: String::new(),
                name: String::new(),
            },
            issue_type: IssueTypeRef {
                id: String::new(),
                name: String::new(),
                icon: IssueIcon::Bug,
                retired: false,
            },
            title: None,
            status: Some(Status::Open),
            status_group: Some(StatusGroup::NotStarted),
        };
        let item = HistoryEntryItem {
            event: String::new(),
            at: String::new(),
            by: Some(PersonRef::Someone),
            kind: HistoryEntryKind::Change,
            comment: Some(String::new()),
            text: Some(String::new()),
            html: Some(String::new()),
            edited_at: Some(String::new()),
            version: Some(1),
            may_edit: Some(true),
            may_delete: Some(true),
            field: Some(String::new()),
            field_name: Some(String::new()),
            field_kind: Some(FieldType::YesNo),
            before_text: Some(String::new()),
            after_text: Some(String::new()),
            before_value: Some(Value::Null),
            after_value: Some(Value::Null),
            before_people: Some(Vec::new()),
            after_people: Some(Vec::new()),
            before_issue: Some(line()),
            after_issue: Some(line()),
            relation: Some(String::new()),
            direction: Some(Direction::Forward),
            issue: Some(line()),
        };
        crate::lean::assert_fields_are_the_keys_of(&HISTORY_FIELDS, &item);
    }
}

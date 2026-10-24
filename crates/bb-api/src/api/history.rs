//! The history of an issue (ISS-06, SEC-06): `/api/v1/issues/{issue}/history`, with the tabs
//! *Everything*, *Comments* and *Changes*.
//!
//! The history is made from the domain events and filtered for the reader: a change to a field the
//! reader may not see is not in it. A row is flat, so a list of them names related things by ID or
//! by reference only (ARCH-LEAN): a comment holds its text and its HTML, a change of an issue is
//! one item for each field that changed, and the items of one event share the `event`.

use axum::extract::State;
use axum::response::Response;
use bb_app::history::{HistoryItem, HistoryQuery, HistoryWhat};
use bb_core::history::{FileEvent, HistoryTab};
use bb_core::relation::Direction;
use bb_policy::{Masked, PersonRef};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::access_bodies::cursor_error;
use super::auth::{AuthHandle, Info, NotFoundAnswer, NotSignedInAnswer, Session, answer};
use super::comments::CommentItem;
use super::many_changes::FieldChangeBody;
use super::relations::IssueRef;
use crate::errors::{ApiError, ApiPath, ApiQuery};
use crate::lean::{FieldSpec, ListParams, Listed, Page};

const HISTORY_LIST: &str = "history";

/// The fields of an item of the history that can be asked for. A row holds only the ones that
/// belong to its kind.
const HISTORY_FIELDS: [FieldSpec; 18] = [
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
    FieldSpec::by_default("change"),
    FieldSpec::by_default("relation_event"),
    FieldSpec::by_default("relation_name"),
    FieldSpec::by_default("direction"),
    FieldSpec::by_default("issue"),
    FieldSpec::by_default("file_event"),
    FieldSpec::by_default("file_name"),
];

/// What happened to a relation.
#[derive(Clone, Copy, Serialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum RelationEvent {
    /// The relation was added.
    Added,
    /// The relation was taken away.
    Removed,
}

/// What an item of the history is.
#[derive(Clone, Copy, Serialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum HistoryEntryKind {
    /// A comment.
    Comment,
    /// One field of the issue that changed.
    Change,
    /// A relation to another issue that was added or taken away.
    Relation,
    /// A file of the issue that was added, put into the trash or brought back.
    File,
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
    /// A change: the field that changed, with what it was and what it is now.
    #[serde(skip_serializing_if = "Option::is_none")]
    change: Option<FieldChangeBody>,
    /// A relation: whether it was added or taken away.
    #[serde(skip_serializing_if = "Option::is_none")]
    relation_event: Option<RelationEvent>,
    /// A relation: what the relation reads (*blocks*).
    #[serde(skip_serializing_if = "Option::is_none")]
    relation_name: Option<String>,
    /// A relation: which end this issue is.
    #[serde(skip_serializing_if = "Option::is_none")]
    direction: Option<Direction>,
    /// A relation: the issue at the other end.
    #[serde(skip_serializing_if = "Option::is_none")]
    issue: Option<IssueRef>,
    /// A file: what happened to it.
    #[serde(skip_serializing_if = "Option::is_none")]
    file_event: Option<FileEvent>,
    /// A file: its name.
    #[serde(skip_serializing_if = "Option::is_none")]
    file_name: Option<String>,
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
        change: None,
        relation_event: None,
        relation_name: None,
        direction: None,
        issue: None,
        file_event: None,
        file_name: None,
    }
}

/// The items of one row of the history: a comment, a file and a relation are one each, a change one for
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
                item.change = Some(FieldChangeBody::from(change));
                item
            })
            .collect(),
        HistoryWhat::File { event, name } => {
            let mut item = start(row, HistoryEntryKind::File);
            item.file_event = Some(*event);
            item.file_name.clone_from(name);
            vec![item]
        }
        HistoryWhat::Relation {
            added,
            direction,
            name,
            issue,
        } => {
            let mut item = start(row, HistoryEntryKind::Relation);
            item.relation_event = Some(if *added {
                RelationEvent::Added
            } else {
                RelationEvent::Removed
            });
            item.direction = Some(*direction);
            item.relation_name = Some(name.clone());
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
    let (cursor, bad_cursor) = params.start(HISTORY_LIST);
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
    bad_cursor?;
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
    use bb_core::issue::IssueIcon;

    use super::*;
    use crate::api::issues::IssueTypeRef;
    use crate::api::projects::ProjectRef;
    use crate::api::workflows::StatusRef;

    #[test]
    fn perf_02_the_field_table_names_exactly_the_fields_of_an_item() {
        let line = || IssueRef {
            id: String::new(),
            readable_id: String::new(),
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
            status: Some(StatusRef::blank()),
            priority: Some(bb_core::issue::Priority::Normal),
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
            change: Some(FieldChangeBody::blank()),
            relation_event: Some(RelationEvent::Added),
            relation_name: Some(String::new()),
            direction: Some(Direction::Forward),
            issue: Some(line()),
            file_event: Some(FileEvent::Added),
            file_name: Some(String::new()),
        };
        crate::lean::assert_fields_are_the_keys_of(&HISTORY_FIELDS, &item);
    }
}

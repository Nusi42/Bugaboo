//! The routes of time entries (TIME-02, PRIV-09): a person's own entries below
//! `/api/v1/me/time-entries` (booked by hand, changed and deleted only by their person), and the
//! entries on an issue below `/api/v1/issues/{issue}/time-entries`.
//!
//! On an issue each person is a `PersonRef` as *Who sees people* allows for the kind *Time
//! entries*, and the length and the note are only in the answer for a reader who may read *Time
//! spent*. No route of time takes an order: a person's own entries come by date, those of an issue
//! newest first, so nothing orders or ranks people by their time (ADR-0035, decision 9).

use axum::extract::State;
use axum::response::Response;
use bb_app::time::{
    AddTime, ChangeTimeEntry, DeleteTimeEntry, IssueTimeEntriesQuery, IssueTimeEntryView,
    MyTimeEntriesQuery, TimeEntryView,
};
use bb_core::fields::Field;
use bb_core::id::TimeEntryId;
use bb_core::own_fields_value::OwnValues;
use bb_core::time::Minutes;
use bb_policy::{
    Authorized, FieldMask, Masked, MaskedValue, PersonRef, drop_hidden, drop_hidden_own_fields,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::access_bodies::cursor_error;
use super::auth::{
    AuthHandle, ConflictAnswer, ForbiddenAnswer, Info, InvalidAnswer, NotFoundAnswer,
    NotSignedInAnswer, Session, answer_authorized, done,
};
use super::bodies::VersionParams;
use super::time::EntryDetailsBody;
use super::{body_id, path_id};
use crate::errors::{ApiError, ApiJson, ApiPath, ApiQuery};
use crate::lean::{FieldSpec, ListParams, Listed, Page};

const MINE_LIST: &str = "my-time-entries";
const ISSUE_LIST: &str = "issue-time-entries";

/// The name of the member that holds the values of own fields.
const OWN_FIELDS: &str = "own_fields";

const MINE_FIELDS: [FieldSpec; 10] = [
    FieldSpec::by_default("id"),
    FieldSpec::by_default("issue"),
    FieldSpec::by_default("readable_id"),
    FieldSpec::by_default("date"),
    FieldSpec::by_default("minutes"),
    FieldSpec::by_default("note"),
    FieldSpec::on_request("billable"),
    FieldSpec::on_request("started_at"),
    FieldSpec::on_request(OWN_FIELDS),
    FieldSpec::by_default("version"),
];

const ISSUE_FIELDS: [FieldSpec; 6] = [
    FieldSpec::by_default("id"),
    FieldSpec::by_default("person"),
    FieldSpec::by_default("date"),
    FieldSpec::by_default("minutes"),
    FieldSpec::on_request("note"),
    FieldSpec::on_request(OWN_FIELDS),
];

/// One of the person's own time entries.
#[derive(Serialize, ToSchema)]
pub(crate) struct TimeEntryItem {
    /// The ID of the entry.
    id: String,
    /// The internal ID of the issue.
    issue: String,
    /// The readable ID of the issue (WEB-142) while the person sees it.
    #[serde(skip_serializing_if = "Option::is_none")]
    readable_id: Option<String>,
    /// The date (YYYY-MM-DD).
    date: String,
    /// How long, in whole minutes.
    minutes: u32,
    /// The note, plain text: shown as text, never as HTML.
    note: String,
    /// Whether the time can be charged to a customer; left out while the organization does not
    /// offer it.
    #[serde(skip_serializing_if = "Option::is_none")]
    billable: Option<bool>,
    /// The start of the timer it was saved from; left out for an entry booked by hand.
    #[serde(skip_serializing_if = "Option::is_none")]
    started_at: Option<String>,
    /// The values of its own fields the person may see, by the key (ID) of the field.
    own_fields: OwnValues,
    /// Counts up with every change. A change is made on the version that was looked at.
    version: i64,
}

impl Masked for TimeEntryItem {
    fn mask(value: &mut MaskedValue, mask: &FieldMask) {
        drop_hidden_own_fields(value, OWN_FIELDS, mask);
    }
}

impl From<&TimeEntryView> for TimeEntryItem {
    fn from(view: &TimeEntryView) -> Self {
        Self {
            id: view.id.to_string(),
            issue: view.issue.to_string(),
            readable_id: view.readable_id.clone(),
            date: view.date.to_string(),
            minutes: view.minutes.get(),
            note: view.note.clone(),
            billable: view.billable,
            started_at: view.started_at.map(|at| at.to_string()),
            own_fields: view.own_fields.clone(),
            version: view.version,
        }
    }
}

/// A time entry on an issue, as the reader sees it.
#[derive(Serialize, ToSchema)]
pub(crate) struct IssueTimeEntryItem {
    /// The ID of the entry.
    id: String,
    /// Whose time it is, as the reader may see people of the kind *Time entries*.
    person: PersonRef,
    /// The date (YYYY-MM-DD).
    date: String,
    /// How long, in whole minutes; only for a reader who may read *Time spent*.
    #[serde(skip_serializing_if = "Option::is_none")]
    minutes: Option<u32>,
    /// The note, plain text; only for a reader who may read *Time spent*.
    #[serde(skip_serializing_if = "Option::is_none")]
    note: Option<String>,
    /// The values of its own fields the reader may see, by the key (ID) of the field.
    own_fields: OwnValues,
}

impl Masked for IssueTimeEntryItem {
    const FIELDS: &'static [(&'static str, Field)] =
        &[("minutes", Field::TimeSpent), ("note", Field::TimeSpent)];

    fn mask(value: &mut MaskedValue, mask: &FieldMask) {
        drop_hidden(value, Self::FIELDS, mask);
        drop_hidden_own_fields(value, OWN_FIELDS, mask);
    }
}

impl From<&IssueTimeEntryView> for IssueTimeEntryItem {
    fn from(view: &IssueTimeEntryView) -> Self {
        Self {
            id: view.id.to_string(),
            person: view.person.clone(),
            date: view.date.to_string(),
            minutes: view.minutes.map(Minutes::get),
            note: view.note.clone(),
            own_fields: view.own_fields.clone(),
        }
    }
}

/// Time booked by hand.
#[derive(Deserialize, ToSchema)]
pub(crate) struct NewTimeEntryBody {
    /// The readable ID (WEB-142) or the internal ID of the issue.
    issue: String,
    /// The date the time belongs to (YYYY-MM-DD).
    date: String,
    /// How long, in whole minutes: at least 1, at most 1,440.
    minutes: u32,
    /// The note, *Billable* and the values of own fields; the defaults of the issue fill in the
    /// rest.
    #[serde(default)]
    entry: Option<EntryDetailsBody>,
}

/// A change of one of the person's own entries; what is left out stays.
#[derive(Deserialize, ToSchema)]
pub(crate) struct ChangeTimeEntryBody {
    /// The version of the entry that was looked at.
    version: i64,
    /// A new date (YYYY-MM-DD).
    #[serde(default)]
    date: Option<String>,
    /// A new length, in whole minutes.
    #[serde(default)]
    minutes: Option<u32>,
    /// A new note, *Billable* or values of own fields (`null` takes a value away).
    #[serde(default)]
    entry: Option<EntryDetailsBody>,
}

/// The dates of the person's own entries to list.
#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct DateRange {
    /// The first date (YYYY-MM-DD); with `to` left out, the person's week of today.
    from: Option<String>,
    /// The last date, included (YYYY-MM-DD).
    to: Option<String>,
}

/// The person's own time entries of some dates (a week of the timesheet), by date, then in the
/// order they were made.
#[utoipa::path(
    get,
    path = "/api/v1/me/time-entries",
    operation_id = "listMyTimeEntries",
    tag = "time",
    params(DateRange, ListParams),
    responses((status = 200, description = "The entries.", body = Page<TimeEntryItem>), InvalidAnswer, NotSignedInAnswer)
)]
async fn list_my_time_entries(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiQuery(range): ApiQuery<DateRange>,
    ApiQuery(params): ApiQuery<ListParams>,
) -> Result<Response, ApiError> {
    let (cursor, bad_cursor) = params.start(MINE_LIST);
    let page = auth
        .api()?
        .my_time_entries(
            info,
            MyTimeEntriesQuery {
                session_token,
                from: range.from.as_deref().map(body_id).transpose()?,
                to: range.to.as_deref().map(body_id).transpose()?,
                cursor,
                limit: params.limit(),
            },
        )
        .await
        .map_err(cursor_error)?;
    bad_cursor?;
    // Each entry carries the rights where its issue is; the wrapper adds nothing to them.
    let mut taken = None;
    let _ = page.map(|page| taken = Some((page.items, page.next_cursor)));
    let (items, next) = taken.unwrap_or_default();
    let items: Vec<Authorized<TimeEntryItem>> = items
        .into_iter()
        .map(|each| each.map(|view| TimeEntryItem::from(&view)))
        .collect();
    Ok(super::auth::answer(Page::of_authorized_items(
        MINE_LIST,
        &items,
        next.as_deref(),
        &params,
        &MINE_FIELDS,
    )?))
}

/// Books time on an issue by hand (*Add time*): a date and a length; the own fields left out take
/// the defaults of the issue, then of its projects. Needs *Book time*.
#[utoipa::path(
    post,
    path = "/api/v1/me/time-entries",
    operation_id = "addTimeEntry",
    tag = "time",
    request_body = NewTimeEntryBody,
    responses(
        (status = 200, description = "The entry.", body = TimeEntryItem),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn add_time_entry(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<NewTimeEntryBody>,
) -> Result<Response, ApiError> {
    let added = auth
        .api()?
        .add_time(
            info,
            AddTime {
                session_token,
                issue: body.issue,
                date: body_id(&body.date)?,
                minutes: Minutes::new(body.minutes),
                details: body.entry.unwrap_or_default().into(),
            },
        )
        .await?;
    answer_authorized(&added.map(|entry| TimeEntryItem::from(&entry)))
}

/// Changes one of the person's own entries. Only its person changes an entry.
#[utoipa::path(
    patch,
    path = "/api/v1/me/time-entries/{entry}",
    operation_id = "changeTimeEntry",
    tag = "time",
    params(("entry" = String, Path, description = "The ID of the entry.")),
    request_body = ChangeTimeEntryBody,
    responses(
        (status = 200, description = "The entry as it is now.", body = TimeEntryItem),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn change_time_entry(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(entry): ApiPath<String>,
    ApiJson(body): ApiJson<ChangeTimeEntryBody>,
) -> Result<Response, ApiError> {
    let changed = auth
        .api()?
        .change_time_entry(
            info,
            ChangeTimeEntry {
                session_token,
                entry: path_id::<TimeEntryId>(&entry)?,
                version: body.version,
                date: body.date.as_deref().map(body_id).transpose()?,
                minutes: body.minutes.map(Minutes::new),
                details: body.entry.unwrap_or_default().into(),
            },
        )
        .await?;
    answer_authorized(&changed.map(|entry| TimeEntryItem::from(&entry)))
}

/// Deletes one of the person's own entries. Only its person deletes an entry.
#[utoipa::path(
    delete,
    path = "/api/v1/me/time-entries/{entry}",
    operation_id = "deleteTimeEntry",
    tag = "time",
    params(("entry" = String, Path, description = "The ID of the entry."), VersionParams),
    responses(
        (status = 204, description = "The entry is gone."),
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn delete_time_entry(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(entry): ApiPath<String>,
    ApiQuery(body): ApiQuery<VersionParams>,
) -> Result<Response, ApiError> {
    auth.api()?
        .delete_time_entry(
            info,
            DeleteTimeEntry {
                session_token,
                entry: path_id::<TimeEntryId>(&entry)?,
                version: body.version,
            },
        )
        .await?;
    Ok(done())
}

/// The time entries on an issue the reader sees, newest first: each person as *Who sees people*
/// allows for the kind *Time entries*; the lengths and the notes only for a reader who may read
/// *Time spent*.
#[utoipa::path(
    get,
    path = "/api/v1/issues/{issue}/time-entries",
    operation_id = "listIssueTimeEntries",
    tag = "time",
    params(("issue" = String, Path, description = "The readable ID (WEB-142) or the internal ID of the issue."), ListParams),
    responses((status = 200, description = "The entries, newest first.", body = Page<IssueTimeEntryItem>), NotSignedInAnswer, NotFoundAnswer)
)]
async fn list_issue_time_entries(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(issue): ApiPath<String>,
    ApiQuery(params): ApiQuery<ListParams>,
) -> Result<Response, ApiError> {
    let (cursor, bad_cursor) = params.start(ISSUE_LIST);
    let page = auth
        .api()?
        .issue_time_entries(
            info,
            IssueTimeEntriesQuery {
                session_token,
                issue,
                cursor,
                limit: params.limit(),
            },
        )
        .await
        .map_err(cursor_error)?;
    bad_cursor?;
    let list = page.map(|page| Listed {
        items: page.items.iter().map(IssueTimeEntryItem::from).collect(),
        next: page.next_cursor,
    });
    let fields = params.fields_readable(&ISSUE_FIELDS, &list.hidden_properties())?;
    Ok(super::auth::answer(Page::of_authorized(
        ISSUE_LIST, &list, &fields,
    )?))
}

/// The routes of time entries.
pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(list_my_time_entries, add_time_entry))
        .routes(routes!(change_time_entry, delete_time_entry))
        .routes(routes!(list_issue_time_entries))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perf_02_the_field_tables_name_exactly_the_fields_of_the_items() {
        let mine = TimeEntryItem {
            id: String::new(),
            issue: String::new(),
            readable_id: Some(String::new()),
            date: String::new(),
            minutes: 0,
            note: String::new(),
            billable: Some(false),
            started_at: Some(String::new()),
            own_fields: OwnValues::default(),
            version: 1,
        };
        crate::lean::assert_fields_are_the_keys_of(&MINE_FIELDS, &mine);
        let theirs = IssueTimeEntryItem {
            id: String::new(),
            person: PersonRef::Someone,
            date: String::new(),
            minutes: Some(0),
            note: Some(String::new()),
            own_fields: OwnValues::default(),
        };
        crate::lean::assert_fields_are_the_keys_of(&ISSUE_FIELDS, &theirs);
    }
}

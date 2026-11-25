//! The routes of e-mailed searches (QRY-07, ADR-0033, amendment 2026-11-24):
//! `/api/v1/search-mails` for the person's list, and `/api/v1/saved-searches/{search}/mail` to get
//! a saved search by e-mail, change when it comes, or stop it.
//!
//! An e-mailed search is the person's own: only a session reaches these routes, never an access key
//! or an agent (M10-Q20). A saved search the person may not open is not found, as one that does not
//! exist (ADR-0034, decision 4). The use cases are in `bb-app`, behind
//! [`AuthApi`](bb_app::auth::AuthApi).

use axum::extract::State;
use axum::response::Response;
use bb_app::search_mail::{
    SearchMailList, SearchMailView, SearchMailsQuery, SetSearchMail, StopSearchMail,
};
use bb_core::id::SavedSearchId;
use bb_core::search_mail::MailPause;
use bb_policy::Masked;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::auth::{
    AuthHandle, ConflictAnswer, Info, InvalidAnswer, NotFoundAnswer, NotSignedInAnswer, Session,
    answer_authorized, done,
};
use super::path_id;
use crate::errors::{ApiError, ApiJson, ApiPath};

/// When a person gets a saved search by e-mail.
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct SearchMailBody {
    /// The weekdays, Monday first: seven, at least one of them true.
    days: [bool; 7],
    /// The whole hour of the day, 0 to 23, in the person's time zone (*Region*).
    hour: i64,
    /// *Only when something is new or changed*: nothing goes when no issue the search finds
    /// changed since the last e-mail.
    only_when_changed: bool,
}

/// One e-mailed search as its person sees it.
#[derive(Serialize, ToSchema)]
pub(crate) struct SearchMailItem {
    /// The ID of the saved search it sends.
    search: String,
    /// The name of the saved search; `null` once the person can no longer open it.
    #[schema(required = true)]
    name: Option<String>,
    /// The weekdays it goes on, Monday first.
    days: [bool; 7],
    /// The hour it goes at, in the person's time zone.
    hour: i8,
    /// *Only when something is new or changed*.
    only_when_changed: bool,
    /// Why it sends nothing until the person sets it again; `null` while it sends.
    #[schema(required = true)]
    paused: Option<MailPause>,
    /// When its last e-mail went; `null` before the first.
    #[schema(required = true)]
    last_sent_at: Option<String>,
}

impl Masked for SearchMailItem {}

impl From<&SearchMailView> for SearchMailItem {
    fn from(view: &SearchMailView) -> Self {
        Self {
            search: view.search.to_string(),
            name: view.name.clone(),
            days: view.days,
            hour: view.hour,
            only_when_changed: view.only_when_changed,
            paused: view.paused,
            last_sent_at: view.last_sent_at.map(|at| at.to_string()),
        }
    }
}

/// The e-mailed searches of the person, with what the window and the list say about all of them.
#[derive(Serialize, ToSchema)]
pub(crate) struct SearchMailListBody {
    /// The e-mailed searches, oldest first. A person has at most `most`, so the list is whole.
    search_mails: Vec<SearchMailItem>,
    /// Whether the list was cut off: never, a person has at most `most`.
    truncated: bool,
    /// The most e-mailed searches a person may have.
    most: u32,
    /// The most issues one e-mail lists; the rest are counted.
    most_listed: u32,
    /// Whether e-mail and *E-mail me this search* are on for the organization: off, nothing goes
    /// until they are on again.
    sending: bool,
    /// Whether the organization uses *Short e-mails*: the e-mails have no titles and no name.
    short_e_mails: bool,
}

impl Masked for SearchMailListBody {}

impl From<&SearchMailList> for SearchMailListBody {
    fn from(list: &SearchMailList) -> Self {
        Self {
            search_mails: list.items.iter().map(SearchMailItem::from).collect(),
            truncated: false,
            most: list.most,
            most_listed: list.most_listed,
            sending: list.sending,
            short_e_mails: list.short_e_mails,
        }
    }
}

fn search_id(text: &str) -> Result<SavedSearchId, ApiError> {
    path_id::<SavedSearchId>(text)
}

/// The e-mailed searches of the person, oldest first.
#[utoipa::path(
    get,
    path = "/api/v1/search-mails",
    operation_id = "listSearchMails",
    tag = "saved-searches",
    responses((status = 200, description = "The e-mailed searches.", body = SearchMailListBody), NotSignedInAnswer)
)]
async fn list_search_mails(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
) -> Result<Response, ApiError> {
    let list = auth
        .api()?
        .search_mails(info, SearchMailsQuery { session_token })
        .await?;
    answer_authorized(&list.map(|list| SearchMailListBody::from(&list)))
}

/// Gets a saved search by e-mail, or changes when it comes. An e-mailed search that sent nothing
/// because the person could no longer open the search sends again. Only to the person.
#[utoipa::path(
    put,
    path = "/api/v1/saved-searches/{search}/mail",
    operation_id = "setSearchMail",
    tag = "saved-searches",
    params(("search" = String, Path, description = "The ID of the saved search.")),
    request_body = SearchMailBody,
    responses(
        (status = 200, description = "The e-mailed search.", body = SearchMailItem),
        InvalidAnswer,
        NotSignedInAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn set_search_mail(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(search): ApiPath<String>,
    body: Result<ApiJson<SearchMailBody>, ApiError>,
) -> Result<Response, ApiError> {
    let read = search_id(&search).and_then(|search| body.map(|ApiJson(body)| (search, body)));
    let (search, body) = auth.after_who_asks(&info, &session_token, read).await?;
    let set = auth
        .api()?
        .set_search_mail(
            info,
            SetSearchMail {
                session_token,
                search,
                days: body.days,
                hour: body.hour,
                only_when_changed: body.only_when_changed,
            },
        )
        .await?;
    answer_authorized(&set.map(|view| SearchMailItem::from(&view)))
}

/// Stops getting a saved search by e-mail.
#[utoipa::path(
    delete,
    path = "/api/v1/saved-searches/{search}/mail",
    operation_id = "stopSearchMail",
    tag = "saved-searches",
    params(("search" = String, Path, description = "The ID of the saved search.")),
    responses(
        (status = 204, description = "The person no longer gets it by e-mail."),
        NotSignedInAnswer,
        NotFoundAnswer
    )
)]
async fn stop_search_mail(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(search): ApiPath<String>,
) -> Result<Response, ApiError> {
    let search = auth
        .after_who_asks(&info, &session_token, search_id(&search))
        .await?;
    auth.api()?
        .stop_search_mail(
            info,
            StopSearchMail {
                session_token,
                search,
            },
        )
        .await?;
    Ok(done())
}

/// The routes of e-mailed searches.
pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(list_search_mails))
        .routes(routes!(set_search_mail, stop_search_mail))
}

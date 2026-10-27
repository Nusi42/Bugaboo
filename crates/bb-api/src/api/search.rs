//! The search routes (QRY-04, QRY-05, SEC-17): the issues a text of the search language finds in
//! the projects the person sees, how many there are, how many in each group of a field, and the
//! fields the person may search by. The project's list is the same search (`listIssues`).
//!
//! Every route counts against the limit of searches of the person (60 a minute) before anything
//! is read. A cursor is sealed for the person and the search it was made for (a hash of the text
//! and where it looks), so it opens nothing for anybody else or for another search. A mistake in
//! the text is answered as `search.invalid` with the key, the arguments and the bytes of each
//! mistake. The use cases are in `bb-app`, behind [`AuthApi`](bb_app::auth::AuthApi).

use std::sync::Arc;

use axum::extract::State;
use axum::response::Response;
use axum::{Extension, extract::FromRef};
use base64ct::{Base64UrlUnpadded, Encoding as _};
use bb_app::auth::RequestInfo;
use bb_app::issues::IssueView;
use bb_app::search::{
    GroupKey, NAME_CHOICES_MAX, NameChoiceKind, NameChoicesQuery, ReadableFieldsQuery,
    SearchCountQuery, SearchGroupsQuery, SearchPage, SearchQuery, SearchScope, SearcherQuery,
    ShownKey,
};
use bb_core::id::{ProjectId, UserId};
use bb_core::issue::{Priority, StatusGroup};
use bb_policy::{Authorized, Masked, PersonRef};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest as _, Sha256};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::access_bodies::cursor_error;
use super::auth::{AuthHandle, Info, InvalidAnswer, NotFoundAnswer, NotSignedInAnswer, Session};
use super::issues::{ISSUE_FIELDS, IssueItem};
use crate::errors::{ApiError, ApiQuery};
use crate::lean::{ListParams, Page};
use crate::limits::Limits;

/// What the search routes take besides the page.
#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct SearchParams {
    /// The search, as text of the search language (`priority is high and assignedTo is me`). Left
    /// out or empty, every issue the person may read is found.
    q: Option<String>,
    /// The ID of a project: the search looks in that project and the ones inside it that the person
    /// sees. Left out, it looks in every project they see.
    within: Option<String>,
}

/// What the grouping route takes.
#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct GroupParams {
    /// The word of the field to group by (`status`, `assignedTo`, `tags`, …).
    by: String,
}

/// How many issues a search finds.
#[derive(Serialize, ToSchema)]
pub(crate) struct SearchCountBody {
    /// The number of issues the person may read that the search finds.
    issues: u64,
}

/// The key of a group, by identity or word; a person as the viewer sees them.
#[derive(Serialize, ToSchema)]
#[serde(tag = "type", content = "value", rename_all = "kebab-case")]
pub(crate) enum SearchGroupKeyBody {
    /// The issues without a value.
    Empty,
    /// A text that is no person's: a status key, a text of an own field.
    Text(String),
    /// A number or a moment.
    Whole(i64),
    /// A person, as the viewer sees them.
    Person(PersonRef),
    /// A group of people, by its ID.
    Group(String),
    /// An issue, by its ID.
    Issue(String),
    /// A project, by its ID.
    Project(String),
    /// An issue type, by its ID.
    IssueType(String),
    /// A choice of an own field, by its ID.
    Choice(String),
    /// A priority.
    Priority(Priority),
    /// A status group.
    StatusGroup(StatusGroup),
    /// A tag, by its ID and its name.
    Tag(SearchNamedBody),
    /// A label, by its ID and its name.
    Label(SearchNamedBody),
}

/// A thing with a name the person may know.
#[derive(Serialize, ToSchema)]
pub(crate) struct SearchNamedBody {
    /// Its ID.
    id: String,
    /// Its name; left out where the person may not know it.
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
}

/// One group of a search.
#[derive(Serialize, ToSchema)]
pub(crate) struct SearchGroupBody {
    /// What the group is.
    key: SearchGroupKeyBody,
    /// How many issues the search finds in it. An issue with several values is in each group.
    issues: u64,
}

/// How many issues a search finds in each group of a field.
#[derive(Serialize, ToSchema)]
pub(crate) struct SearchGroupsBody {
    /// The groups, the one without a value last.
    groups: Vec<SearchGroupBody>,
    /// Whether the list was cut off at its cap: over 200 groups the answer holds the first 199
    /// and the group without a value.
    truncated: bool,
}

/// A field the person may search by.
#[derive(Serialize, ToSchema)]
pub(crate) struct SearchFieldBody {
    /// The word of the field.
    word: String,
    /// What kind of values it holds.
    kind: String,
    /// What may be asked of it.
    operators: Vec<String>,
    /// The values of a fixed list; empty when anything goes.
    choices: Vec<String>,
}

/// The fields the person may search by.
#[derive(Serialize, ToSchema)]
pub(crate) struct SearchFieldsBody {
    /// The fields, in the order they are offered.
    fields: Vec<SearchFieldBody>,
    /// Whether the list was cut off at its cap.
    truncated: bool,
}

/// The most fields one answer holds.
const MAX_FIELDS: usize = 500;

/// The name of a thing as text, from its serialized form (`snake_case` words).
fn word_of<T: Serialize>(thing: &T) -> String {
    match serde_json::to_value(thing) {
        Ok(Value::String(word)) => word,
        _ => String::new(),
    }
}

fn group_key(key: ShownKey) -> SearchGroupKeyBody {
    match key {
        ShownKey::Person(person) => SearchGroupKeyBody::Person(person),
        ShownKey::Tag { id, name } => SearchGroupKeyBody::Tag(SearchNamedBody {
            id: id.to_string(),
            name,
        }),
        ShownKey::Label { id, name } => SearchGroupKeyBody::Label(SearchNamedBody {
            id: id.to_string(),
            name,
        }),
        ShownKey::Other(other) => match other {
            GroupKey::Text(text) => SearchGroupKeyBody::Text(text),
            GroupKey::Whole(number) => SearchGroupKeyBody::Whole(number),
            GroupKey::Group(id) => SearchGroupKeyBody::Group(id.to_string()),
            GroupKey::Issue(id) => SearchGroupKeyBody::Issue(id.to_string()),
            GroupKey::Project(id) => SearchGroupKeyBody::Project(id.to_string()),
            GroupKey::IssueType(id) => SearchGroupKeyBody::IssueType(id.to_string()),
            GroupKey::Choice(id) => SearchGroupKeyBody::Choice(id.to_string()),
            GroupKey::Priority(each) => SearchGroupKeyBody::Priority(each),
            GroupKey::StatusGroup(each) => SearchGroupKeyBody::StatusGroup(each),
            // A person, a tag and a label are shown by the use case, never as a bare ID.
            GroupKey::Empty | GroupKey::Person(_) | GroupKey::Tag(_) | GroupKey::Label(_) => {
                SearchGroupKeyBody::Empty
            }
        },
    }
}

/// Who asks, after the check of the limit of searches: judged first, so a person who is not signed
/// in is refused as such (401) and the limit counts people.
async fn counted(
    auth: &AuthHandle,
    limits: &Limits,
    info: &RequestInfo,
    session_token: &str,
) -> Result<UserId, ApiError> {
    // A session or an access key: the person is the one the use cases resolve for the request.
    let person = auth
        .api()?
        .searcher(
            info.clone(),
            SearcherQuery {
                session_token: session_token.to_owned(),
            },
        )
        .await?;
    limits.check_search(person)?;
    Ok(person)
}

/// Where the search looks, from the `within` parameter.
fn scope_of(within: Option<&str>) -> Result<SearchScope, ApiError> {
    match within {
        None => Ok(SearchScope::Everywhere),
        Some(text) => text
            .parse::<ProjectId>()
            .map(SearchScope::Within)
            .map_err(|_| ApiError::new(bb_core::ErrorCode::InvalidRequest)),
    }
}

/// The name of the list a cursor is sealed for: the person and a hash of the search (its text and
/// where it looks), so a cursor opens for nobody else and for no other search.
fn list_of(person: UserId, text: &str, within: Option<&str>) -> String {
    let mut hash = Sha256::new();
    hash.update(text.as_bytes());
    hash.update([0]);
    hash.update(within.unwrap_or_default().as_bytes());
    format!(
        "search:{person}:{}",
        Base64UrlUnpadded::encode_string(&hash.finalize())
    )
}

/// The issues a search finds, a page at a time, in the order of the search (newest first without
/// `sort by`). Each issue is shown with what the person may see of it in its own project.
#[utoipa::path(
    get,
    path = "/api/v1/search/issues",
    operation_id = "searchIssues",
    tag = "search",
    params(SearchParams, ListParams),
    responses(
        (status = 200, description = "The issues found.", body = Page<IssueItem>),
        InvalidAnswer, NotSignedInAnswer, NotFoundAnswer
    )
)]
async fn search_issues(
    State(auth): State<AuthHandle>,
    Extension(limits): Extension<Arc<Limits>>,
    Info(info): Info,
    Session(session_token): Session,
    ApiQuery(search): ApiQuery<SearchParams>,
    ApiQuery(params): ApiQuery<ListParams>,
) -> Result<Response, ApiError> {
    let person = counted(&auth, &limits, &info, &session_token).await?;
    let text = search.q.unwrap_or_default();
    let list = list_of(person, &text, search.within.as_deref());
    let (cursor, bad_cursor) = params.start(&list);
    let page = auth
        .api()?
        .search(
            info,
            SearchQuery {
                session_token,
                scope: scope_of(search.within.as_deref())?,
                text,
                cursor,
                limit: params.limit(),
            },
        )
        .await
        .map_err(cursor_error)?;
    bad_cursor?;
    issue_page_answer(&list, page, &params)
}

/// The answer of a page of issues, each with the rights of its own project: the list of a project
/// and a search over many projects make it the same way.
pub(super) fn issue_page_answer(
    list: &str,
    page: Authorized<SearchPage>,
    params: &ListParams,
) -> Result<Response, ApiError> {
    let (items, next) = unpack(page);
    let items: Vec<Authorized<IssueItem>> = items
        .into_iter()
        .map(|each| each.map(|view| IssueItem::from(&view)))
        .collect();
    Ok(super::auth::answer(Page::of_authorized_items(
        list,
        &items,
        next.as_deref(),
        params,
        &ISSUE_FIELDS,
    )?))
}

/// The items of a page with the rights of their own project, and where the next page starts. The
/// outer wrapper holds the rights of the person for the whole organization and adds nothing to the
/// items, which each carry their own.
fn unpack(page: Authorized<SearchPage>) -> (Vec<Authorized<IssueView>>, Option<String>) {
    let mut taken = None;
    let _ = page.map(|each| taken = Some((each.items, each.next_cursor)));
    taken.unwrap_or_default()
}

/// How many issues a search finds.
#[utoipa::path(
    get,
    path = "/api/v1/search/issues/count",
    operation_id = "countSearchedIssues",
    tag = "search",
    params(SearchParams),
    responses(
        (status = 200, description = "How many issues the search finds.", body = SearchCountBody),
        InvalidAnswer, NotSignedInAnswer, NotFoundAnswer
    )
)]
async fn count_searched_issues(
    State(auth): State<AuthHandle>,
    Extension(limits): Extension<Arc<Limits>>,
    Info(info): Info,
    Session(session_token): Session,
    ApiQuery(search): ApiQuery<SearchParams>,
) -> Result<Response, ApiError> {
    counted(&auth, &limits, &info, &session_token).await?;
    let found = auth
        .api()?
        .search_count(
            info,
            SearchCountQuery {
                session_token,
                scope: scope_of(search.within.as_deref())?,
                text: search.q.unwrap_or_default(),
            },
        )
        .await?;
    super::auth::answer_authorized(&found.map(|count| SearchCountBody {
        issues: count.issues,
    }))
}

/// How many issues a search finds in each group of a field. A field the person may not read is
/// refused like a field that does not exist.
#[utoipa::path(
    get,
    path = "/api/v1/search/issues/groups",
    operation_id = "groupSearchedIssues",
    tag = "search",
    params(SearchParams, GroupParams),
    responses(
        (status = 200, description = "The groups and how many issues each holds.", body = SearchGroupsBody),
        InvalidAnswer, NotSignedInAnswer, NotFoundAnswer
    )
)]
async fn group_searched_issues(
    State(auth): State<AuthHandle>,
    Extension(limits): Extension<Arc<Limits>>,
    Info(info): Info,
    Session(session_token): Session,
    ApiQuery(search): ApiQuery<SearchParams>,
    ApiQuery(group): ApiQuery<GroupParams>,
) -> Result<Response, ApiError> {
    counted(&auth, &limits, &info, &session_token).await?;
    let found = auth
        .api()?
        .search_groups(
            info,
            SearchGroupsQuery {
                session_token,
                scope: scope_of(search.within.as_deref())?,
                text: search.q.unwrap_or_default(),
                by: group.by,
            },
        )
        .await?;
    super::auth::answer_authorized(&found.map(|shown| {
        SearchGroupsBody {
            truncated: shown.truncated,
            groups: shown
                .groups
                .into_iter()
                .map(|each| SearchGroupBody {
                    key: group_key(each.key),
                    issues: each.issues,
                })
                .collect(),
        }
    }))
}

/// The fields the person may search by where a search looks, the same the checker of the server
/// gets, so the editor of the browser checks a search against the same fields.
#[utoipa::path(
    get,
    path = "/api/v1/search/fields",
    operation_id = "listSearchFields",
    tag = "search",
    params(SearchParams),
    responses(
        (status = 200, description = "The fields.", body = SearchFieldsBody),
        InvalidAnswer, NotSignedInAnswer, NotFoundAnswer
    )
)]
async fn list_search_fields(
    State(auth): State<AuthHandle>,
    Extension(limits): Extension<Arc<Limits>>,
    Info(info): Info,
    Session(session_token): Session,
    ApiQuery(search): ApiQuery<SearchParams>,
) -> Result<Response, ApiError> {
    counted(&auth, &limits, &info, &session_token).await?;
    let found = auth
        .api()?
        .readable_fields(
            info,
            ReadableFieldsQuery {
                session_token,
                scope: scope_of(search.within.as_deref())?,
            },
        )
        .await?;
    super::auth::answer_authorized(&found.map(|fields| {
        let truncated = fields.0.fields.len() > MAX_FIELDS;
        SearchFieldsBody {
            truncated,
            fields: fields
                .0
                .fields
                .into_iter()
                .take(MAX_FIELDS)
                .map(|field| SearchFieldBody {
                    kind: word_of(&field.kind),
                    operators: field.operators.iter().map(word_of).collect(),
                    word: field.word,
                    choices: field.choices,
                })
                .collect(),
        }
    }))
}

/// Which names are asked for.
#[derive(Clone, Copy, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum NamesKind {
    /// People, for the fields that hold a person.
    People,
    /// Groups, for `member of`.
    Groups,
}

/// What the names route takes.
#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct NamesParams {
    /// People or groups.
    #[param(inline)]
    kind: NamesKind,
    /// The start of the names; left out, the first ones.
    starting_with: Option<String>,
    /// How many names at most (default 8, at most 20).
    limit: Option<usize>,
}

/// The names the person may use in a search, in the order of the names.
#[derive(Serialize, ToSchema)]
pub(crate) struct SearchNamesBody {
    /// The names.
    names: Vec<String>,
    /// Whether the list was cut off at the number asked for.
    truncated: bool,
}

/// The most characters of the start of a name that are looked for.
const MAX_NAME_START: usize = 100;

/// The number of names when the request does not say.
const NAMES_DEFAULT: usize = 8;

/// The names of people or groups that the person may use in a search and that start with some
/// words: what a suggestion offers. A person the searcher may not name is not in the list, as one
/// who does not exist; the rule is the one that accepts or refuses a search that names them.
#[utoipa::path(
    get,
    path = "/api/v1/search/names",
    operation_id = "listSearchNames",
    tag = "search",
    params(NamesParams),
    responses(
        (status = 200, description = "The names.", body = SearchNamesBody),
        InvalidAnswer, NotSignedInAnswer
    )
)]
async fn list_search_names(
    State(auth): State<AuthHandle>,
    Extension(limits): Extension<Arc<Limits>>,
    Info(info): Info,
    Session(session_token): Session,
    ApiQuery(params): ApiQuery<NamesParams>,
) -> Result<Response, ApiError> {
    counted(&auth, &limits, &info, &session_token).await?;
    let wanted = params
        .limit
        .unwrap_or(NAMES_DEFAULT)
        .clamp(1, NAME_CHOICES_MAX);
    let starting_with = params.starting_with.unwrap_or_default();
    if starting_with.chars().count() > MAX_NAME_START {
        return Err(ApiError::new(bb_core::ErrorCode::InvalidRequest));
    }
    let found = auth
        .api()?
        .name_choices(
            info,
            NameChoicesQuery {
                session_token,
                kind: match params.kind {
                    NamesKind::People => NameChoiceKind::People,
                    NamesKind::Groups => NameChoiceKind::Groups,
                },
                starting_with,
                limit: wanted + 1,
            },
        )
        .await?;
    super::auth::answer_authorized(&found.map(|choices| SearchNamesBody {
        truncated: choices.0.len() > wanted,
        names: choices.0.into_iter().take(wanted).collect(),
    }))
}

impl Masked for SearchNamesBody {}

impl Masked for SearchCountBody {}
impl Masked for SearchGroupsBody {}
impl Masked for SearchFieldsBody {}

/// The routes of searching, with the limit of searches of the person.
pub(crate) fn routes<S>(limits: &Arc<Limits>) -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(search_issues))
        .routes(routes!(count_searched_issues))
        .routes(routes!(group_searched_issues))
        .routes(routes!(list_search_fields))
        .routes(routes!(list_search_names))
        .layer(Extension(Arc::clone(limits)))
}

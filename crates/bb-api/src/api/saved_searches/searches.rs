//! The routes of saved searches below `/api/v1/saved-searches`.

use axum::extract::State;
use axum::response::Response;
use bb_core::ErrorCode;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::super::access_bodies::cursor_error;
use super::super::auth::{
    AuthHandle, ConflictAnswer, ForbiddenAnswer, Info, InvalidAnswer, NotFoundAnswer,
    NotSignedInAnswer, Session, answer, answer_authorized, done,
};
use super::super::path_id;
use super::{
    CHOICE_FIELDS, CHOICES_LIST, ChangeSavedSearchBody, ChangeSearch, CopyBody, DeleteSearch,
    KindParams, NewSavedSearchBody, OptionalSavedSearchBody, Presentation, RestoreSearch,
    SEARCH_FIELDS, SEARCHES_LIST, SaveCopy, SaveSearch, SavedSearchId, SavedSearchItem,
    SavedSearchQuery, ScopeParams, SearchSet, SearchesQuery, ShareBody, ShareChoiceItem,
    ShareChoicesQuery, ShareKind, ShareSearch, ShareTarget, StopSharingSearch,
};
use crate::errors::{ApiError, ApiJson, ApiPath, ApiQuery};
use crate::lean::{ListParams, Listed, Page};

fn search_id(text: &str) -> Result<SavedSearchId, ApiError> {
    path_id::<SavedSearchId>(text)
}

fn target(kind: ShareKind, id: &str) -> Result<ShareTarget, ApiError> {
    ShareTarget::parse(kind, id).ok_or_else(|| ApiError::new(ErrorCode::InvalidRequest))
}

/// The saved searches the person made, or the ones shared with them, a page at a time, oldest
/// first.
#[utoipa::path(
    get,
    path = "/api/v1/saved-searches",
    operation_id = "listSavedSearches",
    tag = "saved-searches",
    params(ListParams, ScopeParams),
    responses((status = 200, description = "The saved searches.", body = Page<SavedSearchItem>), NotSignedInAnswer)
)]
async fn list_saved_searches(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    params: Result<ApiQuery<ListParams>, ApiError>,
    which: Result<ApiQuery<ScopeParams>, ApiError>,
) -> Result<Response, ApiError> {
    let read = params.and_then(|params| which.map(|which| (params, which)));
    let (ApiQuery(params), ApiQuery(which)) =
        auth.after_who_asks(&info, &session_token, read).await?;
    let (cursor, bad_cursor) = params.start(SEARCHES_LIST);
    let page = auth
        .api()?
        .saved_searches(
            info,
            SearchesQuery {
                session_token,
                scope: which.scope.unwrap_or(SearchSet::Mine),
                cursor,
                limit: params.limit(),
            },
        )
        .await
        .map_err(cursor_error)?;
    bad_cursor?;
    let list = page.map(|page| Listed {
        items: page.items.iter().map(SavedSearchItem::from).collect(),
        next: page.next_cursor,
    });
    let fields = params.fields_readable(&SEARCH_FIELDS, &list.hidden_properties())?;
    Ok(answer(Page::of_authorized(SEARCHES_LIST, &list, &fields)?))
}

/// Saves a search under a name. It is private and the person's own.
#[utoipa::path(
    post,
    path = "/api/v1/saved-searches",
    operation_id = "createSavedSearch",
    tag = "saved-searches",
    request_body = NewSavedSearchBody,
    responses(
        (status = 200, description = "The saved search.", body = SavedSearchItem),
        InvalidAnswer,
        NotSignedInAnswer
    )
)]
async fn create_saved_search(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    body: Result<ApiJson<NewSavedSearchBody>, ApiError>,
) -> Result<Response, ApiError> {
    let read = body.map(|ApiJson(body)| {
        let presentation = body.presentation.map(Presentation::from);
        (body.name, body.search_text, presentation)
    });
    let (name, text, presentation) = auth.after_who_asks(&info, &session_token, read).await?;
    let made = auth
        .api()?
        .save_search(
            info,
            SaveSearch {
                session_token,
                name,
                text,
                presentation: presentation.unwrap_or_default(),
            },
        )
        .await?;
    answer_authorized(&made.map(|view| SavedSearchItem::from(&view)))
}

/// Opens a saved search: the person's own, or one shared with them. For anybody else, and for one
/// that does not exist or is in the trash, the answer is `not found`.
#[utoipa::path(
    get,
    path = "/api/v1/saved-searches/{search}",
    operation_id = "getSavedSearch",
    tag = "saved-searches",
    params(("search" = String, Path, description = "The ID of the saved search.")),
    responses(
        (status = 200, description = "The saved search.", body = SavedSearchItem),
        NotSignedInAnswer,
        NotFoundAnswer
    )
)]
async fn get_saved_search(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(search): ApiPath<String>,
) -> Result<Response, ApiError> {
    let search = auth
        .after_who_asks(&info, &session_token, search_id(&search))
        .await?;
    let found = auth
        .api()?
        .saved_search(
            info,
            SavedSearchQuery {
                session_token,
                search,
            },
        )
        .await?;
    answer_authorized(&found.map(|view| SavedSearchItem::from(&view)))
}

/// Changes a saved search. Only its maker does.
#[utoipa::path(
    patch,
    path = "/api/v1/saved-searches/{search}",
    operation_id = "changeSavedSearch",
    tag = "saved-searches",
    params(("search" = String, Path, description = "The ID of the saved search.")),
    request_body = ChangeSavedSearchBody,
    responses(
        (status = 200, description = "The saved search as it is now.", body = SavedSearchItem),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn change_saved_search(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(search): ApiPath<String>,
    body: Result<ApiJson<ChangeSavedSearchBody>, ApiError>,
) -> Result<Response, ApiError> {
    let read = search_id(&search).and_then(|search| {
        let ApiJson(body) = body?;
        let presentation = body.presentation.map(Presentation::from);
        Ok((
            search,
            body.version,
            body.name,
            body.search_text,
            presentation,
        ))
    });
    let (search, version, name, text, presentation) =
        auth.after_who_asks(&info, &session_token, read).await?;
    let changed = auth
        .api()?
        .change_search(
            info,
            ChangeSearch {
                session_token,
                search,
                version,
                name,
                text,
                presentation,
            },
        )
        .await?;
    answer_authorized(&changed.map(|view| SavedSearchItem::from(&view)))
}

/// Puts a saved search into the trash (*Undo* takes it out again). Its maker may, and so may
/// whoever may change organization settings.
#[utoipa::path(
    delete,
    path = "/api/v1/saved-searches/{search}",
    operation_id = "deleteSavedSearch",
    tag = "saved-searches",
    params(("search" = String, Path, description = "The ID of the saved search.")),
    responses(
        (status = 204, description = "The saved search is in the trash."),
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer
    )
)]
async fn delete_saved_search(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(search): ApiPath<String>,
) -> Result<Response, ApiError> {
    let search = auth
        .after_who_asks(&info, &session_token, search_id(&search))
        .await?;
    auth.api()?
        .delete_search(
            info,
            DeleteSearch {
                session_token,
                search,
            },
        )
        .await?;
    Ok(done())
}

/// Takes a saved search out of the trash (*Undo*).
#[utoipa::path(
    post,
    path = "/api/v1/saved-searches/{search}/restore",
    operation_id = "restoreSavedSearch",
    tag = "saved-searches",
    params(("search" = String, Path, description = "The ID of the saved search.")),
    responses(
        (status = 200, description = "The saved search is out of the trash; the search is in the answer when it is one the person may open.", body = OptionalSavedSearchBody),
        NotSignedInAnswer,
        NotFoundAnswer
    )
)]
async fn restore_saved_search(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(search): ApiPath<String>,
) -> Result<Response, ApiError> {
    let search = auth
        .after_who_asks(&info, &session_token, search_id(&search))
        .await?;
    let restored = auth
        .api()?
        .restore_search(
            info,
            RestoreSearch {
                session_token,
                search,
            },
        )
        .await?;
    answer_authorized(&restored.map(|restored| OptionalSavedSearchBody::from(&restored)))
}

/// *Save a copy*: a private saved search of the person with the text and the presentation of one
/// they may open.
#[utoipa::path(
    post,
    path = "/api/v1/saved-searches/{search}/copy",
    operation_id = "copySavedSearch",
    tag = "saved-searches",
    params(("search" = String, Path, description = "The ID of the saved search.")),
    request_body = CopyBody,
    responses(
        (status = 200, description = "The copy.", body = SavedSearchItem),
        InvalidAnswer,
        NotSignedInAnswer,
        NotFoundAnswer
    )
)]
async fn copy_saved_search(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(search): ApiPath<String>,
    body: Result<ApiJson<CopyBody>, ApiError>,
) -> Result<Response, ApiError> {
    let read = search_id(&search).and_then(|search| body.map(|ApiJson(body)| (search, body.name)));
    let (search, name) = auth.after_who_asks(&info, &session_token, read).await?;
    let copy = auth
        .api()?
        .save_copy(
            info,
            SaveCopy {
                session_token,
                search,
                name,
            },
        )
        .await?;
    answer_authorized(&copy.map(|view| SavedSearchItem::from(&view)))
}

/// Shares a saved search with a group or a project. Only its maker does, with *Share searches* in
/// the project; for a group, by being in it and holding *Share searches* for the whole
/// organization, or with *Manage groups*. Sharing it again changes nothing.
#[utoipa::path(
    post,
    path = "/api/v1/saved-searches/{search}/shares",
    operation_id = "shareSavedSearch",
    tag = "saved-searches",
    params(("search" = String, Path, description = "The ID of the saved search.")),
    request_body = ShareBody,
    responses(
        (status = 200, description = "The saved search as it is now.", body = SavedSearchItem),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer
    )
)]
async fn share_saved_search(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(search): ApiPath<String>,
    body: Result<ApiJson<ShareBody>, ApiError>,
) -> Result<Response, ApiError> {
    let read = search_id(&search).and_then(|search| {
        let ApiJson(body) = body?;
        Ok((search, target(body.kind, &body.id)?))
    });
    let (search, target) = auth.after_who_asks(&info, &session_token, read).await?;
    let shared = auth
        .api()?
        .share_search(
            info,
            ShareSearch {
                session_token,
                search,
                target,
            },
        )
        .await?;
    answer_authorized(&shared.map(|view| SavedSearchItem::from(&view)))
}

/// Stops sharing a saved search with a group or a project. Only its maker does.
#[utoipa::path(
    delete,
    path = "/api/v1/saved-searches/{search}/shares/{kind}/{id}",
    operation_id = "stopSharingSavedSearch",
    tag = "saved-searches",
    params(
        ("search" = String, Path, description = "The ID of the saved search."),
        ("kind" = ShareKind, Path, description = "A group or a project."),
        ("id" = String, Path, description = "The ID of the group or the project.")
    ),
    responses(
        (status = 200, description = "The saved search as it is now.", body = SavedSearchItem),
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer
    )
)]
async fn stop_sharing_saved_search(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath((search, kind, id)): ApiPath<(String, String, String)>,
) -> Result<Response, ApiError> {
    let read = search_id(&search).and_then(|search| {
        let kind = ShareKind::from_code(&kind).ok_or_else(|| ApiError::new(ErrorCode::NotFound))?;
        let target =
            ShareTarget::parse(kind, &id).ok_or_else(|| ApiError::new(ErrorCode::NotFound))?;
        Ok((search, target))
    });
    let (search, target) = auth.after_who_asks(&info, &session_token, read).await?;
    let stopped = auth
        .api()?
        .stop_sharing_search(
            info,
            StopSharingSearch {
                session_token,
                search,
                target,
            },
        )
        .await?;
    answer_authorized(&stopped.map(|view| SavedSearchItem::from(&view)))
}

/// The groups or the projects the person may share a saved search with, by name, a page at a
/// time: what a *Share* window lists.
#[utoipa::path(
    get,
    path = "/api/v1/saved-searches/share-choices",
    operation_id = "listShareChoices",
    tag = "saved-searches",
    params(ListParams, KindParams),
    responses((status = 200, description = "What may be shared with.", body = Page<ShareChoiceItem>), NotSignedInAnswer)
)]
async fn list_share_choices(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    params: Result<ApiQuery<ListParams>, ApiError>,
    which: Result<ApiQuery<KindParams>, ApiError>,
) -> Result<Response, ApiError> {
    let read = params.and_then(|params| which.map(|which| (params, which)));
    let (ApiQuery(params), ApiQuery(which)) =
        auth.after_who_asks(&info, &session_token, read).await?;
    let (cursor, bad_cursor) = params.start(CHOICES_LIST);
    let page = auth
        .api()?
        .share_choices(
            info,
            ShareChoicesQuery {
                session_token,
                kind: which.kind,
                cursor,
                limit: params.limit(),
            },
        )
        .await
        .map_err(cursor_error)?;
    bad_cursor?;
    let list = page.map(|page| Listed {
        items: page.items.iter().map(ShareChoiceItem::from).collect(),
        next: page.next_cursor,
    });
    let fields = params.fields_readable(&CHOICE_FIELDS, &list.hidden_properties())?;
    Ok(answer(Page::of_authorized(CHOICES_LIST, &list, &fields)?))
}

/// The routes of saved searches.
pub(super) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(list_saved_searches, create_saved_search))
        .routes(routes!(list_share_choices))
        .routes(routes!(
            get_saved_search,
            change_saved_search,
            delete_saved_search
        ))
        .routes(routes!(restore_saved_search))
        .routes(routes!(copy_saved_search))
        .routes(routes!(share_saved_search))
        .routes(routes!(stop_sharing_saved_search))
}

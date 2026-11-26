//! The routes that list, open, create, change, share, copy and delete dashboards, below
//! `/api/v1/dashboards`.

use axum::extract::State;
use axum::response::Response;
use bb_app::dashboards::{
    ChangeDashboard, CopyDashboard, CreateDashboard, DashboardQuery, DashboardsQuery,
    DeleteDashboard, OpenHome, RestoreDashboard, ShareDashboard, StopSharingDashboard,
};
use bb_app::saved_searches::Shareable;
use bb_core::ErrorCode;
use bb_core::saved_search::{ShareKind, ShareTarget};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::super::access_bodies::cursor_error;
use super::super::auth::{
    AuthHandle, ConflictAnswer, ForbiddenAnswer, Info, InvalidAnswer, NotFoundAnswer,
    NotSignedInAnswer, Session, answer, answer_authorized, done,
};
use super::super::saved_searches::{
    CopyBody, KindParams, ScopeParams, ShareBody, ShareChoiceItem, scoped_list,
    share_choices_answer,
};
use super::{
    BoxBody, ChangeDashboardBody, DASHBOARD_FIELDS, DASHBOARDS_LIST, DashboardBody,
    DashboardSummary, NewDashboardBody, OptionalDashboardBody, dashboard_id,
};
use crate::errors::{ApiError, ApiJson, ApiPath, ApiQuery};
use crate::lean::{ListParams, Listed, Page};

fn target(kind: ShareKind, id: &str) -> Result<ShareTarget, ApiError> {
    ShareTarget::parse(kind, id).ok_or_else(|| ApiError::new(ErrorCode::InvalidRequest))
}

/// The dashboards the person made (not *Home*), or the ones shared with them, a page at a time,
/// oldest first.
#[utoipa::path(
    get,
    path = "/api/v1/dashboards",
    operation_id = "listDashboards",
    tag = "dashboards",
    params(ListParams, ScopeParams),
    responses((status = 200, description = "The dashboards.", body = Page<DashboardSummary>), NotSignedInAnswer)
)]
async fn list_dashboards(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    params: Result<ApiQuery<ListParams>, ApiError>,
    which: Result<ApiQuery<ScopeParams>, ApiError>,
) -> Result<Response, ApiError> {
    let (params, scope) = scoped_list(&auth, &info, &session_token, params, which).await?;
    let (cursor, bad_cursor) = params.start(DASHBOARDS_LIST);
    let page = auth
        .api()?
        .dashboards(
            info,
            DashboardsQuery {
                session_token,
                scope,
                cursor,
                limit: params.limit(),
            },
        )
        .await
        .map_err(cursor_error)?;
    bad_cursor?;
    let list = page.map(|page| Listed {
        items: page.items.iter().map(DashboardSummary::from).collect(),
        next: page.next_cursor,
    });
    let fields = params.fields_readable(&DASHBOARD_FIELDS, &list.hidden_properties())?;
    Ok(answer(Page::of_authorized(
        DASHBOARDS_LIST,
        &list,
        &fields,
    )?))
}

/// Creates a dashboard. It is private and the person's own; every box must check for them.
#[utoipa::path(
    post,
    path = "/api/v1/dashboards",
    operation_id = "createDashboard",
    tag = "dashboards",
    request_body = NewDashboardBody,
    responses(
        (status = 200, description = "The dashboard.", body = DashboardBody),
        InvalidAnswer,
        ConflictAnswer,
        NotSignedInAnswer
    )
)]
async fn create_dashboard(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    body: Result<ApiJson<NewDashboardBody>, ApiError>,
) -> Result<Response, ApiError> {
    let read = body.and_then(|ApiJson(body)| {
        let boxes = BoxBody::read_all(body.boxes)?;
        Ok((body.name, boxes))
    });
    let (name, boxes) = auth.after_who_asks(&info, &session_token, read).await?;
    let made = auth
        .api()?
        .create_dashboard(
            info,
            CreateDashboard {
                session_token,
                name,
                boxes: boxes.into_iter().map(|each| each.spec).collect(),
            },
        )
        .await?;
    answer_authorized(&made.map(|view| DashboardBody::from(&view)))
}

/// The person's *Home*: their own dashboard, made at first use from the blocks *Home* showed
/// before ("My timer", "My open issues", "Due soon"). Nobody else reaches it.
#[utoipa::path(
    get,
    path = "/api/v1/dashboards/home",
    operation_id = "getHome",
    tag = "dashboards",
    responses(
        (status = 200, description = "The person's Home.", body = DashboardBody),
        NotSignedInAnswer
    )
)]
async fn get_home(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
) -> Result<Response, ApiError> {
    auth.after_who_asks(&info, &session_token, Ok(())).await?;
    let home = auth.api()?.home(info, OpenHome { session_token }).await?;
    answer_authorized(&home.map(|view| DashboardBody::from(&view)))
}

/// Opens a dashboard: the person's own, or one shared with them, with the boxes that check for
/// them, packed upwards. For anybody else, and for one that does not exist or is in the trash, the
/// answer is `not found`.
#[utoipa::path(
    get,
    path = "/api/v1/dashboards/{dashboard}",
    operation_id = "getDashboard",
    tag = "dashboards",
    params(("dashboard" = String, Path, description = "The ID of the dashboard.")),
    responses(
        (status = 200, description = "The dashboard.", body = DashboardBody),
        NotSignedInAnswer,
        NotFoundAnswer
    )
)]
async fn get_dashboard(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(dashboard): ApiPath<String>,
) -> Result<Response, ApiError> {
    let dashboard = auth
        .after_who_asks(&info, &session_token, dashboard_id(&dashboard))
        .await?;
    let found = auth
        .api()?
        .dashboard(
            info,
            DashboardQuery {
                session_token,
                dashboard,
            },
        )
        .await?;
    answer_authorized(&found.map(|view| DashboardBody::from(&view)))
}

/// Changes the name or the boxes of a dashboard (*Change*, on *Home* *Customize*). Only its maker
/// does; a change made on an older version is refused with the newer one.
#[utoipa::path(
    patch,
    path = "/api/v1/dashboards/{dashboard}",
    operation_id = "changeDashboard",
    tag = "dashboards",
    params(("dashboard" = String, Path, description = "The ID of the dashboard.")),
    request_body = ChangeDashboardBody,
    responses(
        (status = 200, description = "The dashboard as it is now.", body = DashboardBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn change_dashboard(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(dashboard): ApiPath<String>,
    body: Result<ApiJson<ChangeDashboardBody>, ApiError>,
) -> Result<Response, ApiError> {
    let read = dashboard_id(&dashboard).and_then(|dashboard| {
        let ApiJson(body) = body?;
        let boxes = body.boxes.map(BoxBody::read_all).transpose()?;
        Ok((dashboard, body.version, body.name, boxes))
    });
    let (dashboard, version, name, boxes) =
        auth.after_who_asks(&info, &session_token, read).await?;
    let changed = auth
        .api()?
        .change_dashboard(
            info,
            ChangeDashboard {
                session_token,
                dashboard,
                version,
                name,
                boxes,
            },
        )
        .await?;
    answer_authorized(&changed.map(|view| DashboardBody::from(&view)))
}

/// Puts a dashboard into the trash: its maker, or a person with *Change organization settings*.
/// *Home* is not deleted.
#[utoipa::path(
    delete,
    path = "/api/v1/dashboards/{dashboard}",
    operation_id = "deleteDashboard",
    tag = "dashboards",
    params(("dashboard" = String, Path, description = "The ID of the dashboard.")),
    responses(
        (status = 204, description = "The dashboard is in the trash."),
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer
    )
)]
async fn delete_dashboard(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(dashboard): ApiPath<String>,
) -> Result<Response, ApiError> {
    let dashboard = auth
        .after_who_asks(&info, &session_token, dashboard_id(&dashboard))
        .await?;
    auth.api()?
        .delete_dashboard(
            info,
            DeleteDashboard {
                session_token,
                dashboard,
            },
        )
        .await?;
    Ok(done())
}

/// Takes a dashboard out of the trash (*Undo*).
#[utoipa::path(
    post,
    path = "/api/v1/dashboards/{dashboard}/restore",
    operation_id = "restoreDashboard",
    tag = "dashboards",
    params(("dashboard" = String, Path, description = "The ID of the dashboard.")),
    responses(
        (status = 200, description = "The dashboard is out of the trash; it is in the answer when it is one the person may open.", body = OptionalDashboardBody),
        NotSignedInAnswer,
        NotFoundAnswer
    )
)]
async fn restore_dashboard(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(dashboard): ApiPath<String>,
) -> Result<Response, ApiError> {
    let dashboard = auth
        .after_who_asks(&info, &session_token, dashboard_id(&dashboard))
        .await?;
    let restored = auth
        .api()?
        .restore_dashboard(
            info,
            RestoreDashboard {
                session_token,
                dashboard,
            },
        )
        .await?;
    answer_authorized(&restored.map(|restored| OptionalDashboardBody::from(&restored)))
}

/// *Save a copy*: a private dashboard of the person with the boxes they see of one they may open.
#[utoipa::path(
    post,
    path = "/api/v1/dashboards/{dashboard}/copy",
    operation_id = "copyDashboard",
    tag = "dashboards",
    params(("dashboard" = String, Path, description = "The ID of the dashboard.")),
    request_body = CopyBody,
    responses(
        (status = 200, description = "The copy.", body = DashboardBody),
        InvalidAnswer,
        NotSignedInAnswer,
        NotFoundAnswer
    )
)]
async fn copy_dashboard(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(dashboard): ApiPath<String>,
    body: Result<ApiJson<CopyBody>, ApiError>,
) -> Result<Response, ApiError> {
    let read = dashboard_id(&dashboard)
        .and_then(|dashboard| body.map(|ApiJson(body)| (dashboard, body.name)));
    let (dashboard, name) = auth.after_who_asks(&info, &session_token, read).await?;
    let copy = auth
        .api()?
        .copy_dashboard(
            info,
            CopyDashboard {
                session_token,
                dashboard,
                name,
            },
        )
        .await?;
    answer_authorized(&copy.map(|view| DashboardBody::from(&view)))
}

/// Shares a dashboard with a group or a project: *Share dashboards* in the project; for a group,
/// being in it with *Share dashboards* for the organization, or *Manage groups*.
#[utoipa::path(
    post,
    path = "/api/v1/dashboards/{dashboard}/shares",
    operation_id = "shareDashboard",
    tag = "dashboards",
    params(("dashboard" = String, Path, description = "The ID of the dashboard.")),
    request_body = ShareBody,
    responses(
        (status = 200, description = "The dashboard as it is now.", body = DashboardBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer
    )
)]
async fn share_dashboard(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(dashboard): ApiPath<String>,
    body: Result<ApiJson<ShareBody>, ApiError>,
) -> Result<Response, ApiError> {
    let read = dashboard_id(&dashboard).and_then(|dashboard| {
        let ApiJson(body) = body?;
        Ok((dashboard, target(body.kind, &body.id)?))
    });
    let (dashboard, target) = auth.after_who_asks(&info, &session_token, read).await?;
    let shared = auth
        .api()?
        .share_dashboard(
            info,
            ShareDashboard {
                session_token,
                dashboard,
                target,
            },
        )
        .await?;
    answer_authorized(&shared.map(|view| DashboardBody::from(&view)))
}

/// Stops sharing a dashboard with a group or a project. Only its maker does.
#[utoipa::path(
    delete,
    path = "/api/v1/dashboards/{dashboard}/shares/{kind}/{id}",
    operation_id = "stopSharingDashboard",
    tag = "dashboards",
    params(
        ("dashboard" = String, Path, description = "The ID of the dashboard."),
        ("kind" = ShareKind, Path, description = "A group or a project."),
        ("id" = String, Path, description = "The ID of the group or the project.")
    ),
    responses(
        (status = 200, description = "The dashboard as it is now.", body = DashboardBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer
    )
)]
async fn stop_sharing_dashboard(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath((dashboard, kind, id)): ApiPath<(String, String, String)>,
) -> Result<Response, ApiError> {
    // The parts of the address are read after the caller is known, so an access key is turned
    // away first, whatever the address holds; a place that is none is not found.
    let read = dashboard_id(&dashboard).and_then(|dashboard| {
        let kind = ShareKind::from_code(&kind).ok_or_else(|| ApiError::new(ErrorCode::NotFound))?;
        let target =
            ShareTarget::parse(kind, &id).ok_or_else(|| ApiError::new(ErrorCode::NotFound))?;
        Ok((dashboard, target))
    });
    let (dashboard, target) = auth.after_who_asks(&info, &session_token, read).await?;
    let stopped = auth
        .api()?
        .stop_sharing_dashboard(
            info,
            StopSharingDashboard {
                session_token,
                dashboard,
                target,
            },
        )
        .await?;
    answer_authorized(&stopped.map(|view| DashboardBody::from(&view)))
}

/// The groups or the projects the person may share a dashboard with, by name, a page at a time:
/// what the *Share* window of a dashboard lists.
#[utoipa::path(
    get,
    path = "/api/v1/dashboards/share-choices",
    operation_id = "listDashboardShareChoices",
    tag = "dashboards",
    params(ListParams, KindParams),
    responses((status = 200, description = "What may be shared with.", body = Page<ShareChoiceItem>), NotSignedInAnswer)
)]
async fn list_dashboard_share_choices(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    params: Result<ApiQuery<ListParams>, ApiError>,
    which: Result<ApiQuery<KindParams>, ApiError>,
) -> Result<Response, ApiError> {
    share_choices_answer(
        (&auth, info, session_token),
        params,
        which,
        Shareable::Dashboards,
    )
    .await
}

/// The routes of the dashboards themselves.
pub(super) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(list_dashboards, create_dashboard))
        .routes(routes!(get_home))
        .routes(routes!(list_dashboard_share_choices))
        .routes(routes!(get_dashboard, change_dashboard, delete_dashboard))
        .routes(routes!(restore_dashboard))
        .routes(routes!(copy_dashboard))
        .routes(routes!(share_dashboard))
        .routes(routes!(stop_sharing_dashboard))
}

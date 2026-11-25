//! What the boxes of a dashboard answer: one box of a dashboard, the preview of a box before it is
//! added, and the ready-made boxes (ADR-0050, decisions 3 and 4). Each box answers in the shape of
//! the route that exists for its read (the issues of a search, the totals of a search, the person's
//! time entries, the timeline, the timer, the history), so the screen draws a box with the parts it
//! has for those routes. Reading a box counts against the limit of searches, like those routes.

use std::sync::Arc;

use axum::Extension;
use axum::extract::State;
use axum::response::Response;
use bb_app::dashboards::{BoxAnswer, BoxPreviewQuery, BoxQuery, ReadyMadeQuery, RecentChanges};
use bb_core::ErrorCode;
use bb_core::id::DashboardBoxId;
use bb_policy::{Authorized, Masked};
use serde::Serialize;
use serde_json::Value;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::super::auth::{
    AuthHandle, Info, InvalidAnswer, NotFoundAnswer, NotSignedInAnswer, Session, answer_authorized,
};
use super::super::history::{HistoryEntryItem, items_of};
use super::super::issue_item::IssueItem;
use super::super::search::counted;
use super::super::search_totals::SearchTotalsAnswer;
use super::super::time::TimerItem;
use super::super::time_entries::TimeEntryItem;
use super::super::timeline::{TimelinePageBody, page_body};
use super::{BoxBody, BoxItem, ReadyMadeBody, ReadyMadeItem, dashboard_id};
use crate::errors::{ApiError, ApiJson, ApiPath};
use crate::limits::Limits;

/// The name the answer of a timeline in a box is sealed under: a box shows one page and gives no
/// cursor, so the name only keeps it apart from every list that does.
const BOX_TIMELINE: &str = "dashboard-box-timeline";

/// One of the newest changes of an issue.
#[derive(Serialize, ToSchema)]
pub(crate) struct RecentChangeBody {
    /// The issue, by its readable ID.
    issue: String,
    /// The entries of the row of its history, as the history of the issue gives them.
    entries: Vec<HistoryEntryItem>,
}

/// What a box answers, by the read of its kind.
#[derive(Serialize, ToSchema)]
#[serde(tag = "answer", rename_all = "snake_case")]
pub(crate) enum BoxAnswerBody {
    /// *Search results*: the first issues of the search, each with what the person may see of it.
    Issues {
        /// The issues, in the order of the search.
        #[schema(value_type = Vec<IssueItem>)]
        issues: Vec<Value>,
    },
    /// *Numbers and chart* and *Time totals* of a search: the totals, as the totals of a search
    /// answer them, groups in the order of their keys (people by their names).
    Totals {
        /// The totals.
        totals: SearchTotalsAnswer,
    },
    /// *My time this week*: the person's own time entries of this week.
    Week {
        /// The entries.
        #[schema(value_type = Vec<TimeEntryItem>)]
        entries: Vec<Value>,
    },
    /// *Small timeline*: one page of the timeline of the search; nothing is dragged.
    Timeline {
        /// The page.
        timeline: TimelinePageBody,
    },
    /// *My timer*: the person's own timer.
    Timer {
        /// The timer.
        timer: TimerItem,
    },
    /// *Recent changes*: the newest changes of the issues the search finds, newest first.
    Changes {
        /// The changes.
        changes: Vec<RecentChangeBody>,
    },
}

impl Masked for BoxAnswerBody {}

/// Each item with what the person may see of it, as JSON.
fn masked<T, U: Masked + Serialize>(
    items: Vec<Authorized<T>>,
    shape: impl Fn(&T) -> U,
) -> Result<Vec<Value>, ApiError> {
    items
        .into_iter()
        .map(|item| {
            item.map(|view| shape(&view))
                .masked_value()
                .map_err(|error| ApiError::internal(&error))
        })
        .collect()
}

/// The changes, each row of its history as the history route gives it.
fn changes_body(changes: &RecentChanges) -> Vec<RecentChangeBody> {
    changes
        .items
        .iter()
        .map(|change| RecentChangeBody {
            issue: change.issue.clone(),
            entries: items_of(&change.item),
        })
        .collect()
}

/// The answer of a box. The outer wrapper of a read holds the rights of the person for the whole
/// organization; the items of a page each carry their own and are masked by them.
fn box_answer(found: BoxAnswer) -> Result<Response, ApiError> {
    let mut failed = None;
    let mut fail = |error: ApiError| failed = Some(error);
    let body = match found {
        BoxAnswer::Issues(page) => page.map(|page| BoxAnswerBody::Issues {
            issues: masked(page.items, |view| IssueItem::from(view)).unwrap_or_else(|error| {
                fail(error);
                Vec::new()
            }),
        }),
        BoxAnswer::Totals(totals) => totals.map(|totals| BoxAnswerBody::Totals {
            totals: SearchTotalsAnswer::of(totals, None),
        }),
        BoxAnswer::Week(page) => page.map(|page| BoxAnswerBody::Week {
            entries: masked(page.items, |view| TimeEntryItem::from(view)).unwrap_or_else(|error| {
                fail(error);
                Vec::new()
            }),
        }),
        BoxAnswer::Timeline(page) => page.map(|mut page| {
            page.next_cursor = None;
            BoxAnswerBody::Timeline {
                timeline: page_body(page, BOX_TIMELINE).unwrap_or_else(|error| {
                    fail(error);
                    TimelinePageBody::default()
                }),
            }
        }),
        BoxAnswer::Timer(timer) => timer.map(|timer| BoxAnswerBody::Timer {
            timer: TimerItem::from(&timer),
        }),
        BoxAnswer::Changes(changes) => changes.map(|changes| BoxAnswerBody::Changes {
            changes: changes_body(&changes),
        }),
    };
    if let Some(error) = failed {
        return Err(error);
    }
    answer_authorized(&body)
}

/// What one box of a dashboard answers for the person who looks at it, through the read of its
/// kind, run as them. A box that does not check for them is not found, as one that does not exist.
#[utoipa::path(
    get,
    path = "/api/v1/dashboards/{dashboard}/boxes/{box}",
    operation_id = "getDashboardBox",
    tag = "dashboards",
    params(
        ("dashboard" = String, Path, description = "The ID of the dashboard."),
        ("box" = String, Path, description = "The ID of the box.")
    ),
    responses(
        (status = 200, description = "What the box answers.", body = BoxAnswerBody),
        InvalidAnswer,
        NotSignedInAnswer,
        NotFoundAnswer
    )
)]
async fn get_dashboard_box(
    State(auth): State<AuthHandle>,
    Extension(limits): Extension<Arc<Limits>>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath((dashboard, dashboard_box)): ApiPath<(String, String)>,
) -> Result<Response, ApiError> {
    let read = dashboard_id(&dashboard).and_then(|dashboard| {
        let found = super::super::path_id::<DashboardBoxId>(&dashboard_box)?;
        Ok((dashboard, found))
    });
    let (dashboard, dashboard_box) = auth.after_who_asks(&info, &session_token, read).await?;
    counted(&auth, &limits, &info, &session_token).await?;
    let found = auth
        .api()?
        .dashboard_box(
            info,
            BoxQuery {
                session_token,
                dashboard,
                dashboard_box,
            },
        )
        .await?;
    box_answer(found)
}

/// What a box would answer before it is added (*Add box*): the same reads as the box itself.
#[utoipa::path(
    post,
    path = "/api/v1/dashboards/box-preview",
    operation_id = "previewDashboardBox",
    tag = "dashboards",
    request_body = BoxBody,
    responses(
        (status = 200, description = "What the box would answer.", body = BoxAnswerBody),
        InvalidAnswer,
        NotSignedInAnswer
    )
)]
async fn preview_dashboard_box(
    State(auth): State<AuthHandle>,
    Extension(limits): Extension<Arc<Limits>>,
    Info(info): Info,
    Session(session_token): Session,
    body: Result<ApiJson<BoxBody>, ApiError>,
) -> Result<Response, ApiError> {
    let read = body.and_then(|ApiJson(body)| body.read());
    let change = auth.after_who_asks(&info, &session_token, read).await?;
    if change.id.is_some() {
        return Err(ApiError::new(ErrorCode::InvalidRequest));
    }
    counted(&auth, &limits, &info, &session_token).await?;
    let found = auth
        .api()?
        .box_preview(
            info,
            BoxPreviewQuery {
                session_token,
                spec: change.spec,
            },
        )
        .await?;
    box_answer(found)
}

/// The ready-made boxes *Add box* offers, in order, each with the box it adds.
#[utoipa::path(
    get,
    path = "/api/v1/dashboards/ready-made-boxes",
    operation_id = "listReadyMadeBoxes",
    tag = "dashboards",
    responses(
        (status = 200, description = "The ready-made boxes.", body = ReadyMadeBody),
        NotSignedInAnswer
    )
)]
async fn list_ready_made_boxes(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
) -> Result<Response, ApiError> {
    auth.after_who_asks(&info, &session_token, Ok(())).await?;
    let list = auth
        .api()?
        .ready_made_boxes(info, ReadyMadeQuery { session_token })
        .await?;
    answer_authorized(&list.map(|list| {
        ReadyMadeBody {
            boxes: list
                .items
                .iter()
                .map(|entry| ReadyMadeItem {
                    which: entry.which,
                    ready_box: BoxItem::of(None, &entry.spec),
                })
                .collect(),
        }
    }))
}

/// The routes of the answers of boxes.
pub(super) fn routes<S>(limits: &Arc<Limits>) -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(get_dashboard_box))
        .routes(routes!(preview_dashboard_box))
        .routes(routes!(list_ready_made_boxes))
        .layer(Extension(Arc::clone(limits)))
}

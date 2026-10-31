//! The routes of the timer (TIME-01, ADR-0035 decision 3): the person's own timer below
//! `/api/v1/me/timer`, kept on the server so every device shows the same one.
//!
//! Every change takes the version of the timer it was made from (ARCH-VERSIONED); starting needs
//! *Book time* on the issue, and two devices that start at the same moment get one timer and one
//! refusal (`timer.running`). Stopping and switching save the time as an entry, with the note,
//! *Billable* and the values of the own fields of time entries the person filled in, and the
//! defaults of the issue for the rest.

use std::collections::BTreeMap;

use axum::extract::State;
use axum::response::Response;
use bb_app::time::{
    ChangeStartTime, EntryDetails, RunningView, StartTimer, StopTimer, SwitchTimer, ThrowAwayTimer,
    TimerQuery, TimerSaved, TimerView,
};
use bb_policy::{FieldMask, Masked, MaskedValue, mask_member};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::auth::{
    AuthHandle, ConflictAnswer, ForbiddenAnswer, Info, InvalidAnswer, NotFoundAnswer,
    NotSignedInAnswer, Session, answer_authorized,
};
use super::body_id;
use super::time_entries::TimeEntryItem;
use crate::errors::{ApiError, ApiJson};

/// What the person fills in for the entry that a stop or a switch saves; left out, the note is
/// empty, *Billable* is off and the own fields take the defaults of the issue.
#[derive(Default, Deserialize, ToSchema)]
pub(crate) struct EntryDetailsBody {
    /// The note, plain text, at most 2,000 characters.
    #[serde(default)]
    note: Option<String>,
    /// Whether the time can be charged to a customer; taken only while the organization offers
    /// it.
    #[serde(default)]
    billable: Option<bool>,
    /// The values of own fields of time entries, by the key (ID) of the field.
    #[serde(default)]
    fields: BTreeMap<String, Value>,
}

impl From<EntryDetailsBody> for EntryDetails {
    fn from(body: EntryDetailsBody) -> Self {
        Self {
            note: body.note,
            billable: body.billable,
            fields: body.fields,
        }
    }
}

/// A timer that runs.
#[derive(Serialize, ToSchema)]
pub(crate) struct RunningItem {
    /// The internal ID of the issue it runs on.
    issue: String,
    /// The readable ID of the issue (WEB-142) while the person sees it.
    #[serde(skip_serializing_if = "Option::is_none")]
    readable_id: Option<String>,
    /// When it started.
    started_at: String,
    /// Whether it runs longer than the longest work day of the working-time rules.
    runs_long: bool,
}

/// The person's timer.
#[derive(Serialize, ToSchema)]
pub(crate) struct TimerItem {
    /// What it runs on; left out while it stands still.
    #[serde(skip_serializing_if = "Option::is_none")]
    running: Option<RunningItem>,
    /// Counts up with every change. A change is made on the version that was looked at; 0 for a
    /// person who never started a timer.
    version: i64,
    /// The key (ID) of the own field each starter field of time entries is, by its mark
    /// (`activity`, `cost-center`), whatever it is called now.
    starters: BTreeMap<String, String>,
}

impl Masked for TimerItem {}

impl From<&TimerView> for TimerItem {
    fn from(view: &TimerView) -> Self {
        Self {
            running: view.running.as_ref().map(RunningItem::from),
            version: view.version,
            starters: view
                .starters
                .iter()
                .map(|(mark, field)| (mark.clone(), field.to_string()))
                .collect(),
        }
    }
}

impl From<&RunningView> for RunningItem {
    fn from(view: &RunningView) -> Self {
        Self {
            issue: view.issue.to_string(),
            readable_id: view.readable_id.clone(),
            started_at: view.started_at.to_string(),
            runs_long: view.runs_long,
        }
    }
}

/// What a stop or a switch did.
#[derive(Serialize, ToSchema)]
pub(crate) struct TimerSavedItem {
    /// The timer as it is now.
    timer: TimerItem,
    /// The entry that was saved; left out when the timer ran for less than a minute.
    #[serde(skip_serializing_if = "Option::is_none")]
    entry: Option<TimeEntryItem>,
}

impl Masked for TimerSavedItem {
    fn mask(value: &mut MaskedValue, mask: &FieldMask) {
        mask_member::<TimeEntryItem>(value, "entry", mask);
    }
}

impl From<&TimerSaved> for TimerSavedItem {
    fn from(saved: &TimerSaved) -> Self {
        Self {
            timer: TimerItem::from(&saved.timer),
            entry: saved.entry.as_ref().map(TimeEntryItem::from),
        }
    }
}

/// The issue to start the timer on.
#[derive(Deserialize, ToSchema)]
pub(crate) struct StartTimerBody {
    /// The readable ID (WEB-142) or the internal ID of the issue.
    issue: String,
}

/// A stop of the timer.
#[derive(Deserialize, ToSchema)]
pub(crate) struct StopTimerBody {
    /// The version of the timer that was looked at.
    version: i64,
    /// What the person filled in for the entry.
    #[serde(default)]
    entry: Option<EntryDetailsBody>,
}

/// A switch of the timer to another issue.
#[derive(Deserialize, ToSchema)]
pub(crate) struct SwitchTimerBody {
    /// The version of the timer that was looked at.
    version: i64,
    /// The readable ID or the internal ID of the issue to go on with.
    issue: String,
    /// What the person filled in for the entry of the time so far (when a required field has no
    /// default on its issue); left out for the defaults.
    #[serde(default)]
    entry: Option<EntryDetailsBody>,
}

/// A new start of the running timer.
#[derive(Deserialize, ToSchema)]
pub(crate) struct StartTimeBody {
    /// The version of the timer that was looked at.
    version: i64,
    /// The new start (RFC 3339): never in the future, never before the end of the last time entry
    /// that has a start.
    started_at: String,
}

/// A change of the timer that only needs its version.
#[derive(Deserialize, ToSchema)]
pub(crate) struct TimerVersionBody {
    /// The version of the timer that was looked at.
    version: i64,
}

/// The person's own timer: what it runs on and since when.
#[utoipa::path(
    get,
    path = "/api/v1/me/timer",
    operation_id = "getTimer",
    tag = "time",
    responses((status = 200, description = "The timer.", body = TimerItem), NotSignedInAnswer)
)]
async fn get_timer(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
) -> Result<Response, ApiError> {
    let shown = auth
        .api()?
        .timer(info, TimerQuery { session_token })
        .await?;
    answer_authorized(&shown.map(|timer| TimerItem::from(&timer)))
}

/// Starts the person's timer on an issue they see. Needs *Book time*; a timer that runs already,
/// also on another device, is a conflict.
#[utoipa::path(
    post,
    path = "/api/v1/me/timer",
    operation_id = "startTimer",
    tag = "time",
    request_body = StartTimerBody,
    responses(
        (status = 200, description = "The running timer.", body = TimerItem),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn start_timer(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<StartTimerBody>,
) -> Result<Response, ApiError> {
    let started = auth
        .api()?
        .start_timer(
            info,
            StartTimer {
                session_token,
                issue: body.issue,
            },
        )
        .await?;
    answer_authorized(&started.map(|timer| TimerItem::from(&timer)))
}

/// Stops the timer and saves the time as an entry, in whole minutes; under a minute nothing is
/// saved. A timer that ran longer than one entry can be is refused until its start time changes.
#[utoipa::path(
    post,
    path = "/api/v1/me/timer/stop",
    operation_id = "stopTimer",
    tag = "time",
    request_body = StopTimerBody,
    responses(
        (status = 200, description = "The timer and the entry that was saved.", body = TimerSavedItem),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn stop_timer(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<StopTimerBody>,
) -> Result<Response, ApiError> {
    let saved = auth
        .api()?
        .stop_timer(
            info,
            StopTimer {
                session_token,
                version: body.version,
                details: body.entry.unwrap_or_default().into(),
            },
        )
        .await?;
    answer_authorized(&saved.map(|saved| TimerSavedItem::from(&saved)))
}

/// Saves the time so far with the defaults of its issue and starts the timer on another issue.
/// A required field without a default on the issue is refused at that field, so the screen asks
/// for it and switches again with the value.
#[utoipa::path(
    post,
    path = "/api/v1/me/timer/switch",
    operation_id = "switchTimer",
    tag = "time",
    request_body = SwitchTimerBody,
    responses(
        (status = 200, description = "The timer on the new issue and the entry that was saved.", body = TimerSavedItem),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn switch_timer(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<SwitchTimerBody>,
) -> Result<Response, ApiError> {
    let saved = auth
        .api()?
        .switch_timer(
            info,
            SwitchTimer {
                session_token,
                version: body.version,
                issue: body.issue,
                details: body.entry.unwrap_or_default().into(),
            },
        )
        .await?;
    answer_authorized(&saved.map(|saved| TimerSavedItem::from(&saved)))
}

/// Moves the start of the running timer.
#[utoipa::path(
    post,
    path = "/api/v1/me/timer/start-time",
    operation_id = "changeTimerStart",
    tag = "time",
    request_body = StartTimeBody,
    responses(
        (status = 200, description = "The timer.", body = TimerItem),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        ConflictAnswer
    )
)]
async fn change_timer_start(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<StartTimeBody>,
) -> Result<Response, ApiError> {
    let moved = auth
        .api()?
        .change_start_time(
            info,
            ChangeStartTime {
                session_token,
                version: body.version,
                started_at: body_id(&body.started_at)?,
            },
        )
        .await?;
    answer_authorized(&moved.map(|timer| TimerItem::from(&timer)))
}

/// Stops the timer and saves nothing.
#[utoipa::path(
    post,
    path = "/api/v1/me/timer/throw-away",
    operation_id = "throwAwayTimer",
    tag = "time",
    request_body = TimerVersionBody,
    responses(
        (status = 200, description = "The timer, standing still.", body = TimerItem),
        NotSignedInAnswer,
        ForbiddenAnswer,
        ConflictAnswer
    )
)]
async fn throw_away_timer(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<TimerVersionBody>,
) -> Result<Response, ApiError> {
    let stopped = auth
        .api()?
        .throw_away_timer(
            info,
            ThrowAwayTimer {
                session_token,
                version: body.version,
            },
        )
        .await?;
    answer_authorized(&stopped.map(|timer| TimerItem::from(&timer)))
}

/// The routes of the timer.
pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(get_timer, start_timer))
        .routes(routes!(stop_timer))
        .routes(routes!(switch_timer))
        .routes(routes!(change_timer_start))
        .routes(routes!(throw_away_timer))
}

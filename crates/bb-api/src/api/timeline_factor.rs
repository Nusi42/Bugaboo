//! The time factor of an issue (PLAN-14, ADR-0030, decision 4 and amendment M8): read like any
//! field the reader may read, set by the person on the bar (always) or by whoever may change the
//! issue and the field, as a factor or as *Takes me …*. No search, sort, list column or total uses
//! it, and no tool of the registry sets it.

use axum::extract::State;
use axum::response::Response;
use bb_app::plan::{FactorGiven, SetTimeFactor, TimeFactorQuery, TimeFactorView};
use bb_core::time::Minutes;
use bb_policy::{Masked, PersonRef};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::auth::{
    AuthHandle, ConflictAnswer, ForbiddenAnswer, Info, InvalidAnswer, NotFoundAnswer,
    NotSignedInAnswer, Session, answer_authorized,
};
use crate::errors::{ApiError, ApiJson, ApiPath};

/// The time factor of an issue. The factor and who set it are the field *Time factor*: a reader
/// whose field level hides it gets the issue, its version and the person on the bar only.
#[derive(Serialize, ToSchema, Masked)]
pub(crate) struct TimeFactorBody {
    /// The internal ID of the issue.
    #[masked(open)]
    issue: String,
    /// The version of the issue now: a change is made on the version that was looked at.
    #[masked(open)]
    version: i64,
    /// The factor in percent of the estimate (125 is ×1.25); 100 when nobody set one.
    #[masked(field = "time-factor")]
    percent: u16,
    /// Who set it (*set by Ada Brandt*); `null` for ×1.
    #[masked(field = "time-factor")]
    #[schema(required = true)]
    set_by: Option<PersonRef>,
    /// Whether the person on the bar set it themselves (*Lea's own estimate*); `false` for a reader
    /// who is not shown the person on the bar by name.
    #[masked(field = "time-factor")]
    own_estimate: bool,
    /// The factor again, for the person on the bar alone, who always sees their own estimate, also
    /// where the field is hidden from their role; `null` for everybody else.
    #[masked(open)]
    #[schema(required = true)]
    own_percent: Option<u16>,
    /// The person on the bar (the first assigned person); `null` for an issue assigned to nobody.
    #[masked(field = "assigned-to")]
    #[schema(required = true)]
    person: Option<PersonRef>,
    /// Whether the reader may set the time factor: the person on the bar with a key that may change
    /// things, or somebody who may change the issue and the field; never an agent.
    #[masked(open)]
    may_change: bool,
}

impl From<TimeFactorView> for TimeFactorBody {
    fn from(view: TimeFactorView) -> Self {
        Self {
            issue: view.issue.to_string(),
            version: view.version,
            percent: view.percent,
            set_by: view.set_by,
            own_estimate: view.own_estimate,
            own_percent: view.own_percent,
            person: view.person,
            may_change: view.may_change,
        }
    }
}

/// A new time factor: as a factor (`percent`) or as a length (`takes_minutes`, *Takes me …*,
/// worked out against *Estimated time*). Exactly one of the two.
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct SetTimeFactorBody {
    /// The version of the issue the person looked at.
    version: i64,
    /// The factor in percent of the estimate, from 10 to 1000; 100 takes it back to ×1.
    #[serde(default)]
    percent: Option<u16>,
    /// *Takes me …*: how long the issue takes the person, in minutes.
    #[serde(default)]
    takes_minutes: Option<u32>,
}

/// The time factor of an issue the person may see.
#[utoipa::path(
    get,
    path = "/api/v1/issues/{issue}/time-factor",
    operation_id = "getTimeFactor",
    tag = "timeline",
    params(("issue" = String, Path, description = "The readable ID (WEB-142) or the internal ID of the issue.")),
    responses(
        (status = 200, description = "The time factor.", body = TimeFactorBody),
        NotSignedInAnswer,
        NotFoundAnswer
    )
)]
async fn get_time_factor(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(issue): ApiPath<String>,
) -> Result<Response, ApiError> {
    let found = auth
        .api()?
        .time_factor(
            info,
            TimeFactorQuery {
                session_token,
                issue,
            },
        )
        .await?;
    answer_authorized(&found.map(TimeFactorBody::from))
}

/// Sets the time factor of an issue: the person on the bar always may, anybody else needs to be
/// allowed to change the issue and the field *Time factor*. The change is a change of the issue
/// (its version counts up, the history shows it), and the person on the bar is told when somebody
/// else made it. It goes back to ×1 by itself when the first assigned person changes.
#[utoipa::path(
    put,
    path = "/api/v1/issues/{issue}/time-factor",
    operation_id = "setTimeFactor",
    tag = "timeline",
    params(("issue" = String, Path, description = "The readable ID (WEB-142) or the internal ID of the issue.")),
    request_body = SetTimeFactorBody,
    responses(
        (status = 200, description = "The time factor now.", body = TimeFactorBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer,
        ConflictAnswer
    )
)]
async fn set_time_factor(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(issue): ApiPath<String>,
    ApiJson(body): ApiJson<SetTimeFactorBody>,
) -> Result<Response, ApiError> {
    let given = match (body.percent, body.takes_minutes) {
        (Some(percent), None) => FactorGiven::Percent(percent),
        (None, Some(minutes)) => FactorGiven::Takes(Minutes::new(minutes)),
        _ => return Err(ApiError::new(bb_core::ErrorCode::InvalidRequest)),
    };
    let set = auth
        .api()?
        .set_time_factor(
            info,
            SetTimeFactor {
                session_token,
                issue,
                version: body.version,
                given,
            },
        )
        .await?;
    answer_authorized(&set.map(TimeFactorBody::from))
}

/// The routes of the time factor.
pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new().routes(routes!(get_time_factor, set_time_factor))
}

//! The routes of *Keep and delete rules* (PRIV-05, ADR-0045), below `/api/v1/admin/keep-rules`:
//! the rules with the periods in force, and for a kind that an administrator deletes the count of
//! what is past its period and *Delete them*, for everybody or for one person. All need *Change
//! organization settings*, which no access key holds; the periods change through the settings
//! routes that exist.

use axum::extract::State;
use axum::response::Response;
use bb_app::keeping::{
    DeleteOldRecords, DeletedRecords, KeepRules, KeepRulesQuery, OldOfPerson, OldRecords,
    OldRecordsQuery,
};
use bb_core::ErrorCode;
use bb_core::id::UserId;
use bb_core::keeping::{KeepGroup, KeepKind, LengthUnit};
use bb_policy::{Masked, PersonRef};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::auth::{
    AuthHandle, ForbiddenAnswer, Info, InvalidAnswer, NotFoundAnswer, NotSignedInAnswer, Session,
    answer_authorized,
};
use super::body_id;
use crate::errors::{ApiError, ApiPath, ApiQuery};

/// How long a kind of data is kept.
#[derive(Serialize, ToSchema)]
pub(crate) struct LengthBody {
    /// How many.
    count: i64,
    /// Of what.
    unit: LengthUnit,
}

/// One row of the rules.
#[derive(Serialize, ToSchema)]
pub(crate) struct KeepRuleBody {
    /// The kind of data.
    kind: KeepKind,
    /// Who deletes it after its period.
    group: KeepGroup,
    /// The period in force.
    period: LengthBody,
    /// The key of the setting that changes the period; left out for a fixed period.
    #[serde(skip_serializing_if = "Option::is_none")]
    setting: Option<&'static str>,
    /// The shortest period the setting allows; left out for a fixed period.
    #[serde(skip_serializing_if = "Option::is_none")]
    floor: Option<i64>,
}

/// The rules, in the order of the page.
#[derive(Serialize, ToSchema)]
pub(crate) struct KeepRulesBody {
    /// The rows.
    rules: Vec<KeepRuleBody>,
}

impl Masked for KeepRulesBody {}

impl From<&KeepRules> for KeepRulesBody {
    fn from(rules: &KeepRules) -> Self {
        Self {
            rules: rules
                .rules
                .iter()
                .map(|row| KeepRuleBody {
                    kind: row.kind,
                    group: row.group,
                    period: LengthBody {
                        count: row.length.count,
                        unit: row.length.unit,
                    },
                    setting: row.setting,
                    floor: row.floor,
                })
                .collect(),
        }
    }
}

/// The records of one person that are past the keep period.
#[derive(Serialize, ToSchema)]
pub(crate) struct OldOfPersonBody {
    /// The person, as the reader sees people in the settings of the organization.
    person: PersonRef,
    /// The ID of the person, for *Delete them* of one person; left out when the reader does not
    /// see the person by name.
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    /// How many of their records.
    count: u64,
}

/// The bodies of the people of a count of old records.
pub(crate) fn people_bodies(people: &[OldOfPerson]) -> Vec<OldOfPersonBody> {
    people
        .iter()
        .map(|person| OldOfPersonBody {
            person: person.person.clone(),
            id: person.id.map(|id| id.to_string()),
            count: person.count,
        })
        .collect()
}

/// The records of a kind past the keep period.
#[derive(Serialize, ToSchema)]
pub(crate) struct OldRecordsBody {
    /// The kind of records.
    kind: KeepKind,
    /// The keep period in force.
    period: LengthBody,
    /// The records with a date before this one are past it.
    before: String,
    /// How many there are.
    total: u64,
    /// How many of each person, in the order of their names, at most `OLD_PEOPLE_MAX`.
    people: Vec<OldOfPersonBody>,
    /// Whether more people have old records than the list names.
    truncated: bool,
}

impl Masked for OldRecordsBody {}

impl From<&OldRecords> for OldRecordsBody {
    fn from(old: &OldRecords) -> Self {
        Self {
            kind: old.kind,
            period: LengthBody {
                count: old.length.count,
                unit: old.length.unit,
            },
            before: old.before.to_string(),
            total: old.total,
            people: people_bodies(&old.people),
            truncated: old.truncated,
        }
    }
}

/// How many records were deleted.
#[derive(Serialize, ToSchema)]
pub(crate) struct DeletedBody {
    /// The count.
    deleted: u64,
}

impl Masked for DeletedBody {}

impl From<&DeletedRecords> for DeletedBody {
    fn from(deleted: &DeletedRecords) -> Self {
        Self {
            deleted: deleted.deleted,
        }
    }
}

/// Whose old records to delete.
#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct WhoseParams {
    /// One person, by ID; everybody's when left out.
    person: Option<String>,
}

/// The kind a path names; an unknown word is not found.
fn kind_of(word: &str) -> Result<KeepKind, ApiError> {
    KeepKind::from_code(word).ok_or_else(|| ApiError::new(ErrorCode::NotFound))
}

/// The person `whose` names, if any.
pub(crate) fn whose_person(
    whose: Result<ApiQuery<WhoseParams>, ApiError>,
) -> Result<Option<UserId>, ApiError> {
    whose.and_then(|ApiQuery(whose)| whose.person.as_deref().map(body_id::<UserId>).transpose())
}

/// Every rule of keeping and deleting, with the period in force.
#[utoipa::path(
    get,
    path = "/api/v1/admin/keep-rules",
    operation_id = "listKeepRules",
    tag = "privacy",
    responses(
        (status = 200, description = "The rules.", body = KeepRulesBody),
        NotSignedInAnswer,
        ForbiddenAnswer
    )
)]
async fn list_keep_rules(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
) -> Result<Response, ApiError> {
    let rules = auth
        .api()?
        .keep_rules(info, KeepRulesQuery { session_token })
        .await?;
    answer_authorized(&rules.map(|rules| KeepRulesBody::from(&rules)))
}

/// How many records of a kind an administrator deletes are past the keep period, for each person
/// and in total, counted now.
#[utoipa::path(
    get,
    path = "/api/v1/admin/keep-rules/{kind}/past",
    operation_id = "countPastKeepPeriod",
    tag = "privacy",
    params(("kind" = String, Path, description = "The kind of records, like time-entries.")),
    responses(
        (status = 200, description = "The count.", body = OldRecordsBody),
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer
    )
)]
async fn count_past_keep_period(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(kind): ApiPath<String>,
) -> Result<Response, ApiError> {
    let kind = auth
        .after_who_asks(&info, &session_token, kind_of(&kind))
        .await?;
    let old = auth
        .api()?
        .old_records(
            info,
            OldRecordsQuery {
                session_token,
                kind,
            },
        )
        .await?;
    answer_authorized(&old.map(|old| OldRecordsBody::from(&old)))
}

/// *Delete them*: deletes the records of a kind an administrator deletes that are past the keep
/// period, of everybody or of one person, with everything that tells about them. Every deletion is
/// in the security log.
#[utoipa::path(
    post,
    path = "/api/v1/admin/keep-rules/{kind}/delete-past",
    operation_id = "deletePastKeepPeriod",
    tag = "privacy",
    params(
        ("kind" = String, Path, description = "The kind of records, like time-entries."),
        WhoseParams
    ),
    responses(
        (status = 200, description = "How many were deleted.", body = DeletedBody),
        InvalidAnswer,
        NotSignedInAnswer,
        ForbiddenAnswer,
        NotFoundAnswer
    )
)]
async fn delete_past_keep_period(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(kind): ApiPath<String>,
    whose: Result<ApiQuery<WhoseParams>, ApiError>,
) -> Result<Response, ApiError> {
    let read = kind_of(&kind).and_then(|kind| whose_person(whose).map(|person| (kind, person)));
    let (kind, person) = auth.after_who_asks(&info, &session_token, read).await?;
    let deleted = auth
        .api()?
        .delete_old_records(
            info,
            DeleteOldRecords {
                session_token,
                kind,
                person,
            },
        )
        .await?;
    answer_authorized(&deleted.map(|deleted| DeletedBody::from(&deleted)))
}

/// The routes of the keep rules.
pub(super) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(list_keep_rules))
        .routes(routes!(count_past_keep_period))
        .routes(routes!(delete_past_keep_period))
}

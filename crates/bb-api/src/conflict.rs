//! The structured answer to a stale version (`edit.version_changed`, ADR-0019 amendment M3) as the
//! API writes it: the usual problem answer plus a `conflict` member with who changed the record,
//! when, which fields, and the values now.
//!
//! It is made in one place for every route: a use case answers that the version is stale, `AuthApi`
//! turns that into a [`bb_app::editing::Conflict`], and `From<AppError>` for [`crate::ApiError`]
//! puts it here. The API description documents it for every route that takes a `version`
//! (`crate::openapi`), so the generated client types it for all of them.

use std::collections::BTreeMap;

use bb_app::editing::Conflict;
use bb_core::Problem;
use bb_policy::PersonRef;
use serde::Serialize;
use utoipa::ToSchema;

/// What changed a record since the person looked at it.
#[derive(Clone, Debug, Serialize, ToSchema)]
pub(crate) struct ConflictBody {
    /// The version the record has now: what *Use mine* sends again.
    version: Option<i64>,
    /// Who changed it last, as the reader may see them.
    changed_by: Option<PersonRef>,
    /// When, as a moment in time.
    changed_at: Option<String>,
    /// The fields that changed, without the ones the reader may not see.
    fields: Vec<String>,
    /// The values those fields have now.
    #[schema(value_type = Object)]
    current: BTreeMap<String, serde_json::Value>,
}

impl From<&Conflict> for ConflictBody {
    fn from(conflict: &Conflict) -> Self {
        Self {
            version: conflict.version,
            changed_by: conflict.changed_by.clone(),
            changed_at: conflict.changed_at.map(|moment| moment.to_string()),
            fields: conflict.fields.clone(),
            current: conflict.current.clone(),
        }
    }
}

/// The answer to a change that found the record at another version, or to any other refused
/// change: the usual problem, and for `edit.version_changed` the `conflict` member.
#[derive(Clone, Debug, Serialize, ToSchema)]
pub(crate) struct ConflictProblem {
    #[serde(flatten)]
    problem: Problem,
    /// What changed the record; only for `edit.version_changed`.
    #[serde(skip_serializing_if = "Option::is_none")]
    conflict: Option<ConflictBody>,
}

impl ConflictProblem {
    pub(crate) const fn new(problem: Problem, conflict: Option<ConflictBody>) -> Self {
        Self { problem, conflict }
    }
}

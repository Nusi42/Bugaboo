//! The bodies and parameters that many routes share, written once (ARCH-API-LINT: one schema for
//! one shape).

use serde::Deserialize;
use utoipa::ToSchema;

/// A change that needs nothing but the version of the record it is made on: giving a role its
/// starting permissions back, throwing the timer away. One schema for all of them; its name is the
/// one the API was frozen with (ADR-0008).
#[derive(Deserialize, ToSchema)]
#[schema(as = ResetRoleBody)]
pub(crate) struct VersionBody {
    /// The version of the record the person looked at; a change made meanwhile stops this one.
    pub(crate) version: i64,
}

/// The version of the record a removal may be made on, in the query, where the removal also works
/// without it: given, a change made meanwhile stops the removal (409), as for time entries.
#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct OptionalVersionParams {
    /// The version of the record the person looked at; a change made meanwhile stops this one.
    /// Without it, the record is removed whatever its version.
    pub(crate) version: Option<i64>,
}

/// The version of the record a removal is made on, in the query: a `DELETE` has no body.
#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct VersionParams {
    /// The version of the record the person looked at; a change made meanwhile stops this one.
    pub(crate) version: i64,
}

//! What the routes that read a stretch of dates have in common: the query `from` and `to`, the
//! answer to a date that is none, and the reading of a person's address with the stretch. The
//! routes of work days and of work times use them.

use bb_app::working_hours::{DateRange, read_range};
use bb_core::ErrorCode;
use bb_core::id::UserId;
use serde::Deserialize;

use crate::errors::{ApiError, ApiQuery};

/// The answer to a date or a moment that is none.
pub(crate) fn invalid() -> ApiError {
    ApiError::new(ErrorCode::InvalidRequest)
}

/// The stretch of dates to read.
#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct RangeParams {
    /// The first date (`2026-10-05`).
    from: String,
    /// The last date, on or after the first; a stretch longer than the route takes is refused.
    to: String,
}

impl RangeParams {
    /// The stretch it says; a date that is none is not a valid request.
    pub(crate) fn read(&self) -> Result<DateRange, ApiError> {
        read_range(&self.from, &self.to).ok_or_else(invalid)
    }
}

/// The person in the address of a route and the stretch of dates it asks about: the one reading of
/// both, for the routes that name a person.
pub(crate) fn person_and_range(
    person: &str,
    range: Result<ApiQuery<RangeParams>, ApiError>,
) -> Result<(UserId, DateRange), ApiError> {
    let person = super::path_id::<UserId>(person)?;
    let ApiQuery(range) = range?;
    Ok((person, range.read()?))
}

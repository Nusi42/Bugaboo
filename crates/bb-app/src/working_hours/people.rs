//! The people whose working hours a reader may open, to choose one from (*Working hours of
//! others*, TIME-12, ADR-0035, decision 8): only for a holder of *See working hours of others*, and
//! by name, because that permission shows names on that page whatever *Who sees people* says.
//! Nothing but a name and an ID is offered: no hours, no counts, no order by any number.

use bb_policy::{Authorized, PeopleContext, Permission, Place};

use super::types::PeopleOfHoursQuery;
use crate::auth::{AppError, AuthStore, Ctx};
use crate::people::ChoicePage;

/// The people to choose from, by name, a page at a time.
///
/// # Errors
/// `access.forbidden` without *See working hours of others*, `request.invalid` for a cursor that
/// is not one.
pub async fn people_of_hours<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: PeopleOfHoursQuery,
) -> Result<Authorized<ChoicePage>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let access = ctx
        .require(
            &actor,
            Permission::SeeWorkingHoursOfOthers,
            Place::Organization,
        )
        .await?;
    let page = ctx
        .people_choices(
            &access,
            PeopleContext::WorkingHours,
            input.search.as_deref(),
            input.cursor.as_deref(),
            input.limit,
            |_| None,
        )
        .await?;
    Ok(access.authorize(page))
}

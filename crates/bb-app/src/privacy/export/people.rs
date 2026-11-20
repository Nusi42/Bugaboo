//! The people *Download someone's data* offers to choose from (PRIV-03, ADR-0044, decision 4): the
//! permission brings its own list, so a holder needs no *Manage people* to pick anybody. They are
//! shown as on any management screen: by name to a manager of people, and to everybody else as
//! *Who sees people* says for *Project members*; a person the viewer may not name is not offered.
//! A switched-off person is offered too, because their data may be asked for as well.

use bb_policy::{Authorized, PeopleContext, Permission, Place};

use super::types::ExportPeopleQuery;
use crate::auth::{AppError, AuthStore, Ctx, PeopleOf};
use crate::people::ChoicePage;

/// The people whose data the viewer may ask for, by name, a page at a time.
///
/// # Errors
/// `access.forbidden` without *Download someone's data*, `request.invalid` for a cursor that is
/// not one.
pub async fn export_people<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ExportPeopleQuery,
) -> Result<Authorized<ChoicePage>, AppError> {
    let actor = ctx.session_actor(&input.session_token).await?;
    let held = ctx
        .require(
            &actor,
            Permission::DownloadSomeonesData,
            Place::Organization,
        )
        .await?;
    let page = ctx
        .people_choices(
            &held,
            (PeopleContext::Management, PeopleOf::NotRemoved),
            input.search.as_deref(),
            input.cursor.as_deref(),
            input.limit,
            |_| None,
        )
        .await?;
    Ok(held.authorize(page))
}

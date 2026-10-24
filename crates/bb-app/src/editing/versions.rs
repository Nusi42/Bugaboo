//! Checking the versions of what is on the screen (ADR-0019): until live updates exist (ISS-14), the
//! browser asks every 30 seconds and when it returns to a tab for the current versions of the
//! records it shows, in one small request. It compares them with what it has and, where somebody
//! else changed a record, offers *Show changes*.

use bb_core::ErrorCode;
use bb_core::edit::VERSIONS_MAX;
use bb_policy::{Authorized, Place};

use super::types::{CheckVersionsQuery, RecordVersion, Versions};
use crate::auth::{AppError, AuthStore, Ctx};

/// The versions of the records asked about, in the order asked. A record that is gone, or that the
/// person may not know about, has no version: the two look the same.
///
/// # Errors
/// `request.too-large` for more than `bb_core::edit::VERSIONS_MAX` records.
pub async fn check_versions<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: CheckVersionsQuery,
) -> Result<Authorized<Versions>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    if input.records.len() > usize::try_from(VERSIONS_MAX).unwrap_or(usize::MAX) {
        return Err(ErrorCode::TooLarge.into());
    }
    let mut items = Vec::with_capacity(input.records.len());
    for record in input.records {
        let version = match ctx.facts_of(&actor, &record).await {
            Ok(facts) => Some(facts.version),
            Err(AppError::Refused(ErrorCode::NotFound)) => None,
            Err(error) => return Err(error),
        };
        items.push(RecordVersion { record, version });
    }
    let access = ctx.evaluation_of(&actor, Place::Organization).await?;
    Ok(access.authorize(Versions { items }))
}

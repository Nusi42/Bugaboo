//! The tags to offer while a person types (TAG-01, M3-Q8).
//!
//! The list holds only tags that are used on issues in projects the person sees, so a tag that is
//! used only in a project they cannot see is never offered, and it is not a way to find out that the
//! project exists. A tag that is limited to other projects than the one the issue is in is left out,
//! because it would be refused.

use bb_core::ErrorCode;
use bb_core::id::ProjectId;
use bb_core::tags::{TAG_SUGGESTIONS_MAX, tag_allowed_in};
use bb_policy::{Authorized, Place};

use super::types::{TagRef, TagSuggestions, TagSuggestionsQuery};
use crate::auth::{AppError, AuthStore, Ctx, internal};

/// How many candidates are read before the ones that do not fit the project are left out: a few
/// more than the list holds, so leaving some out rarely leaves it short.
const CANDIDATES: u64 = 10 * TAG_SUGGESTIONS_MAX as u64;

/// The tags used in the projects the person sees whose name holds what they typed, that may go on
/// an issue in the project.
///
/// # Errors
/// `resource.not_found` for a project that does not exist or that the person may not see.
pub async fn tag_suggestions<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: TagSuggestionsQuery,
) -> Result<Authorized<TagSuggestions>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let shown = ctx.shown_to(&actor).await?;
    let found = ctx.find_project(&input.project).await?;
    let place = found.as_ref().map_or(Place::Organization, |found| {
        Place::Project(found.project.id)
    });
    let held = ctx.evaluation_of(&actor, place).await?;
    let Some(found) = found.filter(|_| held.sees_project()) else {
        return Err(AppError::Refused(ErrorCode::NotFound));
    };
    let chain = ctx.chain_of(found.project.id).await?;
    let seen: Vec<ProjectId> = shown
        .everything()
        .iter()
        .map(|project| project.id)
        .collect();
    let candidates = ctx
        .store()
        .used_tags(&seen, &input.text, CANDIDATES)
        .await
        .map_err(|error| internal::<S>(&error))?;
    let limits = ctx
        .store()
        .tag_projects()
        .await
        .map_err(|error| internal::<S>(&error))?;
    Ok(held.authorize(TagSuggestions {
        tags: candidates
            .iter()
            .filter(|tag| tag_allowed_in(tag.id, &limits, &chain))
            .take(TAG_SUGGESTIONS_MAX)
            .map(TagRef::from)
            .collect(),
    }))
}

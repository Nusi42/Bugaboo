//! The names a suggestion in the search bar offers for a person or a group (UX design section 4.3):
//! the names this person may use in a search, by the same rule that decides whether a search that
//! names them is accepted (`Ctx::search_names`, ADR-0005 amendment M5, decision 9). A person they
//! may not name is not in the list, exactly as one who does not exist.

use bb_policy::{Authorized, Masked, Place as Where};
use serde::Serialize;

use crate::auth::{AppError, AuthStore, Ctx};

/// The most names an answer holds.
pub const NAME_CHOICES_MAX: usize = 20;

/// Which names are asked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameChoiceKind {
    /// People, for the fields that hold a person.
    People,
    /// Groups, for `member of`.
    Groups,
}

/// Ask for the names that start with some words.
pub struct NameChoicesQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// People or groups.
    pub kind: NameChoiceKind,
    /// The start of the names; empty for the first ones.
    pub starting_with: String,
    /// How many names at most, up to one more than [`NAME_CHOICES_MAX`].
    pub limit: usize,
}

/// The names, in the order of the names.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct NameChoices(pub Vec<String>);

impl Masked for NameChoices {}

/// The names of people or groups that the person may use in a search and that start with the words.
///
/// # Errors
/// An internal error when the store fails.
pub async fn name_choices<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    query: NameChoicesQuery,
) -> Result<Authorized<NameChoices>, AppError> {
    let actor = ctx.actor(&query.session_token).await?;
    // One more than the most an answer holds, so the caller can tell that the list was cut off.
    let limit = query.limit.clamp(1, NAME_CHOICES_MAX + 1);
    let names = match query.kind {
        NameChoiceKind::People => {
            ctx.people_name_choices(&actor, &query.starting_with, limit)
                .await?
        }
        NameChoiceKind::Groups => {
            ctx.group_name_choices(&actor, &query.starting_with, limit)
                .await?
        }
    };
    let held = ctx.evaluation_of(&actor, Where::Organization).await?;
    Ok(held.authorize(NameChoices(names)))
}

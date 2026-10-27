//! The names a suggestion offers (people and groups): they are chosen by the same rules as the names
//! a search resolves, so this part of the module reads the same helpers.

use bb_core::people_kinds::PeopleKind;
use bb_policy::Place as Where;

use bb_core::names::same_name;

use super::{Matching, NameCache, names_other_people};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx};

/// The kinds of people a name offered for a suggestion has to be nameable for: a suggestion does
/// not know the field it stands in, so it offers only names that every person field takes.
const OFFERED_KINDS: [PeopleKind; 2] = [PeopleKind::AssignedTo, PeopleKind::ReportedBy];

impl<S: AuthStore> Ctx<'_, S> {
    /// The names of people that this person may use in a search and that start with the words
    /// typed, at most `limit`, in the order of the names.
    ///
    /// # Errors
    /// An internal error when the store fails.
    pub(crate) async fn people_name_choices(
        &self,
        actor: &Actor,
        starting_with: &str,
        limit: usize,
    ) -> Result<(Vec<String>, bool), AppError> {
        let viewer = self.evaluation_of(actor, Where::Organization).await?;
        // The names every person field takes: one read answers for all the kinds when they agree;
        // when one of them names only the person themselves, that one decides (it is the narrowest).
        let kind = OFFERED_KINDS
            .into_iter()
            .find(|kind| !names_other_people(&viewer, *kind))
            .unwrap_or(OFFERED_KINDS[0]);
        let matched = self
            .people_matching(actor, &viewer, kind, starting_with, Matching::Start, limit)
            .await?;
        let mut names: Vec<String> = Vec::new();
        for person in matched.people {
            // Two people of one name are one name in a search: it is offered once.
            if !names
                .iter()
                .any(|name| same_name(name, &person.display_name))
            {
                names.push(person.display_name);
            }
        }
        let cut = matched.cut || names.len() > limit;
        names.truncate(limit);
        Ok((names, cut))
    }

    /// The names of groups that this person may name after `member of` and that start with the
    /// words typed, at most `limit`.
    ///
    /// # Errors
    /// An internal error when the store fails.
    pub(crate) async fn group_name_choices(
        &self,
        actor: &Actor,
        starting_with: &str,
        limit: usize,
    ) -> Result<Vec<String>, AppError> {
        let viewer = self.evaluation_of(actor, Where::Organization).await?;
        let mut cache = NameCache::default();
        // All the groups are read once, so the list is never cut by a bound: the caller asks for
        // one more than it shows.
        Ok(self
            .groups_matching(
                actor,
                &viewer,
                starting_with,
                true,
                Matching::Start,
                &mut cache,
            )
            .await?
            .into_iter()
            .map(|group| group.name)
            .take(limit)
            .collect())
    }
}

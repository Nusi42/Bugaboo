//! The people *Check access* can choose from (SEC-03, M2-Q3).
//!
//! Only a checker who sees names here can choose anybody: a person shown as a role or as
//! *Someone* has no ID to be chosen by, so for such a checker the list is empty and says nothing
//! about who is there.

use bb_core::access::PeopleLevel;
use bb_core::id::{ProjectId, UserId};
use bb_core::people_kinds::PeopleKind;
use bb_core::person::Person;
use bb_policy::{Authorized, Masked, PeopleContext, Permission, PersonRef};
use serde::Serialize;

use super::Place;
use crate::auth::{AppError, AuthStore, Ctx, PeoplePosition, internal};
use crate::people::{read_cursor, write_cursor};
use crate::roles::place_of;

/// List the people to choose from for *Check access*.
pub struct PeopleToCheckQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The project the person is checked in, or `None` for the whole organization.
    pub place: Option<ProjectId>,
    /// Words that every found name holds; the text may be empty, which finds everybody.
    pub search: Option<String>,
    /// Where the page starts: the `next_cursor` of the page before.
    pub cursor: Option<String>,
    /// How many people a page holds at most.
    pub limit: u32,
}

/// One page of people, by name.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PeopleToCheckPage {
    /// The people, each shown as the person who asks may see them: by name.
    pub items: Vec<PersonRef>,
    /// The cursor of the next page, if there is one.
    pub next_cursor: Option<String>,
}

impl Masked for PeopleToCheckPage {}

impl<S: AuthStore> Ctx<'_, S> {
    /// Whether a person has a role that reaches the place (and so is a member there).
    pub(crate) async fn has_a_role_at(
        &self,
        person: UserId,
        place: Place,
    ) -> Result<bool, AppError> {
        Ok(!self.evaluate(person, place).await?.roles().is_empty())
    }
}

/// The people the checker can choose from, a page at a time, by name. A checker who manages
/// people or roles there chooses among everybody; the others among the people who have a role
/// that reaches the place.
///
/// # Errors
/// `access.forbidden` without *Check access* there (`resource.not_found` for a project the
/// person may not know about), `request.invalid` for a cursor that is not one.
pub async fn people_to_check<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: PeopleToCheckQuery,
) -> Result<Authorized<PeopleToCheckPage>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let place: Place = place_of(input.place);
    let checker = ctx.require(&actor, Permission::CheckAccess, place).await?;
    let mut after = input.cursor.as_deref().map(read_cursor).transpose()?;
    let names_shown = checker.people_level(PeopleKind::ProjectMembers, PeopleContext::Management)
        == PeopleLevel::Names;
    if !names_shown {
        return Ok(checker.authorize(PeopleToCheckPage {
            items: Vec::new(),
            next_cursor: None,
        }));
    }
    let limit = u64::from(bb_core::paging::page_size(Some(input.limit)));
    let wanted = usize::try_from(limit).unwrap_or(usize::MAX);
    let manages = checker.manages_people();
    let mut page: Vec<Person> = Vec::with_capacity(wanted.saturating_add(1));
    // The store gives the people by name, a batch at a time; whoever does not manage people or
    // roles keeps only the people who have a role that reaches the place, as *Project members* are
    // shown (M2-Q3), so more batches are read until a page (and one more, to know whether there is
    // a next one) is full.
    'batches: loop {
        let batch = ctx
            .store()
            .people_page(input.search.as_deref(), false, after.as_ref(), limit + 1)
            .await
            .map_err(|error| internal::<S>(&error))?;
        let full = u64::try_from(batch.len()).is_ok_and(|count| count > limit);
        for person in &batch {
            if manages || ctx.has_a_role_at(person.id, place).await? {
                page.push(person.clone());
                if page.len() > wanted {
                    break 'batches;
                }
            }
        }
        match batch.last() {
            Some(last) if full => after = Some(position_of(last)),
            _ => break,
        }
    }
    let more = page.len() > wanted;
    page.truncate(wanted);
    let next_cursor = more.then(|| page.last().map(write_cursor)).flatten();
    let mut items = Vec::with_capacity(page.len());
    for person in &page {
        items.push(
            ctx.person_ref(
                &checker,
                PeopleKind::ProjectMembers,
                PeopleContext::Management,
                person.id,
            )
            .await?,
        );
    }
    Ok(checker.authorize(PeopleToCheckPage { items, next_cursor }))
}

/// Where a page of people ends, for the store.
fn position_of(person: &Person) -> PeoplePosition {
    PeoplePosition {
        name: person.display_name.clone(),
        id: person.id,
    }
}

//! The people *Check access* can choose from (SEC-03, M2-Q3).
//!
//! Only a checker who sees names here can choose anybody: a person shown as a role or as
//! *Someone* has no ID to be chosen by, so for such a checker the list is empty and says nothing
//! about who is there.

use bb_core::access::PeopleLevel;
use bb_core::id::{ProjectId, UserId};
use bb_core::paging::{Cut, page_len, read_len};
use bb_core::people_kinds::PeopleKind;
use bb_core::person::Person;
use bb_policy::{Authorized, Masked, PeopleContext, Permission, PersonRef};
use serde::Serialize;

use super::Place;
use crate::auth::{AppError, AuthStore, Ctx, PeoplePosition, SearchIn, if_there, internal};
use crate::people::write_cursor;

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

/// Which people a list of the people of a place holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Members {
    /// Everybody, for a person who manages people or roles.
    Everybody,
    /// The people who have a role that reaches the place.
    WithRole,
    /// The people an issue that this person reported could be assigned to: see
    /// [`Ctx::may_be_assigned`].
    Assignable { reported_by: UserId },
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The people by name, from `after`, who are members of the place as `members` says. A page of `wanted` people and one more, to
    /// know whether there is a next page. The store gives the people a batch at a time, so more
    /// batches are read until the page is full.
    pub(crate) async fn members_page(
        &self,
        place: Place,
        search: Option<&str>,
        mut after: Option<PeoplePosition>,
        wanted: usize,
        members: Members,
    ) -> Result<Vec<Person>, AppError> {
        let mut page: Vec<Person> = Vec::with_capacity(wanted.saturating_add(1));
        'batches: loop {
            let batch = self
                .store()
                .people_page(search, SearchIn::Names, after.as_ref(), read_len(wanted))
                .await
                .map_err(|error| internal::<S>(&error))?;
            let full = batch.len() > wanted;
            for person in &batch {
                if self.is_listed(person.id, place, members).await? {
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
        Ok(page)
    }

    async fn is_listed(
        &self,
        person: UserId,
        place: Place,
        members: Members,
    ) -> Result<bool, AppError> {
        match members {
            Members::Everybody => Ok(true),
            Members::WithRole => self.has_a_role_at(person, place).await,
            Members::Assignable { reported_by } => match place {
                Place::Project(project) => self.may_be_assigned(person, project, reported_by).await,
                Place::Organization => Ok(false),
            },
        }
    }

    /// Whether a person can be assigned to an issue of the project that `reported_by` reported:
    /// they may see that issue (they see the project and hold *See issues* there, or they reported
    /// it). The one rule, for choosing from the list and for checking a choice. A person who does
    /// not exist, and one who may not see it, are alike.
    pub(crate) async fn may_be_assigned(
        &self,
        person: UserId,
        project: ProjectId,
        reported_by: UserId,
    ) -> Result<bool, AppError> {
        Ok(
            if_there(self.evaluate(person, Place::Project(project)).await)?
                .is_some_and(|theirs| theirs.sees_issue(reported_by)),
        )
    }

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
    let place: Place = Place::from(input.place);
    let checker = ctx.require(&actor, Permission::CheckAccess, place).await?;
    let after = ctx.people_position(input.cursor.as_deref()).await?;
    let names_shown = checker.people_level(PeopleKind::ProjectMembers, PeopleContext::Management)
        == PeopleLevel::Names;
    if !names_shown {
        return Ok(checker.authorize(PeopleToCheckPage {
            items: Vec::new(),
            next_cursor: None,
        }));
    }
    let wanted = page_len(Some(input.limit));
    let page = ctx
        .members_page(
            place,
            input.search.as_deref(),
            after,
            wanted,
            if checker.manages_people() {
                Members::Everybody
            } else {
                Members::WithRole
            },
        )
        .await?;
    let page = Cut::of(page, wanted);
    let next_cursor = page.next_cursor(write_cursor);
    let mut items = Vec::with_capacity(page.items.len());
    for person in &page.items {
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
pub(crate) fn position_of(person: &Person) -> PeoplePosition {
    PeoplePosition {
        name: person.display_name.clone(),
        id: person.id,
    }
}

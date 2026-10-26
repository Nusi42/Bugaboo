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
use bb_policy::{Authorized, Evaluation, Masked, PeopleContext, Permission, PersonRef};
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
pub(crate) enum Members<'a> {
    /// Everybody, for a person who manages people or roles.
    Everybody,
    /// The people who have a role that reaches the place.
    WithRole,
    /// The people the `viewer` (the evaluation of `actor`) may name in an item of the place, by
    /// the one rule, [`Ctx::may_name`]: every person is judged by it. A viewer who is shown other
    /// people only as a role can name nobody but themselves, so only their own name is looked at
    /// instead of reading all people to refuse each (the page is the viewer alone, or empty).
    Naming {
        /// The person who asks.
        actor: UserId,
        /// What they are shown.
        viewer: &'a Evaluation,
        /// The kind of people information of the field the person is named in.
        kind: PeopleKind,
        /// Who reported the issue the person is named in, or `None` for a project.
        reported_by: Option<UserId>,
    },
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
        members: Members<'_>,
    ) -> Result<Vec<Person>, AppError> {
        if let Members::Naming {
            actor,
            viewer,
            kind,
            ..
        } = members
            && !names_other_people(viewer, kind)
        {
            // Only the viewer can be named: the first page holds them (when the search fits their
            // name), a later one nobody. The person still goes through `is_listed`.
            let own = if after.is_none() {
                self.own_page(actor, search).await?
            } else {
                Vec::new()
            };
            let mut page = Vec::with_capacity(own.len());
            for person in own {
                if self.is_listed(person.id, place, members).await? {
                    page.push(person);
                }
            }
            return Ok(page);
        }
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
        members: Members<'_>,
    ) -> Result<bool, AppError> {
        match members {
            Members::Everybody => Ok(true),
            Members::WithRole => self.has_a_role_at(person, place).await,
            Members::Naming {
                actor,
                viewer,
                kind,
                reported_by,
            } => match place {
                Place::Project(project) => {
                    self.may_name(actor, viewer, kind, person, project, reported_by)
                        .await
                }
                Place::Organization => Ok(false),
            },
        }
    }

    /// The person alone, when the words of the search fit their name (all the words, in any
    /// order, ignoring case) or there are none: the list of somebody who sees other people only
    /// as a role.
    async fn own_page(
        &self,
        person: UserId,
        search: Option<&str>,
    ) -> Result<Vec<Person>, AppError> {
        let Some(found) = self
            .store()
            .person(person)
            .await
            .map_err(|error| internal::<S>(&error))?
        else {
            return Ok(Vec::new());
        };
        let name = found.display_name.to_lowercase();
        let fits = search
            .unwrap_or_default()
            .split_whitespace()
            .all(|word| name.contains(&word.to_lowercase()));
        Ok(if fits { vec![found] } else { Vec::new() })
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

    /// Whether a person may be named in a field of an item of the project: in an issue that
    /// `reported_by` reported, someone who may see that issue (`may_be_assigned`); in a project
    /// (no reporter), someone who may see the project. The one rule for *Assigned to*, for the own
    /// fields that hold people, and for the list to choose from.
    pub(crate) async fn may_be_named(
        &self,
        person: UserId,
        project: ProjectId,
        reported_by: Option<UserId>,
    ) -> Result<bool, AppError> {
        match reported_by {
            Some(reporter) => self.may_be_assigned(person, project, reporter).await,
            None => Ok(
                if_there(self.evaluate(person, Place::Project(project)).await)?
                    .is_some_and(|theirs| theirs.sees_project()),
            ),
        }
    }

    /// Whether `viewer` (the evaluation of `actor`) may name a person that exists: themselves, or
    /// anybody when they are shown people by name (`names_other_people`, of the kind of the field). A viewer who is shown
    /// others only as a role has no ID to name them by, so one sent anyway is refused alike for a
    /// person who is there and one who is not. The part of `may_name` that holds without access.
    pub(crate) async fn may_name_person(
        &self,
        actor: UserId,
        viewer: &Evaluation,
        kind: PeopleKind,
        person: UserId,
    ) -> Result<bool, AppError> {
        Ok((person == actor || names_other_people(viewer, kind))
            && self
                .store()
                .person(person)
                .await
                .map_err(|error| internal::<S>(&error))?
                .is_some())
    }

    /// Whether `viewer` may name `person` in a field of an item of the project: the person exists
    /// and is one the viewer may name (`may_name_person`), and may be named there
    /// (`may_be_named`). The one rule of *Assigned to* and of the own fields that hold people, so
    /// the list to choose from, a change of *Assigned to* and a value of an own field agree.
    pub(crate) async fn may_name(
        &self,
        actor: UserId,
        viewer: &Evaluation,
        kind: PeopleKind,
        person: UserId,
        project: ProjectId,
        reported_by: Option<UserId>,
    ) -> Result<bool, AppError> {
        Ok(self.may_name_person(actor, viewer, kind, person).await?
            && self.may_be_named(person, project, reported_by).await?)
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
/// `access.forbidden` without *Check access* there (`resource.not-found` for a project the
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

/// Whether the viewer is shown other people by name where people are named in a field of an item
/// (the kind of the field and *Project members* both show names, ADR-0022): only then can they
/// choose anybody but themselves, whose name they know.
pub(crate) fn names_other_people(viewer: &Evaluation, kind: PeopleKind) -> bool {
    [kind, PeopleKind::ProjectMembers]
        .into_iter()
        .all(|each| viewer.sees_names(each))
}

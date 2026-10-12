//! The people an issue can be assigned to (ISS-01, M3-F28): the members of the project, so nobody
//! is assigned to an issue they cannot open.
//!
//! A person can be assigned to an issue only if they may see it: the list holds the people who see
//! the project and hold *See issues* there, and the reader. Only somebody who may set *Assigned
//! to* can ask (a role that may change issues or make them, and a field mask that lets them
//! change that field): *See issues* alone is not enough. Names of others are listed only when
//! both *Assigned to* and *Project members* show names to the reader (ADR-0022): a person shown
//! as a role or as *Someone* has no ID to be chosen by, so such a reader is offered only
//! themselves, whose name they know.

use bb_core::ErrorCode;
use bb_core::access::FieldLevel;
use bb_core::fields::{Field, FieldKey};
use bb_core::id::{FieldId, IssueTypeId, UserId};
use bb_core::own_fields::FieldFor;
use bb_core::paging::{Cut, page_len};
use bb_core::people_kinds::PeopleKind;
use bb_core::person::Person;
use bb_policy::{Authorized, Evaluation, Masked, PeopleContext, Permission, PersonRef, Place};
use serde::Serialize;

use crate::access::{Members, names_other_people};
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::people::write_cursor;

/// List the people an issue of a project can be assigned to.
pub struct AssignablePeopleQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The project, by its ID or its short name (an old one too).
    pub project: String,
    /// Words that every found name holds; the text may be empty, which finds everybody.
    pub search: Option<String>,
    /// Where the page starts: the `next_cursor` of the page before.
    pub cursor: Option<String>,
    /// How many people a page holds at most.
    pub limit: u32,
    /// An own field of people the list is for: the person then needs the right to change that
    /// field instead of *Assigned to*.
    pub field: Option<FieldId>,
    /// The issue the list is for, by ID or readable ID: whoever reported it decides who may see
    /// it. Without it the list is for an issue the person would report now.
    pub issue: Option<String>,
    /// The type of the issue that is being made, for a field whose level depends on the type.
    pub issue_type: Option<IssueTypeId>,
}

/// One page of people, by name.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AssignablePeoplePage {
    /// The people, each shown as the person who asks may see them: by name.
    pub items: Vec<PersonRef>,
    /// The cursor of the next page, if there is one.
    pub next_cursor: Option<String>,
}

impl Masked for AssignablePeoplePage {}

/// The members of a project to choose from for *Assigned to*, a page at a time, by name.
///
/// # Errors
/// `resource.not_found` for a project that does not exist or that the person may not see,
/// `access.forbidden` without the right to set *Assigned to* there, `request.invalid` for a cursor that is not
/// one.
pub async fn assignable_people<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: AssignablePeopleQuery,
) -> Result<Authorized<AssignablePeoplePage>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let found = ctx.find_project(&input.project).await?;
    let place = found.as_ref().map_or(Place::Organization, |found| {
        Place::Project(found.project.id)
    });
    let held = ctx.evaluation_of(&actor, place).await?;
    if found.is_none() || !held.sees_project() {
        return Err(AppError::Refused(ErrorCode::NotFound));
    }
    let you = actor.person.id;
    let issue = match &input.issue {
        Some(address) => Some(
            ctx.find_issue(address)
                .await?
                .map(|found| found.issue)
                .filter(|issue| {
                    place == Place::Project(issue.project)
                        && issue.deleted_at.is_none()
                        && held.sees_issue(issue.reported_by)
                })
                .ok_or(AppError::Refused(ErrorCode::NotFound))?,
        ),
        None => None,
    };
    let issue_type = issue
        .as_ref()
        .map(|issue| issue.issue_type)
        .or(input.issue_type);
    let (may_ask, reported_by) = match input.field {
        None => (
            may_set_assigned_to(&held),
            Some(issue.as_ref().map_or(you, |issue| issue.reported_by)),
        ),
        Some(field) => {
            let applies_to = ctx
                .field_catalogue()
                .await?
                .iter()
                .find(|definition| definition.field.id == field)
                .map(|definition| definition.field.applies_to);
            let may_change =
                held.field_level_for(issue_type, FieldKey::Own(field)) >= FieldLevel::CanChange;
            match applies_to {
                Some(FieldFor::Issues) => (
                    may_change && may_set_issues(&held),
                    Some(issue.as_ref().map_or(you, |issue| issue.reported_by)),
                ),
                Some(FieldFor::Projects) => (
                    may_change && held.allows(Permission::ChangeProjectSettings),
                    None,
                ),
                None => (false, None),
            }
        }
    };
    if !may_ask {
        return Err(ErrorCode::Forbidden.into());
    }
    let after = ctx.people_position(input.cursor.as_deref()).await?;
    let wanted = page_len(Some(input.limit));
    let people = if names_other_people(&held) {
        let members = Members::Naming { reported_by };
        ctx.members_page(place, input.search.as_deref(), after, wanted, members)
            .await?
    } else if after.is_none() {
        // The reader knows their own name, whatever else they see: they can assign themselves.
        ctx.own_page(you, input.search.as_deref()).await?
    } else {
        Vec::new()
    };
    let page = Cut::of(people, wanted);
    let next_cursor = page.next_cursor(write_cursor);
    let mut items = Vec::with_capacity(page.items.len());
    for person in &page.items {
        items.push(
            ctx.person_ref(
                &held,
                PeopleKind::AssignedTo,
                PeopleContext::Ordinary,
                person.id,
            )
            .await?,
        );
    }
    Ok(held.authorize(AssignablePeoplePage { items, next_cursor }))
}

/// Whether the person may make or change issues here, and see them.
fn may_set_issues(held: &Evaluation) -> bool {
    held.allows(Permission::SeeIssues)
        && (held.allows(Permission::ChangeIssues)
            || held.allows(Permission::ChangeOwnIssues)
            || held.allows(Permission::MakeIssues))
}

/// Whether the person may set *Assigned to*: they set issues, and the field mask lets them change it.
fn may_set_assigned_to(held: &Evaluation) -> bool {
    may_set_issues(held) && held.field_mask().can_change(Field::AssignedTo)
}

impl<S: AuthStore> Ctx<'_, S> {
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
}

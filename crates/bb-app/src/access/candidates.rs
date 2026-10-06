//! The people *Check access* can choose from (SEC-03, M2-Q3).
//!
//! Only a checker who sees names here can choose anybody: a person shown as a role or as
//! *Someone* has no ID to be chosen by, so for such a checker the list is empty and says nothing
//! about who is there.

use bb_core::ErrorCode;
use bb_core::access::PeopleLevel;
use bb_core::id::{ProjectId, UserId};
use bb_core::people_kinds::PeopleKind;
use bb_policy::{Authorized, Masked, PeopleContext, Permission, PersonRef};
use serde::Serialize;

use super::{PersonName, Place};
use crate::auth::{AppError, AuthStore, Ctx, internal};
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
pub struct PersonPage {
    /// The people, each shown as the person who asks may see them: by name.
    pub items: Vec<PersonRef>,
    /// The cursor of the next page, if there is one.
    pub next_cursor: Option<String>,
}

impl Masked for PersonPage {}

fn write_cursor(last: &PersonName) -> String {
    format!("{}\t{}", last.display_name.to_lowercase(), last.id)
}

fn read_cursor(text: &str) -> Result<(String, UserId), ()> {
    let (name, id) = text.split_once('\t').ok_or(())?;
    Ok((name.to_owned(), id.parse().map_err(|_| ())?))
}

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
) -> Result<Authorized<PersonPage>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let place: Place = place_of(input.place);
    let checker = ctx.require(&actor, Permission::CheckAccess, place).await?;
    let after = input
        .cursor
        .as_deref()
        .map(read_cursor)
        .transpose()
        .map_err(|()| AppError::Refused(ErrorCode::InvalidRequest))?;
    let names_shown = checker.people_level(PeopleKind::ProjectMembers, PeopleContext::Management)
        == PeopleLevel::Names;
    if !names_shown {
        return Ok(checker.authorize(PersonPage {
            items: Vec::new(),
            next_cursor: None,
        }));
    }
    let words: Vec<String> = input
        .search
        .as_deref()
        .map(|text| {
            text.to_lowercase()
                .split_whitespace()
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    let mut candidates: Vec<PersonName> = ctx
        .store()
        .people_names()
        .await
        .map_err(|error| internal::<S>(&error))?
        .into_iter()
        .filter(|person| {
            let name = person.display_name.to_lowercase();
            words.iter().all(|word| name.contains(word))
        })
        .collect();
    candidates.sort_by(|a, b| {
        a.display_name
            .to_lowercase()
            .cmp(&b.display_name.to_lowercase())
            .then(a.id.cmp(&b.id))
    });
    let limit = usize::try_from(bb_core::paging::page_size(Some(input.limit))).unwrap_or(1);
    let manages = checker.manages_people();
    let mut page: Vec<PersonName> = Vec::with_capacity(limit + 1);
    for person in candidates.into_iter().filter(|person| {
        after.as_ref().is_none_or(|(name, id)| {
            (person.display_name.to_lowercase(), person.id) > (name.clone(), *id)
        })
    }) {
        if page.len() > limit {
            break;
        }
        // Whoever does not manage people or roles chooses among the people who have a role that
        // reaches the place, as *Project members* are shown (M2-Q3).
        if manages || ctx.has_a_role_at(person.id, place).await? {
            page.push(person);
        }
    }
    let more = page.len() > limit;
    page.truncate(limit);
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
    Ok(checker.authorize(PersonPage { items, next_cursor }))
}

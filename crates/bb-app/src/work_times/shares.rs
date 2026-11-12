//! The share of a person in a project (PLAN-15, ADR-0030, decision 2 and amendment M8): set by a
//! person who decides who has which role in the project, read by the person, and shown on *Project
//! members*. A person with no row has 100 percent. While the person has no role in the project the
//! share counts as nothing and is kept, and it counts again when a role comes back; nothing deletes
//! it before the person leaves (business data is deleted only explicitly).
//!
//! When the shares of one person add up to more than 100 percent, the person sees the sum. A person
//! who decides who has which role in a project the person works in is told the fact only, with the
//! other projects they may see by name; the sum would tell how much the others hold.

use std::collections::HashMap;

use bb_core::ErrorCode;
use bb_core::access::PeopleLevel;
use bb_core::event::RecordKind;
use bb_core::id::{ProjectId, UserId};
use bb_core::paging::{Cut, page_len};
use bb_core::people_kinds::PeopleKind;
use bb_core::work_times::{Share, sum_of_shares};
use bb_policy::{Authorized, PeopleContext, Permission, Place};

use super::types::{
    ChangedShare, MemberShare, ProjectShares, ProjectSharesQuery, SetShare, ShareItem,
    SharesOfPerson,
};
use crate::access::{Actor, Members, ProjectLabel, place_of_address};
use crate::auth::{AppError, AuthStore, Ctx, Stops, by, change_failed, entity, internal};
use crate::event_members::share_recorded;
use crate::people::write_cursor;

/// The label a project that the reader may not see sorts under: after every project they may see.
fn label_order(label: &ProjectLabel) -> (bool, String) {
    match label {
        ProjectLabel::Visible { name, .. } => (false, name.to_lowercase()),
        ProjectLabel::Hidden => (true, String::new()),
    }
}

/// What a person's shares add up to: the shares that count, and whether that is over 100 percent.
fn summed(items: &[ShareItem], extra: u32) -> (u32, bool) {
    let sum = extra
        + sum_of_shares(
            items
                .iter()
                .filter(|item| item.applies)
                .filter_map(|item| Share::new(item.percent)),
        );
    (sum, sum > u32::from(Share::PERCENT_MAX))
}

/// The project of a place that an address of a project made.
fn project_of(place: Place) -> Result<ProjectId, AppError> {
    match place {
        Place::Project(project) => Ok(project),
        Place::Organization => Err(ErrorCode::NotFound.into()),
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The place an address of a project (its ID or its short name) names; a project that is not
    /// there is a place that nobody holds anything in, so the answer is `resource.not-found`.
    async fn project_place(&self, address: &str) -> Result<Place, AppError> {
        let found = self.find_project(address).await?;
        place_of_address(found.map(|found| found.project.id))
    }

    /// The shares in `rows` of one person as items, for a reader: each project as the reader may
    /// name it, and whether the share applies (the person has a role there).
    async fn share_items(
        &self,
        actor: &Actor,
        person: UserId,
        rows: &[(ProjectId, Share)],
    ) -> Result<Vec<ShareItem>, AppError> {
        let pairs: Vec<(UserId, ProjectId)> =
            rows.iter().map(|(project, _)| (person, *project)).collect();
        self.evaluate_people_in(&pairs).await?;
        let mut items = Vec::with_capacity(rows.len());
        for (project, share) in rows {
            items.push(ShareItem {
                project: self.project_label(actor, *project).await?,
                percent: share.percent(),
                applies: self.has_a_role_at(person, Place::Project(*project)).await?,
            });
        }
        items.sort_by_key(|item| label_order(&item.project));
        Ok(items)
    }

    /// The shares a person set, with what they add up to.
    pub(super) async fn shares_of_person(
        &self,
        actor: &Actor,
        person: UserId,
    ) -> Result<SharesOfPerson, AppError> {
        let rows: Vec<(ProjectId, Share)> = self
            .store()
            .shares_of(vec![person])
            .await
            .map_err(|error| internal::<S>(&error))?
            .into_iter()
            .map(|(_, project, share)| (project, share))
            .collect();
        let items = self.share_items(actor, person, &rows).await?;
        let (sum, over_100) = summed(&items, 0);
        Ok(SharesOfPerson {
            items,
            sum,
            over_100,
        })
    }
}

/// Sets the share of a person in a project. Needs *Decide who has which role* in the project.
///
/// # Errors
/// `work-times.share-invalid` for more than 100 percent, `access.forbidden` without the permission
/// (`resource.not-found` for a project the reader may not know), and `resource.not-found` for a
/// person that does not exist.
pub async fn set_share<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: SetShare,
) -> Result<Authorized<ChangedShare>, AppError> {
    // A key is judged by the permission, which no key holds (*Decide who has which role*).
    let actor = ctx.actor(&input.session_token).await?;
    let place = ctx.project_place(&input.project).await?;
    let decider = ctx.require(&actor, Permission::DecideRoles, place).await?;
    let share = Share::new(input.percent).ok_or(ErrorCode::ShareInvalid)?;
    if ctx.person_known(input.person).await?.is_none() {
        return Err(ErrorCode::NotFound.into());
    }
    let (person, project) = (input.person, project_of(place)?);
    let before = ctx
        .store()
        .share(person, project)
        .await
        .map_err(|error| internal::<S>(&error))?;
    if before == Some(share) {
        return Ok(decider.authorize(ChangedShare {
            percent: share.percent(),
        }));
    }
    let event = ctx.event(
        by(actor.person.id),
        entity(RecordKind::Share, person),
        share_recorded(person, project, before, share),
    )?;
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move { S::put_share(transaction, person, project, share).await })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::NONE))?;
    Ok(decider.authorize(ChangedShare {
        percent: share.percent(),
    }))
}

/// The people of a project with their shares there, a page at a time, by name. Every reader who
/// sees the project and is shown people of the kind *Work times* reads the shares; a reader who
/// decides who has which role there also reads what each person's shares add up to, with the other
/// projects they may see named and the ones they may not only counted.
///
/// # Errors
/// `resource.not-found` for a project the reader may not see, `request.invalid` for a cursor that
/// is none.
pub async fn project_shares<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ProjectSharesQuery,
) -> Result<Authorized<ProjectShares>, AppError> {
    let actor = ctx.work_times_actor(&input.session_token).await?;
    let place = ctx.project_place(&input.project).await?;
    let project = project_of(place)?;
    let reader = ctx.require(&actor, Permission::SeeProjects, place).await?;
    let decides = reader.allows(Permission::DecideRoles);
    let level = reader.people_level(PeopleKind::WorkTimes, PeopleContext::WorkTimes);
    if level == PeopleLevel::Hidden {
        return Ok(reader.authorize(ProjectShares {
            items: Vec::new(),
            next_cursor: None,
        }));
    }
    let after = ctx.people_position(input.cursor.as_deref()).await?;
    let wanted = page_len(Some(input.limit));
    let people = ctx
        .members_page(place, None, after, wanted, Members::WithRole)
        .await?;
    let page = Cut::of(people, wanted);
    let next_cursor = page.next_cursor(write_cursor);
    let ids: Vec<UserId> = page.items.iter().map(|person| person.id).collect();
    ctx.preload_people(&ids).await?;
    let here: HashMap<UserId, Share> = ctx
        .store()
        .shares_in(project, ids.clone())
        .await
        .map_err(|error| internal::<S>(&error))?
        .into_iter()
        .collect();
    let mut others: HashMap<UserId, Vec<(ProjectId, Share)>> = HashMap::new();
    if decides {
        for (person, other, share) in ctx
            .store()
            .shares_of(ids.clone())
            .await
            .map_err(|error| internal::<S>(&error))?
        {
            if other != project {
                others.entry(person).or_default().push((other, share));
            }
        }
    }
    // The rights of every person in each project they share in are read together.
    let pairs: Vec<(UserId, ProjectId)> = others
        .iter()
        .flat_map(|(person, rows)| rows.iter().map(|(project, _)| (*person, *project)))
        .collect();
    ctx.evaluate_people_in(&pairs).await?;
    let mut items = Vec::with_capacity(ids.len());
    for person in ids {
        let percent = here.get(&person).copied().unwrap_or(Share::START).percent();
        let mut member = MemberShare {
            person: ctx
                .person_ref(
                    &reader,
                    PeopleKind::WorkTimes,
                    PeopleContext::WorkTimes,
                    person,
                )
                .await?,
            percent,
            over_100: None,
            others: Vec::new(),
            others_hidden: false,
        };
        if decides {
            let rows = others.remove(&person).unwrap_or_default();
            let all = ctx.share_items(&actor, person, &rows).await?;
            member.over_100 = Some(summed(&all, u32::from(percent)).1);
            member.others_hidden = all.iter().any(|item| item.project == ProjectLabel::Hidden);
            member.others = all
                .into_iter()
                .filter(|item| item.project != ProjectLabel::Hidden)
                .collect();
        }
        items.push(member);
    }
    Ok(reader.authorize(ProjectShares { items, next_cursor }))
}

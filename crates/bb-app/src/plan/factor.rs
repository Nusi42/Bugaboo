//! The time factor of an issue (PLAN-14, ADR-0030, decision 4 and amendment M8): the estimate of
//! the person on the bar, given as a factor (*×1.5*) or as a length (*Takes me 10 days*), with who
//! set it.
//!
//! - The person on the bar (the first assigned person) may always change it on their issue; anybody
//!   else needs to be allowed to change the issue and the field *Time factor*.
//! - It is never set by AI or agents: no tool of the registry names it, and an agent is refused here.
//! - It belongs to the issue: a change of it is a change of the issue (one event, the version counts
//!   up), and the change of the first assigned person takes it back to ×1 ([`reset_with_first`]).
//! - The person on the bar hears of a change somebody else made (*Your time factor was changed*).

use bb_core::ErrorCode;
use bb_core::access::PeopleLevel;
use bb_core::event::{FieldChange, RecordKind};
use bb_core::fields::{ASSIGNED_TO_PEOPLE, Field};
use bb_core::id::{IssueId, UserId};
use bb_core::issue::Issue;
use bb_core::people_kinds::PeopleKind;
use bb_core::plan::TimeFactor;
use bb_core::plan::record::IssueTimeFactor;
use bb_core::time::Minutes;
use bb_core::version::{Version, next as next_version};
use bb_policy::{Authorized, Evaluation, Masked, PeopleContext, PersonRef, Place};
use serde::Serialize;
use serde_json::json;

use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, by, entity, internal, set_to};
use crate::event_members::{difference, issue_counted_up};
use crate::issues::member;
use crate::projects::Why;

/// How a time factor is given.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FactorGiven {
    /// A factor, in percent of the estimate (150 is *×1.5*).
    Percent(u16),
    /// *Takes me …*: the length the issue takes the person; the factor is worked out against
    /// *Estimated time*.
    Takes(Minutes),
}

/// Set the time factor of an issue.
pub struct SetTimeFactor {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The issue, by its readable ID or its internal ID.
    pub issue: String,
    /// The version of the issue the person looked at.
    pub version: i64,
    /// The factor, or the length it is worked out from.
    pub given: FactorGiven,
}

/// Read the time factor of an issue.
pub struct TimeFactorQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The issue, by its readable ID or its internal ID.
    pub issue: String,
}

/// The time factor of an issue as the reader may see it: the field *Time factor* decides whether
/// they see it at all, and the people in it are shown as *Who sees people* allows.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TimeFactorView {
    /// The issue.
    pub issue: IssueId,
    /// The version of the issue now.
    pub version: i64,
    /// The factor, in percent of the estimate; 100 when nobody set one.
    pub percent: u16,
    /// Who set it; none for ×1.
    pub set_by: Option<PersonRef>,
    /// Whether the person on the bar set it themselves (*Lea's own estimate*); only for a reader
    /// who is shown that person by name.
    pub own_estimate: bool,
    /// The factor again, for the person on the bar alone: they always see their own estimate, also
    /// where the field is hidden from their role (ADR-0030, decision 4).
    pub own_percent: Option<u16>,
    /// The person on the bar; none for an issue assigned to nobody.
    pub person: Option<PersonRef>,
}

impl Masked for TimeFactorView {}

/// The person on the bar of an issue assigned to `people` (in their order): the first of them.
#[must_use]
pub fn person_on_bar(people: &[UserId]) -> Option<UserId> {
    people.first().copied()
}

/// The members of the event of a change of the time factor: the factor before and after (none for
/// ×1), and the person on the bar it is for.
fn factor_changes(
    before: Option<&IssueTimeFactor>,
    after: Option<&IssueTimeFactor>,
    person: UserId,
) -> Vec<FieldChange> {
    let percent = |factor: Option<&IssueTimeFactor>| factor.map(|kept| kept.factor.percent());
    difference(
        member::TIME_FACTOR,
        json!(percent(before)),
        json!(percent(after)),
    )
    .into_iter()
    .chain([set_to(member::TIME_FACTOR_FOR, json!(person.to_string()))])
    .collect()
}

/// The change that the change of an issue from `people_now` to `people_next` makes to its time
/// factor `stored`: back to ×1 when the first assigned person changes (another person is put first,
/// or the first is taken off), nothing when only the others change (ADR-0030, amendment M8). The
/// member of the event, which stands beside the change of *Assigned to* and so says why.
#[must_use]
pub fn reset_with_first(
    stored: Option<&IssueTimeFactor>,
    people_now: &[UserId],
    people_next: &[UserId],
) -> Option<FieldChange> {
    let stored = stored?;
    (person_on_bar(people_now) != person_on_bar(people_next)).then(|| FieldChange {
        field: member::TIME_FACTOR.to_owned(),
        before: Some(json!(stored.factor.percent())),
        after: Some(serde_json::Value::Null),
    })
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The people an issue is assigned to, in their order.
    pub(crate) async fn assigned_to(&self, issue: IssueId) -> Result<Vec<UserId>, AppError> {
        Ok(self
            .store()
            .assigned_people(issue)
            .await
            .map_err(|error| internal::<S>(&error))?
            .into_iter()
            .map(|assigned| assigned.person)
            .collect())
    }

    /// The first assigned person of an issue.
    async fn first_assigned(&self, issue: IssueId) -> Result<Option<UserId>, AppError> {
        Ok(person_on_bar(&self.assigned_to(issue).await?))
    }

    /// What the reader may see of a person on a bar: by name, as a role, or nobody (*Hidden*).
    /// The person themselves always sees themselves.
    pub(crate) async fn shown_person(
        &self,
        held: &Evaluation,
        person: UserId,
    ) -> Result<Option<PersonRef>, AppError> {
        let hidden = held.person() != Some(person)
            && held.people_level(ASSIGNED_TO_PEOPLE, PeopleContext::Ordinary)
                == PeopleLevel::Hidden;
        if hidden {
            return Ok(None);
        }
        Ok(Some(
            self.person_ref(held, ASSIGNED_TO_PEOPLE, PeopleContext::Ordinary, person)
                .await?,
        ))
    }

    /// The view of the time factor of `issue` for a reader who holds `held` in its project. Who set
    /// it is told only as openly as the person on the bar: that it is the person's own estimate,
    /// and who set it, only to a reader who is shown the person by name; any other reader is shown
    /// the person on the bar as they see them, whoever set it.
    async fn factor_view(
        &self,
        held: &Evaluation,
        issue: &Issue,
        version: i64,
        factor: Option<&IssueTimeFactor>,
        first: Option<UserId>,
    ) -> Result<TimeFactorView, AppError> {
        let person = match first {
            Some(first) => self.shown_person(held, first).await?,
            None => None,
        };
        let named = names_person(held, first, person.as_ref());
        let percent = factor.map_or(TimeFactor::ONE, |kept| kept.factor).percent();
        let set_by = match factor {
            // Not more openly than the person on the bar, whoever set it: a factor the person set
            // and one somebody else set look alike.
            Some(_) if !named => Some(person.clone().unwrap_or(PersonRef::Someone)),
            Some(kept) => Some(
                self.person_ref(
                    held,
                    PeopleKind::Authors,
                    PeopleContext::Ordinary,
                    kept.set_by,
                )
                .await?,
            ),
            None => None,
        };
        Ok(TimeFactorView {
            issue: issue.id,
            version,
            percent,
            set_by,
            own_estimate: named && factor.is_some_and(|kept| kept.set_by == kept.person),
            // The person on the bar always sees their own estimate (ADR-0030, decision 4).
            own_percent: first
                .filter(|first| held.person() == Some(*first))
                .map(|_| percent),
            person,
        })
    }
}

/// Whether the reader is shown the person on the bar by name: they are the person, or *Who sees
/// people* names them.
pub(crate) fn names_person(
    held: &Evaluation,
    first: Option<UserId>,
    shown: Option<&PersonRef>,
) -> bool {
    first.is_some_and(|first| held.person() == Some(first))
        || matches!(shown, Some(PersonRef::Name { .. }))
}

/// The time factor of an issue the reader may see.
///
/// # Errors
/// `resource.not-found` for an issue that does not exist, is in the trash or that the person may
/// not see.
pub async fn time_factor<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    query: TimeFactorQuery,
) -> Result<Authorized<TimeFactorView>, AppError> {
    let actor = ctx.actor(&query.session_token).await?;
    let (issue, held) = ctx.visible_issue(&actor, &query.issue).await?;
    let factor = ctx
        .store()
        .time_factor(issue.id)
        .await
        .map_err(|error| internal::<S>(&error))?;
    let first = ctx.first_assigned(issue.id).await?;
    let view = ctx
        .factor_view(&held, &issue, issue.version, factor.as_ref(), first)
        .await?;
    Ok(held.authorize(view))
}

/// Whether `actor` may set the time factor of `issue`, whose person on the bar is `first`: that
/// person always may; anybody else needs to be allowed to change the issue and the field. No agent
/// ever may (ADR-0030, amendment M8).
fn ensure_may_set(
    actor: &Actor,
    held: &Evaluation,
    issue: &Issue,
    first: UserId,
) -> Result<(), AppError> {
    if actor.agent().is_some() {
        return Err(ErrorCode::Forbidden.into());
    }
    // The person on the bar always may, with a credential that may change things at all.
    let allowed = (actor.person.id == first && actor.key_may_change())
        || (held.may_change_issue(issue.reported_by)
            && held
                .field_mask_for(Some(issue.issue_type))
                .can_change(Field::TimeFactor));
    if allowed {
        Ok(())
    } else {
        Err(ErrorCode::Forbidden.into())
    }
}

/// Sets the time factor of an issue, as a factor or as *Takes me …*. ×1 takes it away.
///
/// # Errors
/// `resource.not-found` for an issue the person may not see; `access.forbidden` for an agent, or
/// for somebody who is not the person on the bar and may not change the issue or the field;
/// `request.invalid` for an issue assigned to nobody, a factor outside the range, or a length for an
/// issue without an estimate; `edit.version-changed` when the issue changed since it was read.
pub async fn set_time_factor<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: SetTimeFactor,
) -> Result<Authorized<TimeFactorView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let (issue, held) = ctx.visible_issue(&actor, &input.issue).await?;
    let first = ctx
        .first_assigned(issue.id)
        .await?
        .ok_or(AppError::Refused(ErrorCode::InvalidRequest))?;
    ensure_may_set(&actor, &held, &issue, first)?;
    let record = entity(RecordKind::Issue, issue.id);
    let place = Place::Project(issue.project);
    if issue.version != input.version {
        return Err(AppError::stale(record, place, actor.person.id));
    }
    // A length is worked out against the estimate only for a reader who may see it: the refusal
    // is the same with or without one, so it tells nothing of a hidden estimate.
    let estimate = issue.estimated_time.filter(|_| {
        held.field_mask_for(Some(issue.issue_type))
            .can_see(Field::EstimatedTime)
    });
    let factor = match input.given {
        FactorGiven::Percent(percent) => TimeFactor::new(percent),
        FactorGiven::Takes(takes) => {
            estimate.and_then(|estimate| TimeFactor::of_length(estimate, takes))
        }
    }
    .ok_or(AppError::Refused(ErrorCode::InvalidRequest))?;
    let before = ctx
        .store()
        .time_factor(issue.id)
        .await
        .map_err(|error| internal::<S>(&error))?;
    let after = (factor != TimeFactor::ONE).then_some(IssueTimeFactor {
        issue: issue.id,
        factor,
        person: first,
        set_by: actor.person.id,
    });
    if before == after {
        let view = ctx
            .factor_view(&held, &issue, issue.version, before.as_ref(), Some(first))
            .await?;
        return Ok(held.authorize(view));
    }
    let changes = issue_counted_up(
        factor_changes(before.as_ref(), after.as_ref(), first),
        issue.version,
    );
    let event = ctx.event(by(actor.person.id), record.clone(), changes)?;
    let mut written = issue.clone();
    written.updated_at = ctx.now();
    let (why, version, kept) = (Why::default(), issue.version, after.clone());
    let stopped = why.clone();
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                if !S::update_issue(transaction, &written, Version::new(version)).await? {
                    return Err(stopped.stop::<S>(ErrorCode::VersionChanged));
                }
                match &kept {
                    Some(factor) => S::put_time_factor(transaction, factor).await,
                    None => S::delete_time_factor(transaction, written.id)
                        .await
                        .map(|_| ()),
                }
            })
        })
        .await
        .map_err(|error| {
            why.answer::<S>(&error, ErrorCode::VersionChanged).about(
                &record,
                place,
                actor.person.id,
            )
        })?;
    let view = ctx
        .factor_view(
            &held,
            &issue,
            next_version(issue.version),
            after.as_ref(),
            Some(first),
        )
        .await?;
    Ok(held.authorize(view))
}

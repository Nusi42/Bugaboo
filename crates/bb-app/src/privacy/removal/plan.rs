//! The plan of *Remove a person* (PRIV-04, ADR-0044, decision 6): before anything happens, three
//! lists, all read from the walk of the person's data (`PersonData`) and the keep periods
//! (`KeepPeriods`): what is deleted now, what stays shown as *Former user*, and what is kept for
//! its keep period and until when. Nothing in the plan names a table: a kind is named by the plain
//! name of its entity, as in *Download my data*.

use std::collections::BTreeMap;

use bb_core::id::UserId;
use bb_core::keeping::{KeepKind, KeptUntil};
use bb_core::message::{self, Message};
use bb_core::people_kinds::PeopleKind;
use bb_core::removal::{AtRemoval, at_removal, is_data_export, kept_when_shared};

use bb_core::settings::privacy::{PRIVACY_REMOVED_TEXTS, RemovedTexts};
use bb_core::time::day::date_of;
use bb_core::wording::Region;
use bb_policy::{Authorized, Masked, PeopleContext, Permission, PersonRef, Place};
use jiff::civil::Date;
use serde::Serialize;

use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::ports::KeepPeriods;
use crate::privacy::export::title_of;

/// The plan of the removal of a person.
pub struct RemovalPlanQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The person.
    pub person: UserId,
}

/// One kind of record in a list of the plan, and how many of the person's rows it has.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PlanKind {
    /// The kind, in its plain name (`personal-kind.…`).
    pub title: String,
    /// How many rows name the person.
    pub rows: u64,
}

/// One kind the keep rules keep for its period, and until when.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct KeptByLaw {
    /// The row of the keep rules.
    pub kind: KeepKind,
    /// The kind, in the words of the keep rules.
    pub title: String,
    /// How many rows name the person; none for the backups, which hold everything.
    pub rows: Option<u64>,
    /// The last day it is kept; none when no day can be named.
    pub until: Option<Date>,
    /// Why no day can be named, in words: the backups are kept by their rotation whatever their
    /// age (ADR-0045, M9-F41).
    pub no_fixed_end: Option<String>,
}

/// What *Remove a person* would do, before it is done.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RemovalPlan {
    /// The person, as the viewer sees them.
    pub person: PersonRef,
    /// Deleted in the change that removes the person.
    pub deleted_now: Vec<PlanKind>,
    /// Kept as records of the organization, shown as *Former user*.
    pub kept_as_former_user: Vec<PlanKind>,
    /// What the window offers first for the person's comments (`privacy.removed-texts`).
    pub comments: RemovedTexts,
    /// Kept for their keep period, and until when; the name stays in the legal-hold field as long
    /// as work days are kept.
    pub kept_by_law: Vec<KeptByLaw>,
}

impl Masked for RemovalPlan {}

/// The words of a kept kind whose end no day can name.
fn no_fixed_end(kind: KeepKind) -> String {
    message::removal_no_fixed_end(Message::from(kind.name()).render()).render()
}

/// One kept kind and its end.
fn kept(kind: KeepKind, rows: Option<u64>, until: KeptUntil) -> KeptByLaw {
    let (until, no_fixed_end) = match until {
        KeptUntil::Day(day) => (Some(day), None),
        KeptUntil::NoFixedEnd => (None, Some(no_fixed_end(kind))),
    };
    KeptByLaw {
        kind,
        title: Message::from(kind.name()).render(),
        rows,
        until,
        no_fixed_end,
    }
}

/// The plan of the removal of a person, with *Remove people*.
///
/// # Errors
/// `access.forbidden` without *Remove people* for the whole organization, `resource.not-found` for
/// nobody or for a person who was removed already.
pub async fn removal_plan<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: RemovalPlanQuery,
) -> Result<Authorized<RemovalPlan>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let access = ctx
        .require(&actor, Permission::RemovePeople, Place::Organization)
        .await?;
    let known = ctx.known_person_or_missing(input.person).await?;
    if known
        .standing
        .as_ref()
        .is_some_and(bb_core::person::PersonStanding::removed)
    {
        return Err(AppError::Refused(bb_core::ErrorCode::NotFound));
    }
    let failed = |error| internal::<S>(&error);
    let kinds = ctx.store().kinds_of(input.person).await.map_err(failed)?;
    let layers = ctx.setting_layers_of(None).await?;
    let today = date_of(ctx.now(), Region::of(&layers).zone());
    let mut deleted_now = Vec::new();
    let mut kept_as_former_user = Vec::new();
    let mut by_law: BTreeMap<KeepKind, u64> = BTreeMap::new();
    for kind in &kinds {
        let shown = |rows| PlanKind {
            title: title_of(&kind.marks),
            rows,
        };
        // A kind of the account goes by the rows that are the person's own; the rows that name
        // them another way stay, shown as *Former user*, and so do shared saved searches.
        let (gone, stays) = match at_removal(&kind.marks) {
            AtRemoval::Deleted { by } => {
                let mut gone = ctx
                    .store()
                    .rows_linked(input.person, &kind.kind, by)
                    .await
                    .map_err(failed)?;
                if kept_when_shared(&kind.marks) {
                    gone = gone.saturating_sub(
                        ctx.store()
                            .count_shared_searches_of(input.person)
                            .await
                            .map_err(failed)?,
                    );
                }
                (gone, kind.rows.saturating_sub(gone))
            }
            AtRemoval::KeptAsFormerUser => (0, kind.rows),
            AtRemoval::KeptByLaw(period) => {
                *by_law.entry(period).or_default() += kind.rows;
                continue;
            }
        };
        if gone > 0 && is_data_export(&kind.marks) {
            deleted_now.push(PlanKind {
                title: Message::from(message::REMOVAL_EXPORTS_GO).render(),
                rows: gone,
            });
        } else if gone > 0 {
            deleted_now.push(shown(gone));
        }
        if stays > 0 {
            kept_as_former_user.push(shown(stays));
        }
    }
    let mut kept_by_law = Vec::new();
    for (kind, rows) in by_law {
        // The newest record of the kind is kept longest: the last work day, today's entry of the
        // security log.
        let newest = if kind == KeepKind::WorkDays {
            ctx.store()
                .work_day_before(input.person, Date::MAX)
                .await
                .map_err(failed)?
                .map_or(today, |day| day.day.date)
        } else {
            today
        };
        kept_by_law.push(kept(kind, Some(rows), layers.kept_until(kind, newest)));
    }
    // Every backup holds the person until the rotation lets it go.
    kept_by_law.push(kept(
        KeepKind::Backups,
        None,
        layers.kept_until(KeepKind::Backups, today),
    ));
    // The registry only takes the words of the choice, so another value is broken data.
    let comments =
        RemovedTexts::from_code(&layers.text(&PRIVACY_REMOVED_TEXTS)).ok_or_else(|| {
            AppError::Internal("the setting privacy.removed-texts holds no choice".to_owned())
        })?;
    let person = ctx
        .person_ref(
            &access,
            PeopleKind::ProjectMembers,
            PeopleContext::Management,
            input.person,
        )
        .await?;
    Ok(access.authorize(RemovalPlan {
        person,
        deleted_now,
        kept_as_former_user,
        comments,
        kept_by_law,
    }))
}

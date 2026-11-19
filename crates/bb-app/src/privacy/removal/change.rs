//! The change of *Remove a person* (PRIV-04, ADR-0044, decisions 6 to 8): one change deletes the
//! account and everything that only serves it, through each module's own function and, for the
//! rest, by the walk's conditions; scrubs the copies of the person's values in the changes; clears
//! the name and the address, keeping the name in the legal-hold field while work days are kept;
//! makes the person a tombstone with its label; and writes the line of the replay list.
//!
//! [`remove_in`] is the change itself, so a replay after a restore runs the same steps with the
//! values of its line, with no session and no switch-off first.

use std::collections::BTreeSet;

use bb_core::ErrorCode;
use bb_core::backup::replay::ReplayKind;
use bb_core::event::{EntityRef, RecordKind};
use bb_core::id::{CommentId, IdError, IdSource, IssueId, JobId, UserId};
use bb_core::keeping::{KeepKind, KeptUntil};
use bb_core::oauth::GrantEnd;
use bb_core::person::{HoldReason, PersonStanding, Standing, cleared_address};
use bb_core::removal::{AtRemoval, at_removal, deleted_kinds, kept_when_shared};
use bb_core::settings::privacy::{PRIVACY_REMOVED_TEXTS, RemovedTexts};
use bb_core::settings::{Layers, Level, Scope, registry};
use bb_core::sign_in::SecurityAction;
use bb_core::time::day::date_of;
use bb_core::whose::{Whose, whose};
use bb_core::wording::Region;
use bb_policy::{Permission, Place, former_user_label, taken_in};
use jiff::Timestamp;
use jiff::civil::Date;

use super::Scrub;
use super::finish::{FinishRemoval, queue_finish_in};
use crate::agents::stop_agents_in;
use crate::auth::{
    AppError, AuthStore, Ctx, Stops, TakenBack, by, change_failed, end_ways_in, entity,
    fixed_entity,
};
use crate::people::state_change;
use crate::ports::KeepPeriods;
use crate::privacy::replay::{Asked, line, taken_back_in};
use crate::settings::record_of;
use crate::work_times::delete_work_times_in;

/// Remove a person.
pub struct RemovePerson {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The person.
    pub person: UserId,
    /// What happens to the person's comments; none takes what `privacy.removed-texts` offers.
    pub comments: Option<RemovedTexts>,
}

/// Everything one removal needs, so the replay runs it with the values of its line.
pub struct Removal<K> {
    /// The person.
    pub person: UserId,
    /// When.
    pub at: Timestamp,
    /// The code of the tombstone, drawn by `former_user_label`.
    pub label: String,
    /// What happens to the person's comments.
    pub comments: RemovedTexts,
    /// The keep periods of the organization.
    pub keep: K,
    /// Today, in the organization's zone: the legal hold starts no earlier.
    pub today: Date,
    /// Who asks: an administrator, whom the standing names, or the replay.
    pub asked: Asked,
    /// The ID of the job that finishes the removal (`privacy.finish-removal`), queued in its change.
    pub job: JobId,
}

/// What a removal did that the work after it needs: the job that reindexes the texts, and the
/// security log.
#[derive(Debug, Default)]
pub struct Removed {
    /// The comments it deleted.
    pub comments: Vec<CommentId>,
    /// The issues whose texts changed: their comments went.
    pub issues: Vec<IssueId>,
    /// The ways in it ended; none when it found the person removed already.
    pub(crate) taken: Option<TakenBack>,
}

/// The kinds of record whose changes are scrubbed by the person's ID: the person's record and every
/// kind whose records belong to the person (`whose`) and go with the account.
fn scrubbed_kinds() -> Vec<RecordKind> {
    RecordKind::ALL
        .iter()
        .copied()
        .filter(|kind| match whose(*kind) {
            Whose::ThePerson => true,
            Whose::Owner { of } => {
                matches!(at_removal(&of.marks), AtRemoval::Deleted { .. })
                    && !kept_when_shared(&of.marks)
            }
            Whose::Nobody => false,
        })
        .collect()
}

/// The records of the person's own settings, as their events name them (M2-F75).
fn own_settings(person: UserId) -> Result<Vec<EntityRef>, AppError> {
    registry()
        .filter(|definition| definition.levels().contains(&Level::Person))
        .map(|definition| {
            fixed_entity(
                RecordKind::Setting,
                &record_of(Scope::Person(person), definition.key()),
            )
        })
        .collect()
}

/// The legal-hold field of the tombstone: the name and its end while the person's last work day
/// is inside its keep period (ADR-0044, decision 6); none otherwise.
fn legal_hold<K: KeepPeriods>(
    removal: &Removal<K>,
    name: &str,
    last_work_day: Option<Date>,
) -> Option<(String, Option<Date>)> {
    let day = last_work_day?;
    match removal.keep.kept_until(KeepKind::WorkDays, day) {
        KeptUntil::Day(until) if until >= removal.today => Some((name.to_owned(), Some(until))),
        KeptUntil::Day(_) => None,
        KeptUntil::NoFixedEnd => Some((name.to_owned(), None)),
    }
}

/// Why a removal could not be prepared: the store failed (its error kept, so the replay of a
/// restore reports the cause), no label was free, the setting of the comments holds no choice, or
/// no ID could be made for its job.
#[derive(Debug)]
pub enum NotPrepared<E> {
    /// The store failed.
    Store(E),
    /// Every label was taken.
    NoFreeLabel,
    /// The setting `privacy.removed-texts` holds no choice.
    NoChoice,
    /// No ID could be made for the job.
    NoId(IdError),
}

impl<E> NotPrepared<E> {
    /// The error of a use case.
    fn into_app<S: AuthStore<Error = E>>(self) -> AppError {
        match self {
            Self::Store(error) => crate::auth::internal::<S>(&error),
            Self::NoFreeLabel => AppError::Internal("no free label for a former user".to_owned()),
            Self::NoChoice => {
                AppError::Internal("the setting privacy.removed-texts holds no choice".to_owned())
            }
            Self::NoId(error) => AppError::Internal(error.to_string()),
        }
    }
}

/// Prepares a removal from what is read before its change: the label, drawn so that no removed
/// person has it (the unique label refuses a draw that another removal took meanwhile), the keep
/// periods, today in the organization's zone, the comments choice (`privacy.removed-texts` when
/// none is given) and the ID of its job. The use case and the replay both start here; the
/// person's name, address and handles are read inside the change (`remove_in`).
///
/// # Errors
/// The error of the store, or no free label, or no choice in the setting ([`NotPrepared`]).
pub async fn prepare_removal<S: AuthStore>(
    store: &S,
    ids: &mut impl IdSource,
    person: UserId,
    at: Timestamp,
    comments: Option<RemovedTexts>,
    asked: Asked,
) -> Result<Removal<Layers>, NotPrepared<S::Error>> {
    let failed = NotPrepared::Store;
    let taken: BTreeSet<String> = store
        .former_labels()
        .await
        .map_err(failed)?
        .into_iter()
        .collect();
    let label = former_user_label(ids, &taken_in(&taken)).ok_or(NotPrepared::NoFreeLabel)?;
    let layers = store.setting_layers(None, None).await.map_err(failed)?;
    let comments = match comments {
        Some(choice) => choice,
        // The registry only takes the words of the choice, so another value is broken data.
        None => RemovedTexts::from_code(&layers.text(&PRIVACY_REMOVED_TEXTS))
            .ok_or(NotPrepared::NoChoice)?,
    };
    Ok(Removal {
        person,
        at,
        label,
        comments,
        today: date_of(at, Region::of(&layers).zone()),
        keep: layers,
        asked,
        job: JobId::generate(ids).map_err(NotPrepared::NoId)?,
    })
}

/// Removes a person inside a change that is open (ADR-0044, decision 6): the one function the use
/// case and the replay call.
///
/// # Errors
/// Aborts for a person who is removed already or not switched off (unless the replay asks);
/// otherwise the error of the store.
pub async fn remove_in<S: AuthStore, K: KeepPeriods>(
    transaction: &mut S::Transaction,
    removal: &Removal<K>,
) -> Result<Removed, S::Error> {
    let person = removal.person;
    let standing = S::standing_in(transaction, person).await?;
    match (removal.asked, standing.as_ref().map(|found| found.standing)) {
        (Asked::ByReplay, Some(Standing::Removed)) => return Ok(Removed::default()),
        (Asked::ByAdministrator(_), Some(Standing::SwitchedOff)) | (Asked::ByReplay, _) => {}
        (Asked::ByAdministrator(_), _) => return Err(S::aborted()),
    }
    // A replay into a backup older than the account finds nobody to remove.
    let Some(words) = S::person_words(transaction, person).await? else {
        return Ok(Removed::default());
    };
    let last_work_day = S::work_day_before_in(transaction, person, Date::MAX)
        .await?
        .map(|day| day.day.date);
    let hold = legal_hold(removal, &words.name, last_work_day);
    // The changes about the person's records are found through those records, so before they go.
    let scrub = Scrub {
        kinds: scrubbed_kinds(),
        records: own_settings(person).map_err(|_| S::aborted())?,
        names: vec![words.name.clone()],
        handles: words.handles.clone(),
        addresses: vec![words.address.clone()],
    };
    S::scrub_changes(transaction, person, &scrub).await?;
    let taken = end_ways_in::<S>(transaction, person, GrantEnd::TakenBack, removal.at).await?;
    stop_agents_in::<S>(transaction, person).await?;
    // Stopping leaves the proposals of the agents open; nobody could decide them once they go.
    S::delete_open_proposals_of_agents_of(transaction, person).await?;
    // Deleting the agents deletes their keys and says which.
    let keys = S::delete_agents_of(transaction, person).await?;
    taken_back_in::<S, _>(transaction, removal.at, ReplayKind::AccessKey, &keys).await?;
    delete_work_times_in::<S>(transaction, person).await?;
    // A change of the address that waits for its link goes with the account.
    S::drop_e_mail_change(transaction, person).await?;
    // Private searches go with their names and texts in the changes (M5-F62, M5-F63).
    let searches = S::delete_private_searches_of(transaction, person).await?;
    let searches = Scrub {
        records: searches
            .into_iter()
            .map(|search| entity(RecordKind::SavedSearch, search))
            .collect(),
        ..Scrub::default()
    };
    S::scrub_changes(transaction, person, &searches).await?;
    S::delete_two_step(transaction, person).await?;
    S::delete_grants_of(transaction, person).await?;
    // The rows that give the person roles go too, under the lock of every change of roles. Only a
    // switched-off person is removed, who counts as nobody among those who decide roles already.
    S::lock_policy(transaction).await?;
    // The files of the exports go with their records, after the change (the job).
    let exports = S::exports_of_in(transaction, person).await?;
    for (kind, links) in deleted_kinds() {
        // A shared search stays, with *Former user* as its maker (M5-F62).
        if !kept_when_shared(&kind) {
            S::delete_linked(transaction, person, &kind, links).await?;
        }
    }
    S::delete_jobs_with_address(transaction, &words.address).await?;
    let comments = match removal.comments {
        RemovedTexts::Delete => S::delete_comments_of(transaction, person).await?,
        RemovedTexts::Keep => Vec::new(),
    };
    S::clear_person(transaction, person, &cleared_address(&removal.label)).await?;
    let (hold_name, hold_until) = hold.map_or((None, None), |(name, until)| (Some(name), until));
    S::set_standing(
        transaction,
        &PersonStanding {
            person,
            standing: Standing::Removed,
            since: removal.at,
            by: removal.asked.by(),
            label: Some(removal.label.clone()),
            hold_reason: hold_name.as_ref().map(|_| HoldReason::WorkDays),
            hold_name,
            hold_until,
        },
    )
    .await?;
    S::wait_to_replay(
        transaction,
        &[line(
            removal.at,
            ReplayKind::Removal,
            person,
            Some(removal.comments.code()),
        )],
    )
    .await?;
    let mut issues: Vec<IssueId> = comments.iter().map(|(_, issue)| *issue).collect();
    issues.sort_unstable();
    issues.dedup();
    let removed = Removed {
        comments: comments.into_iter().map(|(comment, _)| comment).collect(),
        issues,
        taken: Some(taken),
    };
    let left = FinishRemoval {
        comments: removed.comments.clone(),
        issues: removed.issues.clone(),
        exports,
    };
    queue_finish_in::<S>(transaction, removal.job, &left, removal.at).await?;
    Ok(removed)
}

/// Removes a switched-off person for good, with *Remove people* (ADR-0044, decision 6).
///
/// # Errors
/// `access.forbidden` without *Remove people* for the whole organization, `resource.not-found` for
/// nobody or a person who is removed already, `person.not-switched-off` for an active person,
/// and `edit.version-changed` for one switched on meanwhile.
pub async fn remove_person<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: RemovePerson,
) -> Result<Removed, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    ctx.require(&actor, Permission::RemovePeople, Place::Organization)
        .await?;
    let person = input.person;
    let known = ctx.known_person_or_missing(person).await?;
    match known.standing() {
        Standing::Removed => return Err(AppError::Refused(ErrorCode::NotFound)),
        Standing::Active => return Err(AppError::Refused(ErrorCode::PersonNotSwitchedOff)),
        Standing::SwitchedOff => {}
    }
    let removal = prepare_removal(
        ctx.store(),
        &mut ctx.ids(),
        person,
        ctx.now(),
        input.comments,
        Asked::ByAdministrator(actor.person.id),
    )
    .await
    .map_err(NotPrepared::into_app::<S>)?;
    let event = ctx.event(
        by(actor.person.id),
        entity(RecordKind::Person, person),
        vec![state_change(Standing::SwitchedOff, Standing::Removed)],
    )?;
    let removed = ctx
        .store()
        .record_change(&event, move |transaction| {
            Box::pin(async move { remove_in::<S, _>(transaction, &removal).await })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::both(ErrorCode::VersionChanged)))?;
    ctx.note(
        SecurityAction::PersonRemoved,
        Some(actor.person.id),
        Some(entity(RecordKind::Person, person)),
    )
    .await;
    if let Some(taken) = &removed.taken {
        ctx.note_taken_back(person, taken).await;
    }
    ctx.write_replay_list().await;
    Ok(removed)
}

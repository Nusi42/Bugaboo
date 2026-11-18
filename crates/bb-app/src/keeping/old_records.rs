//! The records past their keep period that only an administrator deletes (ADR-0045, decision 6;
//! ADR-0035, decision 7): the rows of the group *by an administrator* of `KEEP_KINDS`, today work
//! days and time entries. One use case counts, for each person and in total, what of a kind is past
//! its period, when the page asks (no job counts or deletes them), and one deletes it on *Delete
//! them*, for everybody or for one person. Both need *Change organization settings*, which no access
//! key holds and no agent acts with. A deletion takes along everything that tells about the records
//! (the store's part), and is written to the security log with whose records and how many: an event
//! about each person holds the count.

use std::sync::Arc;

use bb_core::ErrorCode;
use bb_core::event::RecordKind;
use bb_core::id::UserId;
use bb_core::keeping::{KeepGroup, KeepKind, KeepRule, Length, rule};
use bb_core::people_kinds::PeopleKind;
use bb_core::sign_in::SecurityAction;
use bb_core::time::day::date_of;
use bb_core::wording::Region;
use bb_policy::{Authorized, Evaluation, Masked, PeopleContext, Permission, PersonRef, Place};
use jiff::civil::Date;
use serde_json::json;

use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, Stops, by, change_failed, entity, internal, set_to};
use crate::member;
use crate::ports::ChangeStore;
use crate::store_macros::{reads, writes};

/// The most people a count of old records names: the answer says when more have old records.
pub const OLD_PEOPLE_MAX: usize = 500;

/// How many records of one kind of one person are past the keep period.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OldCount {
    /// The person.
    pub person: UserId,
    /// How many of their records.
    pub count: u64,
}

/// What the records past their keep period need from storage. The store knows, for each kind of
/// the group *by an administrator*, which rows and which date, and what goes with a record.
pub trait OldRecordStore: ChangeStore + 'static {
    reads! {
        /// How many records of `kind` each person has with a date before `before`, by person; a
        /// person without such records is left out. Counted when asked: no job counts them.
        fn old_records(kind: KeepKind, before: Date) -> Vec<OldCount>;
    }

    writes! {
        /// Deletes the records of `kind` with a date before `before`, of one person or of
        /// everybody, with everything that tells about them (their parts, the events of their
        /// changes, the notifications about them); how many of whose were deleted.
        fn delete_old_records(kind: KeepKind, before: Date, person: Option<UserId>) -> Vec<OldCount>;
    }
}

/// How many records of a kind are past the keep period, for each person and in total.
pub struct OldRecordsQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The kind of records: one of the group *by an administrator*.
    pub kind: KeepKind,
}

/// *Delete them*: the records of a kind past the keep period, of everybody or of one person.
pub struct DeleteOldRecords {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The kind of records: one of the group *by an administrator*.
    pub kind: KeepKind,
    /// One person; `None` for everybody.
    pub person: Option<UserId>,
    /// The date the administrator's count gave (`OldRecords::before`, as `2026-11-18`): the records
    /// before it are deleted, and only while it is still the date of the period now.
    pub before: String,
}

/// The records of one person that are past the keep period.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OldOfPerson {
    /// The person, as the reader sees people in the settings of the organization.
    pub person: PersonRef,
    /// The ID of the person, for *Delete them* of one person; none when the reader does not see
    /// the person by name.
    pub id: Option<UserId>,
    /// How many of their records.
    pub count: u64,
}

/// The records of a kind past the keep period.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OldRecords {
    /// The kind of records.
    pub kind: KeepKind,
    /// The keep period in force.
    pub length: Length,
    /// The records with a date before this one are past it.
    pub before: Date,
    /// How many there are.
    pub total: u64,
    /// How many of each person, in the order of their names, at most [`OLD_PEOPLE_MAX`].
    pub people: Vec<OldOfPerson>,
    /// Whether more people have old records than the list names.
    pub truncated: bool,
}

impl Masked for OldRecords {}

/// How many records were deleted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeletedRecords {
    /// The count.
    pub deleted: u64,
}

impl Masked for DeletedRecords {}

/// What a deletion of a kind writes: the members of the event about each person (how many went,
/// and before which date) and the action of the security log. Only the kinds of the group *by an
/// administrator* have them.
struct Written {
    deleted: &'static str,
    before: &'static str,
    action: SecurityAction,
}

/// The row of `kind`, when an administrator deletes it; any other kind is not found here.
fn deleted_by_an_administrator(kind: KeepKind) -> Result<(&'static KeepRule, Written), AppError> {
    let written = match kind {
        KeepKind::WorkDays => Written {
            deleted: member::work_day::DELETED,
            before: member::work_day::BEFORE,
            action: SecurityAction::WorkDaysDeleted,
        },
        KeepKind::TimeEntries => Written {
            deleted: member::time_entry::OLD_DELETED,
            before: member::time_entry::OLD_BEFORE,
            action: SecurityAction::TimeEntriesDeleted,
        },
        KeepKind::IpAddresses
        | KeepKind::Sessions
        | KeepKind::SignUpLinks
        | KeepKind::Drafts
        | KeepKind::Notifications
        | KeepKind::Trash
        | KeepKind::Uploads
        | KeepKind::DeadJobs
        | KeepKind::AppGrants
        | KeepKind::ToolCalls
        | KeepKind::AwayTimes
        | KeepKind::SecurityLog => return Err(ErrorCode::NotFound.into()),
    };
    let row = rule(kind);
    debug_assert_eq!(row.group, KeepGroup::ByAnAdministrator);
    Ok((row, written))
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The actor, when they may delete records past their keep period.
    pub(super) async fn keeper(
        &self,
        session_token: &str,
    ) -> Result<(Actor, Arc<Evaluation>), AppError> {
        let actor = self.actor(session_token).await?;
        let evaluation = self
            .require(
                &actor,
                Permission::ChangeOrganizationSettings,
                Place::Organization,
            )
            .await?;
        Ok((actor, evaluation))
    }

    /// The keep period of `row` in force, and the first date still inside it today, in the
    /// organization's zone.
    async fn kept_from_today(&self, row: &KeepRule) -> Result<(Length, Date), AppError> {
        let layers = self.setting_layers_of(None).await?;
        let today = date_of(self.now(), Region::of(&layers).zone());
        let before = row
            .kept_from(&layers, today)
            .ok_or(AppError::Refused(ErrorCode::InvalidRequest))?;
        Ok((row.length(&layers), before))
    }
}

/// How many records of a kind of the group *by an administrator* are past the keep period, for
/// each person and in total.
///
/// # Errors
/// `access.forbidden` without *Change organization settings*, `resource.not-found` for a kind that
/// Bugaboo deletes by itself.
pub async fn old_records<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: OldRecordsQuery,
) -> Result<Authorized<OldRecords>, AppError> {
    let (_, evaluation) = ctx.keeper(&input.session_token).await?;
    let (row, _) = deleted_by_an_administrator(input.kind)?;
    let (length, before) = ctx.kept_from_today(row).await?;
    let counts = ctx
        .store()
        .old_records(input.kind, before)
        .await
        .map_err(|error| internal::<S>(&error))?;
    let ids: Vec<_> = counts.iter().map(|old| old.person).collect();
    ctx.preload_people(&ids).await?;
    let mut people = Vec::with_capacity(counts.len());
    for OldCount { person, count } in &counts {
        let shown = ctx
            .person_ref(
                &evaluation,
                PeopleKind::TimeEntries,
                PeopleContext::Management,
                *person,
            )
            .await?;
        let id = matches!(shown, PersonRef::Name { .. }).then_some(*person);
        people.push(OldOfPerson {
            person: shown,
            id,
            count: *count,
        });
    }
    // In the order of their names; people not shown by name after them.
    people.sort_by_key(|old| name_of(&old.person));
    let truncated = people.len() > OLD_PEOPLE_MAX;
    people.truncate(OLD_PEOPLE_MAX);
    Ok(evaluation.authorize(OldRecords {
        kind: input.kind,
        length,
        before,
        total: counts.iter().map(|old| old.count).sum(),
        people,
        truncated,
    }))
}

/// The name a person is sorted by: none sorts last.
fn name_of(person: &PersonRef) -> (bool, String) {
    match person {
        PersonRef::Name { name, .. } => (false, name.to_lowercase()),
        _ => (true, String::new()),
    }
}

/// *Delete them*: deletes the records of a kind of the group *by an administrator* that are past
/// the keep period, of everybody or of one person, with everything that tells about them, and
/// writes each person's deletion to the security log.
///
/// # Errors
/// `access.forbidden` without *Change organization settings*, `resource.not-found` for a kind that
/// Bugaboo deletes by itself, `request.invalid` for a `before` that is no date,
/// `keeping.past-changed` when it is not the date of the period now: the period changed, or a day
/// passed, since the count.
pub async fn delete_old_records<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: DeleteOldRecords,
) -> Result<Authorized<DeletedRecords>, AppError> {
    let (actor, evaluation) = ctx.keeper(&input.session_token).await?;
    let (row, written) = deleted_by_an_administrator(input.kind)?;
    let asked: Date = input
        .before
        .parse()
        .map_err(|_| AppError::Refused(ErrorCode::InvalidRequest))?;
    let (_, before) = ctx.kept_from_today(row).await?;
    if before != asked {
        return Err(ErrorCode::KeepPastChanged.into());
    }
    let maker = ctx.event_maker(by(actor.person.id));
    let (kind, person) = (input.kind, input.person);
    let (deleted_member, before_member) = (written.deleted, written.before);
    let deleted = ctx
        .store()
        .record_changes(&[], move |transaction| {
            Box::pin(async move {
                let deleted = S::delete_old_records(transaction, kind, before, person).await?;
                for old in &deleted {
                    let event = maker
                        .create(
                            entity(RecordKind::Person, old.person),
                            vec![
                                set_to(deleted_member, json!(old.count)),
                                set_to(before_member, json!(before.to_string())),
                            ],
                        )
                        .map_err(|_| S::aborted())?;
                    S::append_follow_up_event(transaction, &event).await?;
                }
                Ok(deleted)
            })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::NONE))?;
    for old in &deleted {
        ctx.note(
            written.action,
            Some(actor.person.id),
            Some(entity(RecordKind::Person, old.person)),
        )
        .await;
    }
    Ok(evaluation.authorize(DeletedRecords {
        deleted: deleted.iter().map(|old| old.count).sum(),
    }))
}

//! Time entries (TIME-01, TIME-02, TIME-13): the entry a timer saves and the one booked by hand
//! (*Add time*), changing and deleting one's own.
//!
//! An entry is made in one place, [`NewEntry`]: the length held to the bound of `bb_core::time`, the
//! note, *Billable* while the organization offers it, and the values of the own fields of time
//! entries, each the value asked for or else the default of the entry: the value on its issue, then
//! on its project, then on the projects above (the nearest wins), then the default of the field. A
//! required field that is still empty refuses the entry, at that field. A person makes at most
//! `TIME_ENTRIES_PER_DAY_MAX` entries on a date; that is judged inside the change.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use bb_core::ErrorCode;
use bb_core::event::{EntityKind, FieldChange, RecordKind};
use bb_core::id::{EntityId, FieldId, IssueId, TimeEntryId};
use bb_core::issue::Issue;
use bb_core::own_fields::FieldFor;
use bb_core::own_fields_value::FieldValue;
use bb_core::settings::working_hours::TIME_BILLABLE;
use bb_core::time::Minutes;
use bb_core::time::entry::TimeEntry;
use bb_core::version::{FIRST_VERSION, Version, counted_up, next};
use bb_policy::{Authorized, Evaluation, Permission, Place};
use jiff::Timestamp;
use jiff::civil::Date;
use serde_json::json;

use super::limits::{clean_note, entry_length, room_for_an_entry};
use super::types::{AddTime, ChangeTimeEntry, DeleteTimeEntry, EntryDetails, TimeEntryView};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, Done, by, entity, internal, set_to};
use crate::fields::{Item, ItemValues, Making, PlannedValues};
use crate::issues::Where;
use crate::member::time_entry as member;
use crate::projects::Why;

/// A time entry ready to be written, with the values of its own fields.
pub(super) struct NewEntry {
    /// The entry.
    pub(super) entry: TimeEntry,
    values: PlannedValues,
}

impl NewEntry {
    /// What the entry says in its event: its issue, date, length, note, *Billable*, start and the
    /// values of its own fields.
    pub(super) fn changes(&self) -> Vec<FieldChange> {
        crate::event_members::time_entry_booked(&self.entry, self.values.changes())
    }

    /// Writes the entry and its values inside the change. A person whose date holds as many
    /// entries as one date may have stops it with `time.too-many-entries`.
    ///
    /// # Errors
    /// What the store reports, or the stop of the change.
    pub(super) async fn write<S: AuthStore>(
        &self,
        transaction: &mut S::Transaction,
        why: &Why,
    ) -> Result<(), S::Error> {
        // The person's time is held first, so two bookings at once count one after the other.
        S::hold_time_of(transaction, self.entry.person).await?;
        let booked = S::entries_on_date_in(transaction, self.entry.person, self.entry.date).await?;
        if room_for_an_entry(booked).is_err() {
            return Err(why.stop::<S>(ErrorCode::TooManyTimeEntries));
        }
        S::insert_time_entry(transaction, &self.entry).await?;
        write_values::<S>(transaction, self.entry.id, &self.values).await
    }
}

/// Writes the values of the own fields of an entry that differ.
async fn write_values<S: AuthStore>(
    transaction: &mut S::Transaction,
    entry: TimeEntryId,
    values: &PlannedValues,
) -> Result<(), S::Error> {
    let kind = EntityKind::from(RecordKind::TimeEntry);
    for (field, cells) in &values.writes() {
        S::put_field_value(transaction, kind.clone(), entry.into(), *field, cells).await?;
    }
    Ok(())
}

/// What a new entry is made of besides its issue: when it was, how long, and since when the timer
/// ran (none for *Add time*).
pub(super) struct EntryTime {
    /// The date.
    pub(super) date: Date,
    /// The length.
    pub(super) minutes: Minutes,
    /// The start of the timer it is saved from.
    pub(super) started_at: Option<Timestamp>,
}

impl<S: AuthStore> Ctx<'_, S> {
    /// An issue the actor sees and may book time on: `resource.not-found` for one they do not see,
    /// `access.forbidden` without *Book time* there.
    pub(super) async fn bookable_issue(
        &self,
        actor: &Actor,
        address: &str,
    ) -> Result<(Issue, Arc<Evaluation>), AppError> {
        let (issue, held) = self.visible_issue(actor, address).await?;
        if !held.allows(Permission::BookTime) {
            return Err(ErrorCode::Forbidden.into());
        }
        Ok((issue, held))
    }

    /// Whether the organization offers *Billable*.
    pub(super) async fn billable_offered(&self) -> Result<bool, AppError> {
        Ok(self.setting_layers_of(None).await?.yes(&TIME_BILLABLE))
    }

    /// The item the own fields of an entry on `issue` are judged for.
    pub(super) async fn entry_item(
        &self,
        issue: &Issue,
        id: Option<TimeEntryId>,
    ) -> Result<Item, AppError> {
        Ok(Item {
            kind: RecordKind::TimeEntry,
            id: id.map(EntityId::from),
            chain: self.chain_of(issue.project).await?,
            issue_type: Some(issue.issue_type),
            reported_by: Some(issue.reported_by),
        })
    }

    /// The defaults of the own fields of time entries on an issue: for each field, the value on the
    /// issue, then on its project, then on the projects above; the nearest wins. Read once, for
    /// every field together.
    pub(super) async fn entry_defaults(
        &self,
        issue: &Issue,
        chain: &[bb_core::id::ProjectId],
    ) -> Result<BTreeMap<FieldId, FieldValue>, AppError> {
        let on_issue = self
            .load_values(RecordKind::Issue, &[issue.id.into()])
            .await?;
        self.entry_defaults_from(issue, chain, &on_issue).await
    }

    /// [`Self::entry_defaults`] with the values of the issue already read, for a caller that needs
    /// them for something else too.
    pub(super) async fn entry_defaults_from(
        &self,
        issue: &Issue,
        chain: &[bb_core::id::ProjectId],
        on_issue: &ItemValues,
    ) -> Result<BTreeMap<FieldId, FieldValue>, AppError> {
        let fields: Vec<FieldId> = self
            .field_catalogue()
            .await?
            .iter()
            .filter(|definition| {
                definition.field.applies_to == FieldFor::TimeEntries
                    && definition.field.retired_at.is_none()
            })
            .map(|definition| definition.field.id)
            .collect();
        if fields.is_empty() {
            return Ok(BTreeMap::new());
        }
        let projects: Vec<EntityId> = chain.iter().map(|project| (*project).into()).collect();
        let on_projects = self.load_values(RecordKind::Project, &projects).await?;
        Ok(fields
            .into_iter()
            .filter_map(|field| {
                on_issue
                    .value(issue.id.into(), field)
                    .or_else(|| {
                        nearest_on_projects(&on_projects, &projects, field).map(|(_, value)| value)
                    })
                    .map(|value| (field, value.clone()))
            })
            .collect())
    }

    /// A new entry of the actor on an issue they may book on (`held` is their evaluation there).
    ///
    /// # Errors
    /// `time.entry-too-short` and `time.entry-too-long` for the length, `time.note-too-long` and
    /// `request.invalid` for the note, the refusals of own fields (a required field without a
    /// value or a default is `field.required` at that field).
    pub(super) async fn new_entry(
        &self,
        actor: &Actor,
        (issue, held): (&Issue, &Evaluation),
        time: EntryTime,
        details: &EntryDetails,
    ) -> Result<NewEntry, AppError> {
        let minutes = entry_length(time.minutes)?;
        let note = clean_note(details.note.as_deref().unwrap_or_default())?;
        let billable = self.billable_offered().await? && details.billable.unwrap_or(false);
        let id = TimeEntryId::generate(&mut self.ids())
            .map_err(|error| AppError::Internal(error.to_string()))?;
        let item = self.entry_item(issue, None).await?;
        let inherited = self.entry_defaults(issue, &item.chain).await?;
        let nothing_yet = self.load_values(RecordKind::TimeEntry, &[]).await?;
        let values = self
            .plan_values(
                actor,
                held,
                &item,
                &details.fields,
                &nothing_yet,
                Making::New(&inherited),
            )
            .await?;
        Ok(NewEntry {
            entry: TimeEntry {
                id,
                person: actor.person.id,
                issue: issue.id,
                date: time.date,
                minutes,
                note,
                billable,
                started_at: time.started_at,
                created_at: self.now(),
                version: FIRST_VERSION,
            },
            values,
        })
    }

    /// One of the actor's own entries with its issue and what they may do there: an entry of
    /// somebody else, or one that is not there, answers `resource.not-found`, like an entry on an
    /// issue they no longer see.
    async fn own_entry(
        &self,
        actor: &Actor,
        id: TimeEntryId,
    ) -> Result<(TimeEntry, Issue, Arc<Evaluation>), AppError> {
        // Time is a person's own: an agent never books or changes it (M7-Q7, ADR-0035, decision 9).
        actor.ensure_not_an_agent()?;
        let entry = self
            .store()
            .time_entry(id)
            .await
            .map_err(|error| internal::<S>(&error))?
            .filter(|entry| entry.person == actor.person.id)
            .ok_or(AppError::Refused(ErrorCode::NotFound))?;
        let (issue, held) = self.bookable_issue(actor, &entry.issue.to_string()).await?;
        Ok((entry, issue, held))
    }

    /// Entries as their own person sees them, read together for a page: the issues, the values of
    /// own fields and the setting of *Billable* once, the rights and the projects once for each
    /// project (PERF-N1). Each entry is masked with the rights where its issue is, so a field a
    /// person may see through a role in that project is in their own entry. The issue of an entry
    /// the person no longer sees keeps the entry in their list, without its readable ID and with
    /// the own fields the organization lets them see; an access key is told nothing of an entry
    /// whose issue it does not see (AUTH-07).
    pub(super) async fn entry_views(
        &self,
        actor: &Actor,
        entries: &[TimeEntry],
    ) -> Result<Vec<Authorized<TimeEntryView>>, AppError> {
        let mut issue_ids: Vec<IssueId> = entries.iter().map(|entry| entry.issue).collect();
        issue_ids.sort_unstable();
        issue_ids.dedup();
        let issues: HashMap<IssueId, Issue> = self
            .store()
            .issues_by_ids(&issue_ids)
            .await
            .map_err(|error| internal::<S>(&error))?
            .into_iter()
            .map(|issue| (issue.id, issue))
            .collect();
        let ids: Vec<EntityId> = entries.iter().map(|entry| entry.id.into()).collect();
        let values = self.load_values(RecordKind::TimeEntry, &ids).await?;
        let billable = self.billable_offered().await?;
        let organization = self.evaluation_of(actor, Place::Organization).await?;
        let mut maker = self.line_maker().await?;
        let mut views = Vec::with_capacity(entries.len());
        for entry in entries {
            let issue = issues.get(&entry.issue);
            let (line, held) = match issue {
                Some(issue) => (
                    self.line_of(actor, &mut maker, issue).await?,
                    self.seen_in(actor, issue, Where::Live).await?,
                ),
                None => (None, None),
            };
            let held = match held {
                Some(held) => held,
                None if actor.key.is_some() => continue,
                None => Arc::clone(&organization),
            };
            let shown = values.shown(&held, issue.map(|issue| issue.issue_type), entry.id.into());
            let own_field_names = values.names(&shown);
            let (readable_id, issue_title) = match line {
                Some(line) => (Some(line.key), line.title),
                None => (None, None),
            };
            let view = TimeEntryView {
                id: entry.id,
                issue: entry.issue,
                readable_id,
                issue_title,
                date: entry.date,
                minutes: entry.minutes,
                note: entry.note.clone(),
                billable: billable.then_some(entry.billable),
                started_at: entry.started_at,
                own_fields: shown,
                own_field_names,
                version: entry.version,
            };
            views.push(held.authorize(view));
        }
        Ok(views)
    }

    /// One entry as its own person sees it.
    pub(super) async fn entry_view(
        &self,
        actor: &Actor,
        entry: &TimeEntry,
    ) -> Result<Authorized<TimeEntryView>, AppError> {
        self.entry_views(actor, std::slice::from_ref(entry))
            .await?
            .pop()
            .ok_or_else(|| AppError::Internal("an entry without its view".into()))
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// What booking the time would record, answered without saving: the entry [`add_time`] would
    /// write, judged the same way (AI-21).
    ///
    /// # Errors
    /// As [`add_time`], but for a full date, which only the write sees.
    pub(crate) async fn preview_add_time(
        &self,
        actor: &Actor,
        input: &AddTime,
    ) -> Result<Vec<FieldChange>, AppError> {
        let (issue, held) = self.bookable_issue(actor, &input.issue).await?;
        let made = self
            .new_entry(
                actor,
                (&issue, &held),
                EntryTime {
                    date: input.date,
                    minutes: input.minutes,
                    started_at: None,
                },
                &input.details,
            )
            .await?;
        Ok(made.changes())
    }
}

/// Books time on an issue by hand (*Add time*): a date and a length, with the defaults of the issue
/// for the own fields that are left out.
///
/// # Errors
/// `resource.not-found` for an issue the person does not see, `access.forbidden` without *Book
/// time*, the refusals of [`Ctx::new_entry`], and `time.too-many-entries` for a full date.
pub async fn add_time<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: AddTime,
) -> Result<Authorized<TimeEntryView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    actor.ensure_not_an_agent()?;
    let (issue, held) = ctx.bookable_issue(&actor, &input.issue).await?;
    let made = ctx
        .new_entry(
            &actor,
            (&issue, &held),
            EntryTime {
                date: input.date,
                minutes: input.minutes,
                started_at: None,
            },
            &input.details,
        )
        .await?;
    let event = ctx.event(
        by(actor.person.id),
        entity(RecordKind::TimeEntry, made.entry.id),
        made.changes(),
    )?;
    let why = Why::default();
    let stopped = why.clone();
    let entry = made.entry.clone();
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move { made.write::<S>(transaction, &stopped).await })
        })
        .await
        .map_err(|error| why.said_or_failed::<S>(&error))?;
    ctx.entry_view(&actor, &entry).await
}

/// Changes one of the person's own entries: its date, length, note, *Billable* or the values of
/// its own fields. What is left out stays.
///
/// # Errors
/// `resource.not-found` for an entry that is not the person's, `access.forbidden` without *Book
/// time*, `edit.version-changed` for an entry that changed since it was read, the refusals of the
/// length, the note and the own fields, and `time.too-many-entries` for a new date that is full.
pub async fn change_time_entry<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ChangeTimeEntry,
) -> Result<Authorized<TimeEntryView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let (entry, issue, held) = ctx.own_entry(&actor, input.entry).await?;
    if entry.version != input.version {
        return Err(entry_stale(&actor, entry.id, issue.project));
    }
    let details = &input.details;
    let note = match &details.note {
        Some(note) => clean_note(note)?,
        None => entry.note.clone(),
    };
    let billable = if ctx.billable_offered().await? {
        details.billable.unwrap_or(entry.billable)
    } else {
        entry.billable
    };
    let changed = TimeEntry {
        date: input.date.unwrap_or(entry.date),
        minutes: entry_length(input.minutes.unwrap_or(entry.minutes))?,
        note,
        billable,
        version: next(entry.version),
        ..entry.clone()
    };
    let item = ctx.entry_item(&issue, Some(entry.id)).await?;
    let stored = ctx
        .load_values(RecordKind::TimeEntry, &[entry.id.into()])
        .await?;
    let values = ctx
        .plan_values(
            &actor,
            &held,
            &item,
            &details.fields,
            &stored,
            Making::Change,
        )
        .await?;
    // Every event of an entry says the date of the entry, so the history can show readers who may not
    // see the moment the date and never the moment (PRIV-09).
    let mut members = vec![set_to(member::ISSUE, json!(entry.issue.to_string()))];
    if changed.date == entry.date {
        members.push(set_to(member::DATE, json!(entry.date.to_string())));
    }
    let mut differs = |name: &str, before: serde_json::Value, after: serde_json::Value| {
        if before != after {
            members.push(FieldChange {
                field: name.to_owned(),
                before: Some(before),
                after: Some(after),
            });
        }
    };
    differs(
        member::DATE,
        json!(entry.date.to_string()),
        json!(changed.date.to_string()),
    );
    differs(
        member::MINUTES,
        json!(entry.minutes),
        json!(changed.minutes),
    );
    differs(member::NOTE, json!(entry.note), json!(changed.note));
    differs(
        member::BILLABLE,
        json!(entry.billable),
        json!(changed.billable),
    );
    members.extend(values.changes());
    members.push(counted_up(entry.version));
    let event = ctx.event(
        by(actor.person.id),
        entity(RecordKind::TimeEntry, entry.id),
        members,
    )?;
    let why = Why::default();
    let stopped = why.clone();
    let (written, moved) = (changed.clone(), changed.date != entry.date);
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                if moved {
                    S::hold_time_of(transaction, written.person).await?;
                    let booked =
                        S::entries_on_date_in(transaction, written.person, written.date).await?;
                    if room_for_an_entry(booked).is_err() {
                        return Err(stopped.stop::<S>(ErrorCode::TooManyTimeEntries));
                    }
                }
                if !S::update_time_entry(transaction, &written, Version::new(entry.version)).await?
                {
                    return Err(stopped.stop::<S>(ErrorCode::VersionChanged));
                }
                write_values::<S>(transaction, written.id, &values).await
            })
        })
        .await
        .map_err(|error| {
            failed_change::<S>(&why, &error, || {
                entry_stale(&actor, entry.id, issue.project)
            })
        })?;
    ctx.entry_view(&actor, &changed).await
}

/// Deletes one of the person's own entries, with the values of its own fields.
///
/// # Errors
/// `resource.not-found` for an entry that is not the person's, `access.forbidden` without *Book
/// time*, `edit.version-changed` for an entry that changed since it was read.
pub async fn delete_time_entry<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: DeleteTimeEntry,
) -> Result<Done, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let (entry, issue, _) = ctx.own_entry(&actor, input.entry).await?;
    if entry.version != input.version {
        return Err(entry_stale(&actor, entry.id, issue.project));
    }
    let event = ctx.event(
        by(actor.person.id),
        entity(RecordKind::TimeEntry, entry.id),
        vec![
            set_to(member::ISSUE, json!(entry.issue.to_string())),
            set_to(member::DATE, json!(entry.date.to_string())),
            FieldChange {
                field: member::MINUTES.to_owned(),
                before: Some(json!(entry.minutes)),
                after: None,
            },
            counted_up(entry.version),
        ],
    )?;
    let why = Why::default();
    let stopped = why.clone();
    let (id, version) = (entry.id, Version::new(entry.version));
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                if S::delete_time_entry(transaction, id, version).await? {
                    Ok(())
                } else {
                    Err(stopped.stop::<S>(ErrorCode::VersionChanged))
                }
            })
        })
        .await
        .map_err(|error| {
            failed_change::<S>(&why, &error, || entry_stale(&actor, id, issue.project))
        })?;
    Ok(Done)
}

/// The stale version of the actor's timer: the conflict names the version it has now.
pub(super) fn timer_stale(actor: &Actor) -> AppError {
    AppError::stale(
        entity(RecordKind::Timer, actor.person.id),
        Place::Organization,
        actor.person.id,
    )
}

/// The stale version of one of the actor's entries, on an issue of `project`.
fn entry_stale(actor: &Actor, entry: TimeEntryId, project: bb_core::id::ProjectId) -> AppError {
    AppError::stale(
        entity(RecordKind::TimeEntry, entry),
        Place::Project(project),
        actor.person.id,
    )
}

/// What a change of time that failed answers: a stale version as the conflict of its record
/// (`stale`), anything else as the reason the change gave or the failure of the store.
pub(super) fn failed_change<S: AuthStore>(
    why: &Why,
    error: &S::Error,
    stale: impl FnOnce() -> AppError,
) -> AppError {
    if why.said() == Some(ErrorCode::VersionChanged) {
        stale()
    } else {
        why.said_or_failed::<S>(error)
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// An answer about the person's own time, with the organization's evaluation.
    pub(super) async fn own_answer<T>(
        &self,
        actor: &Actor,
        value: T,
    ) -> Result<Authorized<T>, AppError> {
        Ok(self
            .evaluation_of(actor, Place::Organization)
            .await?
            .authorize(value))
    }
}

/// The default a field has from the projects: the value of the nearest project that holds one, and
/// that project. `projects` run from the top down, so the last one is the nearest. This is the one
/// rule of which project a default comes from: the defaults of a new entry and the page of a
/// project that says where its inherited defaults come from both ask it.
pub(crate) fn nearest_on_projects<'v>(
    values: &'v ItemValues,
    projects: &[EntityId],
    field: FieldId,
) -> Option<(EntityId, &'v FieldValue)> {
    projects
        .iter()
        .rev()
        .find_map(|project| values.value(*project, field).map(|value| (*project, value)))
}

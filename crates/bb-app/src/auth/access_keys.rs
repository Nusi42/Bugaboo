//! Access keys (AUTH-07, SEC-14, SEC-15, ADR-0025 decision 8): keys for scripts and apps that can
//! only do less than their owner, and the use cases that make, list and delete them.
//!
//! The rules that hold in every use case here:
//! - a key is `bb_` + a public prefix + a 256-bit secret; only a hash of the secret is stored, and
//!   the secret is returned once, with the answer that makes the key;
//! - every key runs out, after the organization's default number of days unless the person picks
//!   another date, never later than the organization's longest time;
//! - a key has no permissions of its own: at every request it is its owner's evaluation narrowed
//!   by its choice and its projects (`Ctx::evaluation_of`), so it follows its owner losing a role
//!   at once and never holds more than the owner;
//! - an expired, a deleted, an unknown and a malformed key get the same answer, and nothing says
//!   which; guessing is limited per network by the failures of the network;
//! - no key reaches the owner's own sign-in data (password, passkeys, second step, sessions, access
//!   keys): those use cases start at a session, which a key is not;
//! - making and deleting a key, and a key turned away, go into the security log (the network
//!   shortened, never the secret and never the prefix of a key that is not known).

use std::collections::BTreeSet;

use bb_auth::{AccessKeySecret, same_digest};
use bb_core::ErrorCode;
use bb_core::access_key::{AccessKey, KeyChoice, MOST_KEYS, MOST_PROJECTS};
use bb_core::event::{DomainEvent, RecordKind};
use bb_core::id::{AccessKeyId, ProjectId, UserId};
use bb_core::paging::{Cut, page_len, read_len, read_time_cursor, write_time_cursor};
use bb_core::sign_in::{SecurityAction, SessionStage};
use bb_core::time::SECONDS_PER_DAY;
use bb_policy::{Authorized, Choice, Narrowing, Permission, Place};
use jiff::Timestamp;
use serde_json::json;

use super::ctx::Ctx;
use super::error::{AppError, Stops, change_failed, internal};
use super::events::{by, entity, set_to};
use super::store::{AccessKeyRecord, AuthStore};
use super::types::{
    AccessKeyPage, AccessKeyRules, AccessKeyRulesQuery, AccessKeyView, AccessKeysQuery,
    CreateAccessKey, CreatedAccessKey, DeleteAccessKey, Done, ProjectChoicePage,
    ProjectChoicesQuery, RequireSessionQuery, SeeServerStatusQuery,
};
use crate::access::{Actor, KeyInUse};
use crate::input::{clean_name, cursor_or_invalid};

/// The purpose under which guessing keys is counted per network (`Ctx::reserve_network`).
const GUESSING: &str = "access-key";

/// What a key narrows its owner to: its choice and its projects.
pub(crate) fn narrowing_of(record: &AccessKeyRecord) -> Narrowing {
    let narrowing = match record.key.choice {
        KeyChoice::SeeOnly => Narrowing::see_only(),
        KeyChoice::SeeAndChange => Narrowing::see_and_change(),
        KeyChoice::Everything => Narrowing::everything(),
        KeyChoice::Permissions => Narrowing::new(Choice::Permissions(
            record.permissions.iter().copied().collect::<BTreeSet<_>>(),
        )),
    };
    if record.projects.is_empty() {
        narrowing
    } else {
        narrowing.in_projects(record.projects.iter().copied().collect())
    }
}

/// The moment cut to the minute: the last use of a key is kept to the minute (PRIV-02: no more
/// exact than a purpose needs), and a busy key writes at most once a minute.
fn to_the_minute(moment: Timestamp) -> Timestamp {
    let seconds = moment.as_second();
    Timestamp::from_second(seconds - seconds.rem_euclid(60)).unwrap_or(moment)
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The key and its owner behind this text, if it is a key that works now.
    async fn working_key(
        &self,
        key_text: &str,
    ) -> Result<Option<(AccessKeyRecord, bb_core::person::Person)>, AppError> {
        let Some(secret) = AccessKeySecret::parse(key_text) else {
            return Ok(None);
        };
        let hash = secret.hash();
        let Some(record) = self
            .store()
            .access_key_by_hash(&hash)
            .await
            .map_err(|error| internal::<S>(&error))?
        else {
            return Ok(None);
        };
        // The lookup is by hash; this comparison is for the record that came back. The prefix of
        // the key as it was presented has to be the one that is stored, so the text of a key is
        // only ever one text.
        if !same_digest(&record.key.secret_hash, &hash)
            || record.key.prefix != secret.prefix()
            || record.key.expires_at <= self.now()
        {
            return Ok(None);
        }
        let Some(owner) = self
            .store()
            .person(record.key.owner)
            .await
            .map_err(|error| internal::<S>(&error))?
        else {
            return Ok(None);
        };
        Ok(Some((record, owner)))
    }

    /// The owner of the access key a script came with, narrowed by the key at every use.
    ///
    /// The key is looked at first, and a key that works never touches the limit on guessing: only
    /// a lookup that fails is a try of the network (SEC-14). So keys used at the same time, or by
    /// somebody on a network where others have been guessing, are never made to wait; a guess is
    /// the only thing a network over its limit is answered with a wait.
    ///
    /// # Errors
    /// `auth.not_signed_in` for a key that does not work, whatever the reason (expired,
    /// deleted, unknown, malformed); 429 when that is a guess of a network over its limit;
    /// `auth.second_step_required` while the owner has to set up a second step.
    pub(crate) async fn key_actor(&self, key_text: &str) -> Result<Actor, AppError> {
        let Some((record, owner)) = self.working_key(key_text).await? else {
            let slot = self.reserve_network(GUESSING)?;
            self.slot_failed(slot, None).await;
            // Who sent it is not known: the entry holds the shortened network only.
            self.note(SecurityAction::AccessKeyRefused, None, None)
                .await;
            return Err(AppError::Refused(ErrorCode::NotSignedIn));
        };
        // A key follows its owner's two-step requirement: a person who has to set up a second
        // step cannot get around it with a key (ADR-0025, decision 8; M2-F46).
        if self.must_set_up_second_step(owner.id).await? {
            let subject = Some(entity(RecordKind::AccessKey, record.key.id));
            self.note(SecurityAction::AccessKeyRefused, Some(owner.id), subject)
                .await;
            return Err(AppError::Refused(ErrorCode::SecondStepRequired));
        }
        self.via_key
            .store(true, std::sync::atomic::Ordering::Relaxed);
        // A busy key does not write at all within the minute it was last used in.
        let minute = to_the_minute(self.now());
        if record.key.last_used_at != Some(minute) {
            self.store()
                .touch_access_key(record.key.id, minute)
                .await
                .map_err(|error| internal::<S>(&error))?;
        }
        Ok(Actor {
            person: owner,
            key: Some(KeyInUse {
                id: record.key.id,
                narrowing: narrowing_of(&record),
            }),
        })
    }

    /// The answer for an access key that was used where only a session goes: the owner's own
    /// sign-in data. A key that works is written down with its owner; one that does not gets the
    /// answer of any key that does not work.
    pub(crate) async fn key_on_sign_in_data(&self, key_text: &str) -> AppError {
        match self.key_actor(key_text).await {
            Ok(actor) => {
                let subject = actor
                    .key
                    .as_ref()
                    .map(|key| entity(RecordKind::AccessKey, key.id));
                self.note(
                    SecurityAction::AccessKeyRefused,
                    Some(actor.person.id),
                    subject,
                )
                .await;
                AppError::Refused(ErrorCode::AccessKeyNotAllowed)
            }
            Err(error) => error,
        }
    }

    /// The key as its owner sees it in the list.
    async fn key_view(
        &self,
        actor: &Actor,
        record: &AccessKeyRecord,
    ) -> Result<AccessKeyView, AppError> {
        let mut projects = Vec::new();
        for project in &record.projects {
            projects.push(self.project_label(actor, *project).await?);
        }
        Ok(AccessKeyView {
            id: record.key.id,
            name: record.key.name.clone(),
            prefix: record.key.prefix.clone(),
            choice: record.key.choice,
            permissions: record.permissions.clone(),
            projects,
            created_at: record.key.created_at,
            expires_at: record.key.expires_at,
            last_used_at: record.key.last_used_at,
            expired: record.key.expires_at <= self.now(),
        })
    }
}

/// When a key made now stops working: the date the person picked, or the default number of days;
/// in the future and at most the organization's longest time (`request.invalid` otherwise).
fn expiry(
    now: Timestamp,
    picked: Option<Timestamp>,
    default_days: i64,
    longest_days: i64,
) -> Result<Timestamp, AppError> {
    // A default longer than the longest time is cut to it: the longest time is the rule.
    let default_days = default_days.min(longest_days);
    let latest = now
        .as_second()
        .saturating_add(longest_days.saturating_mul(SECONDS_PER_DAY));
    let wanted = picked.map_or_else(
        || {
            now.as_second()
                .saturating_add(default_days.saturating_mul(SECONDS_PER_DAY))
        },
        Timestamp::as_second,
    );
    if wanted <= now.as_second() || wanted > latest {
        return Err(ErrorCode::InvalidRequest.into());
    }
    Timestamp::from_second(wanted).map_err(|_| ErrorCode::InvalidRequest.into())
}

/// Checks *What it may do* and its permissions: single permissions only with the choice *More
/// options*, and at least one then.
fn clean_choice(
    choice: KeyChoice,
    permissions: &[Permission],
) -> Result<Vec<Permission>, AppError> {
    let wanted: BTreeSet<Permission> = permissions.iter().copied().collect();
    // Single permissions come with the choice *More options* and only with it, and never one a
    // key does not hold (inviting people, changing roles: `NOT_FOR_KEYS`).
    if (choice == KeyChoice::Permissions) == wanted.is_empty()
        || !wanted.iter().all(|permission| permission.keys_may_hold())
    {
        return Err(ErrorCode::InvalidRequest.into());
    }
    Ok(wanted.into_iter().collect())
}

/// A try at making the key: a new secret, the key made from `draft` with it, and the event that
/// names what the key may do and when it runs out; never the secret or its hash. The public
/// prefix is 8 symbols and unique, so two keys can (very rarely) draw the same one: the store
/// refuses the second, and the key is made once more with a new secret.
fn a_try_at_the_key<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    draft: &AccessKeyRecord,
) -> Result<(AccessKeySecret, AccessKeyRecord, DomainEvent), AppError> {
    let secret = AccessKeySecret::generate()?;
    let mut stored = draft.clone();
    secret.prefix().clone_into(&mut stored.key.prefix);
    stored.key.secret_hash = secret.hash();
    let event = ctx.event(
        by(stored.key.owner),
        entity(RecordKind::AccessKey, stored.key.id),
        vec![
            set_to("name", json!(stored.key.name)),
            set_to("choice", json!(stored.key.choice.code())),
            set_to("expires_at", json!(stored.key.expires_at.to_string())),
        ],
    )?;
    Ok((secret, stored, event))
}

/// Creates an access key for the person. The key is returned once; only a hash of its secret is
/// kept. The password is asked again, so a stolen session cannot make a key that outlives it.
///
/// # Errors
/// `auth.wrong_credentials` for a wrong password, `request.invalid` for a name, a date, a choice
/// or a project that cannot be used (or too many keys), `resource.not_found` for a project the
/// person may not know about, `auth.access_key_not_allowed` for a key.
pub async fn create_access_key<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: CreateAccessKey,
) -> Result<Authorized<CreatedAccessKey>, AppError> {
    let record = ctx.active(&input.session_token).await?;
    ctx.confirm_password(&record.person, input.password).await?;
    let owner = record.person.id;
    let name = clean_name(&input.name)?;
    let permissions = clean_choice(input.choice, &input.permissions)?;
    let projects: Vec<ProjectId> = input
        .projects
        .iter()
        .copied()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    if projects.len() > MOST_PROJECTS {
        return Err(ErrorCode::InvalidRequest.into());
    }
    // A project the person may not know about is not one they can name, whether it exists or not.
    let actor = Actor {
        person: record.person.clone(),
        key: None,
    };
    for project in &projects {
        let access = ctx.evaluation_of(&actor, Place::Project(*project)).await?;
        if !access.knows_project() {
            return Err(AppError::Refused(ErrorCode::NotFound));
        }
    }
    let settings = ctx.settings().await?;
    let now = ctx.now();
    let picked = input
        .expires_at
        .as_deref()
        .map(str::parse::<Timestamp>)
        .transpose()
        .map_err(|_| AppError::Refused(ErrorCode::InvalidRequest))?;
    let expires_at = expiry(
        now,
        picked,
        settings.access_key_default_days,
        settings.access_key_longest_days,
    )?;
    let held = ctx
        .store()
        .access_key_count(owner)
        .await
        .map_err(|error| internal::<S>(&error))?;
    if usize::try_from(held).map_or(true, |held| held >= MOST_KEYS) {
        return Err(ErrorCode::InvalidRequest.into());
    }
    let draft = AccessKeyRecord {
        key: AccessKey {
            id: AccessKeyId::generate(&mut ctx.ids()),
            owner,
            name,
            prefix: String::new(),
            secret_hash: bb_core::security_log::Digest::new([0; 32]),
            choice: input.choice,
            created_at: now,
            expires_at,
            last_used_at: None,
        },
        permissions,
        projects,
    };
    let mut clashed = false;
    let (secret, stored) = loop {
        let (secret, stored, event) = a_try_at_the_key(ctx, &draft)?;
        let to_store = stored.clone();
        let result = ctx
            .store()
            .record_change(&event, move |transaction| {
                Box::pin(async move { S::insert_access_key(transaction, &to_store).await })
            })
            .await;
        match result {
            Ok(()) => break (secret, stored),
            Err(error) if S::is_duplicate(&error) && !clashed => clashed = true,
            Err(error) => return Err(change_failed::<S>(&error, Stops::NONE)),
        }
    };
    ctx.note(
        SecurityAction::AccessKeyCreated,
        Some(owner),
        Some(entity(RecordKind::AccessKey, stored.key.id)),
    )
    .await;
    let access = ctx.evaluate(owner, Place::Organization).await?;
    let key = ctx.key_view(&actor, &stored).await?;
    Ok(access.authorize(CreatedAccessKey {
        key,
        secret: secret.encode().as_str().to_owned(),
    }))
}

/// Deletes an access key of the person at once. A deleted key cannot come back.
///
/// # Errors
/// `resource.not_found` for a key that is not the person's (it does not say whether it exists).
pub async fn delete_access_key<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: DeleteAccessKey,
) -> Result<Done, AppError> {
    let record = ctx.active(&input.session_token).await?;
    let (owner, id) = (record.person.id, input.id);
    let event = ctx.event(
        by(owner),
        entity(RecordKind::AccessKey, id),
        vec![super::events::changed("access_key")],
    )?;
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                if S::delete_access_key(transaction, owner, id).await? {
                    Ok(())
                } else {
                    Err(S::aborted())
                }
            })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::both(ErrorCode::NotFound)))?;
    ctx.note(
        SecurityAction::AccessKeyDeleted,
        Some(owner),
        Some(entity(RecordKind::AccessKey, id)),
    )
    .await;
    Ok(Done)
}

/// The access keys of the person, one page at a time (newest first).
///
/// # Errors
/// `request.invalid` for a cursor that is not one.
pub async fn access_keys<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: AccessKeysQuery,
) -> Result<Authorized<AccessKeyPage>, AppError> {
    let record = ctx.active(&input.session_token).await?;
    let owner: UserId = record.person.id;
    let len = page_len(Some(input.limit));
    let after = cursor_or_invalid(input.cursor.as_deref(), read_time_cursor)?;
    let page = Cut::of(
        ctx.store()
            .access_keys_of(owner, after, read_len(len))
            .await
            .map_err(|error| internal::<S>(&error))?,
        len,
    );
    let next_cursor = page.next_cursor(|last| write_time_cursor(last.key.created_at, last.key.id));
    let found = page.items;
    let actor = Actor {
        person: record.person,
        key: None,
    };
    let mut items = Vec::new();
    for key in &found {
        items.push(ctx.key_view(&actor, key).await?);
    }
    let access = ctx.evaluate(owner, Place::Organization).await?;
    Ok(access.authorize(AccessKeyPage { items, next_cursor }))
}

/// How long a new access key lives in this organization: the default number of days (never more
/// than the longest time) and the longest time.
///
/// # Errors
/// `auth.not_signed_in` without a session.
pub async fn access_key_rules<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: AccessKeyRulesQuery,
) -> Result<Authorized<AccessKeyRules>, AppError> {
    let record = ctx.active(&input.session_token).await?;
    let settings = ctx.settings().await?;
    let access = ctx.evaluate(record.person.id, Place::Organization).await?;
    Ok(access.authorize(AccessKeyRules {
        default_days: settings
            .access_key_default_days
            .min(settings.access_key_longest_days),
        longest_days: settings.access_key_longest_days,
        // What a key never holds is not offered.
        permissions: Permission::ALL
            .iter()
            .filter(|permission| permission.keys_may_hold())
            .map(|permission| (*permission, permission.topic()))
            .collect(),
    }))
}

/// How many projects are looked at to fill one page of the projects to choose from: a person who
/// sees few of many projects gets a short page and a cursor, not a slow answer.
const SCANNED_AT_MOST: u64 = 500;

/// How many projects the store is asked for at a time while a page is filled (the most a page of
/// any list holds).
const READ_AT_A_TIME: u64 = bb_core::paging::PAGE_SIZE_MAX as u64;

/// Where a page of the projects to choose from starts: after a short name (a project the person
/// was shown, or the start) and past a number of projects, always a multiple of
/// [`SCANNED_AT_MOST`].
///
/// A cursor is readable, so it holds nothing a person may not know about: the short name is the
/// last project of the page before (which the person has seen), and the number only counts whole
/// stretches that were looked at without filling a page, which says no more than the page that
/// stopped there ("there is more"). It never holds the name or the ID of a project that is hidden
/// from the person, and never a count of how many there are between two they can see.
struct ChoicePosition {
    after: Option<String>,
    skip: u64,
}

impl ChoicePosition {
    /// What the cursor text says: `NAME`, `NAME+500`, `+500`, or nothing for the start.
    fn read(cursor: Option<&str>) -> Result<Self, AppError> {
        let invalid = || AppError::Refused(ErrorCode::InvalidRequest);
        let Some(text) = cursor else {
            return Ok(Self {
                after: None,
                skip: 0,
            });
        };
        let (name, skip) = match text.split_once('+') {
            Some((name, skip)) => (name, skip.parse::<u64>().map_err(|_| invalid())?),
            None => (text, 0),
        };
        if skip % SCANNED_AT_MOST != 0 {
            return Err(invalid());
        }
        let after = if name.is_empty() {
            None
        } else {
            Some(
                bb_core::project::ShortName::new(name)
                    .map_err(|_| invalid())?
                    .to_string(),
            )
        };
        Ok(Self { after, skip })
    }

    fn write(&self) -> String {
        let name = self.after.as_deref().unwrap_or_default();
        if self.skip == 0 {
            name.to_owned()
        } else {
            format!("{name}+{}", self.skip)
        }
    }
}

/// The projects the person may choose for an access key: those they know about, by short name, a
/// page at a time (see [`ChoicePosition`] for what the cursor holds).
///
/// # Errors
/// `request.invalid` for a cursor that is not one of these.
pub async fn project_choices<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ProjectChoicesQuery,
) -> Result<Authorized<ProjectChoicePage>, AppError> {
    let record = ctx.active(&input.session_token).await?;
    let owner = record.person.id;
    let limit = usize::try_from(bb_core::paging::page_size(Some(input.limit))).unwrap_or(1);
    let start = ChoicePosition::read(input.cursor.as_deref())?;
    let actor = Actor {
        person: record.person,
        key: None,
    };
    let (mut items, mut looked_at, mut more) = (Vec::new(), 0_u64, false);
    'scan: while looked_at < SCANNED_AT_MOST {
        let batch = ctx
            .store()
            .projects_after(
                start.after.as_deref(),
                start.skip + looked_at,
                READ_AT_A_TIME,
            )
            .await
            .map_err(|error| internal::<S>(&error))?;
        let full = u64::try_from(batch.len()).is_ok_and(|count| count == READ_AT_A_TIME);
        for project in batch {
            let access = ctx
                .evaluation_of(&actor, Place::Project(project.id))
                .await?;
            if access.knows_project() {
                if items.len() == limit {
                    more = true;
                    break 'scan;
                }
                items.push(project);
            }
            looked_at += 1;
        }
        if !full {
            break;
        }
    }
    let next = if more {
        // A full page continues after its last project, which the person has just been shown.
        items.last().map(|last| ChoicePosition {
            after: Some(last.short_name.clone()),
            skip: 0,
        })
    } else if looked_at >= SCANNED_AT_MOST {
        // A page that looked at enough projects continues past them.
        Some(ChoicePosition {
            after: start.after,
            skip: start.skip + SCANNED_AT_MOST,
        })
    } else {
        None
    };
    let access = ctx.evaluate(owner, Place::Organization).await?;
    Ok(access.authorize(ProjectChoicePage {
        items,
        next_cursor: next.as_ref().map(ChoicePosition::write),
    }))
}

/// Whether the person behind a session, or behind an access key, may see how the server is doing:
/// a key counts with the narrowing it has, so a key that may see the status is let in.
///
/// # Errors
/// `auth.not_signed_in`, `auth.access_key_not_allowed` never (a key is the actor here),
/// `access.forbidden` without the permission.
pub async fn see_server_status<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: SeeServerStatusQuery,
) -> Result<Authorized<Done>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let access = ctx
        .require(&actor, Permission::SeeStatus, Place::Organization)
        .await?;
    Ok(access.authorize(Done))
}

/// Who asks, judged before anything else is read: an access key is turned away (and written into
/// the security log) from the routes of the person's own sign-in data before a malformed body or
/// an address that does not exist gets an answer of its own. A session passes.
///
/// # Errors
/// `auth.access_key_not_allowed` for a key, `auth.not_signed_in` for no session.
pub async fn require_session<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: RequireSessionQuery,
) -> Result<Done, AppError> {
    ctx.session(&input.session_token, SessionStage::Active)
        .await
        .map(|_| Done)
}

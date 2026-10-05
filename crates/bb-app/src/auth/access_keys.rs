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
use bb_core::access_key::{AccessKey, KeyChoice, MOST_KEYS, MOST_PROJECTS, NAME_MAX};
use bb_core::id::{AccessKeyId, ProjectId, UserId};
use bb_core::sign_in::SecurityAction;
use bb_policy::{Authorized, Choice, Narrowing, Permission, Place};
use jiff::Timestamp;
use serde_json::json;

use super::ctx::{AppError, Ctx, by, change_failed, entity, internal, set_to};
use super::store::{AccessKeyRecord, AuthStore};
use super::types::{
    AccessKeyPage, AccessKeyRules, AccessKeyRulesQuery, AccessKeyView, AccessKeysQuery,
    CreatedAccessKey, DeleteAccessKey, Done, MakeAccessKey, ProjectChoicePage, ProjectChoicesQuery,
};
use crate::access::{Actor, KeyInUse};

/// The purpose under which guessing keys is counted per network (`Ctx::reserve_network`).
const GUESSING: &str = "access-key";

const SECONDS_PER_DAY: i64 = 24 * 3600;

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
        // The lookup is by hash; this comparison is for the record that came back.
        if !same_digest(&record.key.secret_hash, &hash) || record.key.expires_at <= self.now() {
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
            let subject = entity("access_key", record.key.id).ok();
            self.note(SecurityAction::AccessKeyRefused, Some(owner.id), subject)
                .await;
            return Err(AppError::Refused(ErrorCode::SecondStepRequired));
        }
        self.via_key
            .store(true, std::sync::atomic::Ordering::Relaxed);
        self.store()
            .touch_access_key(record.key.id, to_the_minute(self.now()))
            .await
            .map_err(|error| internal::<S>(&error))?;
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
                    .and_then(|key| entity("access_key", key.id).ok());
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

/// The name as it is kept: trimmed, not empty, not too long, no control characters.
fn clean_name(name: &str) -> Result<String, AppError> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > NAME_MAX || name.chars().any(char::is_control) {
        return Err(ErrorCode::InvalidRequest.into());
    }
    Ok(name.to_owned())
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
    // Single permissions come with the choice *More options* and only with it.
    if (choice == KeyChoice::Permissions) == wanted.is_empty() {
        return Err(ErrorCode::InvalidRequest.into());
    }
    Ok(wanted.into_iter().collect())
}

/// Makes an access key for the person. The key is returned once; only a hash of its secret is
/// kept. The password is asked again, so a stolen session cannot make a key that outlives it.
///
/// # Errors
/// `auth.wrong_credentials` for a wrong password, `request.invalid` for a name, a date, a choice
/// or a project that cannot be used (or too many keys), `resource.not_found` for a project the
/// person may not know about, `auth.access_key_not_allowed` for a key.
pub async fn make_access_key<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: MakeAccessKey,
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
    let secret = AccessKeySecret::generate()?;
    let stored = AccessKeyRecord {
        key: AccessKey {
            id: AccessKeyId::generate(&mut ctx.ids()),
            owner,
            name,
            prefix: secret.prefix().to_owned(),
            secret_hash: secret.hash(),
            choice: input.choice,
            created_at: now,
            expires_at,
            last_used_at: None,
        },
        permissions,
        projects,
    };
    // The event names what the key may do and when it runs out; never the secret or its hash.
    let event = ctx.event(
        by(owner),
        entity("access_key", stored.key.id)?,
        vec![
            set_to("name", json!(stored.key.name)),
            set_to("choice", json!(stored.key.choice.code())),
            set_to("expires_at", json!(stored.key.expires_at.to_string())),
        ],
    )?;
    let to_store = stored.clone();
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move { S::insert_access_key(transaction, &to_store).await })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, None))?;
    ctx.note(
        SecurityAction::AccessKeyMade,
        Some(owner),
        Some(entity("access_key", stored.key.id)?),
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
        entity("access_key", id)?,
        vec![super::ctx::changed("access_key")],
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
        .map_err(|error| change_failed::<S>(&error, Some(ErrorCode::NotFound)))?;
    ctx.note(
        SecurityAction::AccessKeyDeleted,
        Some(owner),
        Some(entity("access_key", id)?),
    )
    .await;
    Ok(Done)
}

/// Where a page ends, as text: the time the last key was made (microseconds) and its ID.
fn write_cursor(last: &AccessKeyRecord) -> String {
    format!("{}.{}", last.key.created_at.as_microsecond(), last.key.id)
}

fn read_cursor(text: &str) -> Result<(Timestamp, AccessKeyId), ()> {
    let (micros, id) = text.split_once('.').ok_or(())?;
    let created = Timestamp::from_microsecond(micros.parse().map_err(|_| ())?).map_err(|_| ())?;
    Ok((created, id.parse().map_err(|_| ())?))
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
    let limit = u64::from(bb_core::paging::page_size(Some(input.limit)));
    let after = input
        .cursor
        .as_deref()
        .map(read_cursor)
        .transpose()
        .map_err(|()| AppError::Refused(ErrorCode::InvalidRequest))?;
    // One more than the page, to know whether there is a next one.
    let mut found = ctx
        .store()
        .access_keys_of(owner, after, limit + 1)
        .await
        .map_err(|error| internal::<S>(&error))?;
    let more = u64::try_from(found.len()).is_ok_and(|count| count > limit);
    found.truncate(usize::try_from(limit).unwrap_or(usize::MAX));
    let next_cursor = more.then(|| found.last().map(write_cursor)).flatten();
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
        permissions: Permission::ALL
            .iter()
            .map(|permission| (*permission, permission.topic()))
            .collect(),
    }))
}

/// How many projects are looked at to fill one page of the projects to choose from: a person who
/// sees few of many projects gets a short page and a cursor, not a slow answer.
const SCANNED_AT_MOST: u64 = 500;

/// The projects the person may choose for an access key: those they know about, by short name, a
/// page at a time (the cursor is the short name of the last project of the page before).
///
/// # Errors
/// `request.invalid` for a cursor that is not a short name.
pub async fn project_choices<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ProjectChoicesQuery,
) -> Result<Authorized<ProjectChoicePage>, AppError> {
    let record = ctx.active(&input.session_token).await?;
    let owner = record.person.id;
    let limit = usize::try_from(bb_core::paging::page_size(Some(input.limit))).unwrap_or(1);
    let mut after = match input.cursor {
        Some(name) => Some(
            bb_core::project::ShortName::new(&name)
                .map_err(|_| AppError::Refused(ErrorCode::InvalidRequest))?
                .to_string(),
        ),
        None => None,
    };
    let actor = Actor {
        person: record.person,
        key: None,
    };
    let (mut items, mut scanned, mut more) = (Vec::new(), 0_u64, false);
    // One more than the page, to know whether there is a next one.
    'scan: while scanned < SCANNED_AT_MOST {
        let batch = ctx
            .store()
            .projects_after(after.as_deref(), 100)
            .await
            .map_err(|error| internal::<S>(&error))?;
        let full = batch.len() == 100;
        for project in batch {
            scanned += 1;
            after = Some(project.short_name.clone());
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
        }
        if !full {
            break;
        }
    }
    // A page that stopped because it looked at enough projects continues after the last it saw;
    // a page that stopped for being full continues after its last project.
    let next_cursor = if more {
        items.last().map(|project| project.short_name.clone())
    } else if scanned >= SCANNED_AT_MOST {
        after
    } else {
        None
    };
    let access = ctx.evaluate(owner, Place::Organization).await?;
    Ok(access.authorize(ProjectChoicePage { items, next_cursor }))
}

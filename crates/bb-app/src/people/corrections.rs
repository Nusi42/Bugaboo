//! Corrections of a person's name and e-mail address (PRIV-06, ADR-0044, decision 9).
//!
//! A person changes their own name at once; the history and the security log keep the change. A
//! new e-mail address counts only once the person opens the link that was sent to it (one hour,
//! one use, the secret kept as its keyed digest); the old address keeps working until then and is
//! told when the change is done. An address that another account has is answered like a free one,
//! and the e-mail to it says an account exists there instead, so no answer tells whether an address
//! is taken. With e-mail off nothing could prove the new address, so the change is refused and an
//! administrator changes it.
//!
//! An administrator with *Manage people* corrects the name and the e-mail address of somebody
//! else, always with a reason; the change counts at once, the person is told (and both addresses,
//! for an address), and the security log records who corrected what of whom. A correction
//! replaces a value that was wrong, so its event keeps only the keyed digests of the values, never
//! the old value in clear. Only a session reaches these use cases: no access key or agent.

use bb_auth::{EmailAddress, Purpose, SecretToken};
use bb_core::ErrorCode;
use bb_core::app_paths;
use bb_core::e_mail_change::{EmailChange, LINK_HOURS, lasts};
use bb_core::event::{Actor as EventActor, FieldChange, RecordKind};
use bb_core::hex;
use bb_core::id::{InvitationId, JobId, UserId};
use bb_core::keeping::Step;
use bb_core::message::{self, Message};
use bb_core::person::Person;
use bb_core::security_log::Digest;
use bb_core::sign_in::SecurityAction;
use bb_core::wording::Region;
use bb_policy::{Authorized, Masked, Permission, Place};
use jiff::Timestamp;
use serde::Serialize;
use serde_json::{Value, json};

use super::{PersonDetail, PersonQuery};
use crate::access::Actor;
use crate::auth::{
    AppError, AuthStore, Capped, Ctx, Done, LinkMail, MailText, Stops, by, change_failed, changed,
    entity, internal, locked, set_to,
};
use crate::input::{clean_name, given_reason};
use crate::jobs::JobKind;
use crate::keeping::KEEP_STEP_ROWS;
use crate::mailer::MailMessage;
use crate::member;
use crate::notifications::{MailJob, send_failure_for_job};
use crate::ports::ChangeStore;
use crate::projects::Why;
use crate::store_macros::{reads, writes};

/// What the corrections need from storage.
pub trait CorrectionStore: ChangeStore + 'static {
    reads! {
        /// The change of an address that waits for the link whose secret has this keyed digest.
        fn e_mail_change_by_digest(digest: &Digest) -> Option<EmailChange>;
    }

    writes! {
        /// Gives the person this name.
        fn set_person_name(person: UserId, name: &str) -> ();
        /// Gives the person this e-mail address. The change stops itself when another account has
        /// it (the address is a unique key).
        fn set_person_email(person: UserId, email: &str) -> ();
        /// Stores the change of the person's address that waits for its link, in place of the one
        /// that waited before.
        fn put_e_mail_change(change: &EmailChange) -> ();
        /// Deletes the change of the address that waits for the link with this keyed digest of its
        /// secret; whether there was one. The one statement that uses a link up.
        fn take_e_mail_change(digest: &Digest) -> bool;
        /// Deletes the change of the person's address that waits, if one does.
        fn drop_e_mail_change(person: UserId) -> ();
        /// Deletes at most `limit` of the changes whose link ran out before `now`; returns how many.
        fn delete_old_e_mail_changes(now: Timestamp, limit: u64) -> u64;
    }
}

/// Change the name of the person who asks.
pub struct ChangeMyName {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The new name.
    pub name: String,
}

/// Ask for a new e-mail address for the person who asks: a link goes to it.
pub struct AskEmailChange {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The password, asked again for this.
    pub password: String,
    /// The new address.
    pub email: String,
}

/// Open the link that makes a new e-mail address the person's own.
pub struct ConfirmEmailChange {
    /// The secret of the link.
    pub secret: String,
}

/// Correct the name of somebody else, as an administrator.
pub struct CorrectName {
    /// The secret of the session of the request.
    pub session_token: String,
    /// Whose name.
    pub person: UserId,
    /// The name from now on.
    pub name: String,
    /// Why, in the administrator's words; the person is told.
    pub reason: Option<String>,
}

/// Correct the e-mail address of somebody else, as an administrator.
pub struct CorrectEmail {
    /// The secret of the session of the request.
    pub session_token: String,
    /// Whose address.
    pub person: UserId,
    /// The address from now on.
    pub email: String,
    /// Why, in the administrator's words; the person is told.
    pub reason: Option<String>,
}

/// The name of the person who asks, as they changed it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MyName {
    /// The name.
    pub name: String,
}

impl Masked for MyName {}

/// Which note an address gets when the address of an account changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ChangedBy {
    /// The person opened the link to their new address.
    Themselves,
    /// An administrator corrected the address.
    Administrator,
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The member of an event that holds a value only as its keyed digest: what a correction keeps
    /// of the value it replaced, and of an address.
    fn digest_change(
        &self,
        field: &str,
        purpose: Purpose,
        before: &str,
        after: &str,
    ) -> Result<FieldChange, AppError> {
        let digests = &self.services.digests;
        let hex_of = |value: &str| -> Result<Value, AppError> {
            Ok(json!(hex::encode(
                digests.digest(purpose, value.as_bytes())?.as_bytes()
            )))
        };
        Ok(FieldChange {
            field: field.to_owned(),
            before: Some(hex_of(before)?),
            after: Some(hex_of(after)?),
        })
    }
}

/// What a correction writes.
enum Corrected {
    /// A name.
    Name(String),
    /// An address an administrator gives.
    Email(String),
    /// The address of the change that waits for the link with this keyed digest of its secret.
    Confirmed { address: String, digest: Digest },
}

/// A change of a person's name or address: its event, what it writes, and the e-mails that go
/// with it.
struct CorrectionWrite<'e> {
    event: &'e bb_core::event::DomainEvent,
    person: UserId,
    value: Corrected,
    mails: Vec<MailJob>,
}

impl CorrectionWrite<'_> {
    /// Writes the change in one go: the one write of every correction (ARCH-EVENTS reads the call
    /// by its name).
    async fn record_change<S: AuthStore>(self, ctx: &Ctx<'_, S>) -> Result<(), AppError> {
        let jobs = self
            .mails
            .into_iter()
            .map(|mail| {
                Ok((
                    JobId::generate(&mut ctx.ids())
                        .map_err(|error| AppError::Internal(error.to_string()))?,
                    mail.to_value(),
                ))
            })
            .collect::<Result<Vec<_>, AppError>>()?;
        let kind = JobKind::MailSend.name()?;
        let (now, person, value) = (ctx.now(), self.person, self.value);
        let why = Why::default();
        let stopped = why.clone();
        ctx.store()
            .record_change(self.event, move |transaction| {
                Box::pin(async move {
                    match &value {
                        Corrected::Name(name) => {
                            S::set_person_name(transaction, person, name).await?;
                        }
                        Corrected::Email(address) => {
                            S::drop_e_mail_change(transaction, person).await?;
                            S::set_person_email(transaction, person, address).await?;
                        }
                        Corrected::Confirmed { address, digest } => {
                            if !S::take_e_mail_change(transaction, digest).await? {
                                return Err(stopped.stop::<S>(ErrorCode::EmailChangeLinkInvalid));
                            }
                            S::set_person_email(transaction, person, address).await?;
                        }
                    }
                    for (id, payload) in &jobs {
                        S::enqueue_job(transaction, *id, &kind, payload, now, now).await?;
                    }
                    Ok(())
                })
            })
            .await
            .map_err(|error| {
                why.answer_with::<S>(
                    &error,
                    ErrorCode::EmailChangeLinkInvalid,
                    ErrorCode::EmailTaken,
                )
            })?;
        // The person as this request knew them is out of date.
        locked(&ctx.known.people).remove(&person);
        Ok(())
    }
}

/// Changes the name of the person who asks, at once. The history keeps the change, and so does the
/// security log. *Who sees people* decides who sees the new name, as it did the old one.
///
/// # Errors
/// `request.invalid` for a name that is empty, too long or holds a control character.
pub async fn change_my_name<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ChangeMyName,
) -> Result<Authorized<MyName>, AppError> {
    let actor = ctx.session_actor(&input.session_token).await?;
    let name = clean_name(&input.name)?;
    let held = ctx.evaluation_of(&actor, Place::Organization).await?;
    let me = &actor.person;
    if own_name(me) == name {
        return Ok(held.authorize(MyName { name }));
    }
    let event = ctx.event(
        by(me.id),
        entity(RecordKind::Person, me.id),
        vec![FieldChange {
            field: member::person::NAME.to_owned(),
            before: Some(json!(own_name(me))),
            after: Some(json!(name)),
        }],
    )?;
    CorrectionWrite {
        event: &event,
        person: me.id,
        value: Corrected::Name(name.clone()),
        mails: Vec::new(),
    }
    .record_change(ctx)
    .await?;
    ctx.note(
        SecurityAction::OwnNameChanged,
        Some(me.id),
        Some(entity(RecordKind::Person, me.id)),
    )
    .await;
    Ok(held.authorize(MyName { name }))
}

/// The name of the person who asks: their own, which they always see.
fn own_name(me: &Person) -> &str {
    &me.display_name
}

/// The address of the account of `person`: whom the e-mails of a change of it go to.
fn address_of(person: &Person) -> &str {
    &person.email
}

/// Asks for a new e-mail address: a job sends a link to it, which makes it the person's address
/// when it is opened within the hour. The answer is the same whether the address is free or not.
/// The password is asked again, so a stolen session cannot move the account to another address.
///
/// # Errors
/// `request.invalid` for an address that cannot be one, `e-mail-change.mail-off` while e-mail is
/// off, `auth.wrong-credentials` for a wrong password.
pub async fn ask_e_mail_change<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: AskEmailChange,
) -> Result<Done, AppError> {
    let actor = ctx.session_actor(&input.session_token).await?;
    let email = EmailAddress::parse(&input.email).ok_or(ErrorCode::InvalidRequest)?;
    if !ctx.can_send_mail().await? {
        return Err(ErrorCode::EmailChangeNeedsMail.into());
    }
    ctx.confirm_password(&actor.person, input.password).await?;
    let me = actor.person.id;
    if email.as_str() == address_of(&actor.person) {
        return Ok(Done);
    }
    // The change names no address: the address is only in the job, until the job has run.
    let event = ctx.event(
        by(me),
        entity(RecordKind::Person, me),
        vec![changed("e-mail-change-asked")],
    )?;
    let job =
        JobId::generate(&mut ctx.ids()).map_err(|error| AppError::Internal(error.to_string()))?;
    let (kind, payload) = (
        JobKind::MailSend.name()?,
        MailJob::EmailChange {
            person: me,
            email: email.as_str().to_owned(),
            link: InvitationId::generate(&mut ctx.ids()),
        }
        .to_value(),
    );
    let now = ctx.now();
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(
                async move { S::enqueue_job(transaction, job, &kind, &payload, now, now).await },
            )
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::NONE))?;
    ctx.note(
        SecurityAction::EmailChangeAsked,
        Some(me),
        Some(entity(RecordKind::Person, me)),
    )
    .await;
    Ok(Done)
}

/// Opens the link to a new e-mail address: from now on it is the person's address, and the old one
/// is told. A session is not needed: the secret of the link proves the address.
///
/// # Errors
/// `e-mail-change.link-invalid` for a link that ran out, was used, was replaced by a newer one, or
/// whose address an account took meanwhile.
pub async fn confirm_e_mail_change<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ConfirmEmailChange,
) -> Result<Done, AppError> {
    let invalid = || AppError::Refused(ErrorCode::EmailChangeLinkInvalid);
    let secret = SecretToken::parse(&input.secret).ok_or_else(invalid)?;
    let digest = ctx.services.digests.secret(Purpose::EmailChange, &secret)?;
    let failed = |error| internal::<S>(&error);
    let now = ctx.now();
    let change = ctx
        .store()
        .e_mail_change_by_digest(&digest)
        .await
        .map_err(failed)?
        .filter(|change| change.works_at(now))
        .ok_or_else(invalid)?;
    let whose = ctx
        .store()
        .person(change.person)
        .await
        .map_err(failed)?
        .ok_or_else(invalid)?;
    let event = ctx.event(
        by(whose.id),
        entity(RecordKind::Person, whose.id),
        vec![ctx.digest_change(
            member::person::EMAIL,
            Purpose::Address,
            address_of(&whose),
            &change.new_email,
        )?],
    )?;
    let told = MailJob::EmailChanged {
        email: address_of(&whose).to_owned(),
        at: now,
        by: ChangedBy::Themselves,
    };
    CorrectionWrite {
        event: &event,
        person: whose.id,
        value: Corrected::Confirmed {
            address: change.new_email,
            digest,
        },
        mails: vec![told],
    }
    .record_change(ctx)
    .await
    .map_err(|error| match error {
        // The address was taken by another account since the link went: the link does not work.
        AppError::Refused(ErrorCode::EmailTaken) => invalid(),
        other => other,
    })?;
    ctx.note(
        SecurityAction::OwnEmailChanged,
        Some(whose.id),
        Some(entity(RecordKind::Person, whose.id)),
    )
    .await;
    Ok(Done)
}

/// What an administrator asks to correct, checked: who asks, whom, why, and the person as they are.
struct Correction {
    actor: Actor,
    whose: Person,
    reason: String,
}

impl<S: AuthStore> Ctx<'_, S> {
    /// Checks a correction by an administrator: a session with *Manage people*, somebody else, a
    /// reason, and a person who is there.
    async fn correction(
        &self,
        session_token: &str,
        person: UserId,
        reason: Option<&str>,
    ) -> Result<Correction, AppError> {
        let actor = self.session_actor(session_token).await?;
        self.require(&actor, Permission::ManagePeople, Place::Organization)
            .await?;
        if person == actor.person.id {
            // One's own name and address are changed in *My settings › Profile*: the address
            // through its link, never at once on the word of the person who asks.
            return Err(ErrorCode::CorrectionOwnProfile.into());
        }
        let reason = given_reason(reason, ErrorCode::CorrectionReasonNeeded)?;
        let whose = self
            .store()
            .person(person)
            .await
            .map_err(|error| internal::<S>(&error))?
            .ok_or(AppError::Refused(ErrorCode::NotFound))?;
        Ok(Correction {
            actor,
            whose,
            reason,
        })
    }

    /// The event of a correction: by the administrator, about the person, with the digests of the
    /// value and the reason, which the notification of the person reads.
    fn correction_event(
        &self,
        correction: &Correction,
        value: FieldChange,
    ) -> Result<bb_core::event::DomainEvent, AppError> {
        self.event(
            by(correction.actor.person.id),
            entity(RecordKind::Person, correction.whose.id),
            vec![value, set_to(member::REASON, json!(correction.reason))],
        )
    }

    /// The person as the administrator sees them after a correction.
    async fn corrected(
        &self,
        session_token: String,
        person: UserId,
    ) -> Result<Authorized<PersonDetail>, AppError> {
        super::person(
            self,
            PersonQuery {
                session_token,
                person,
            },
        )
        .await
    }
}

/// Corrects the name of somebody else, with *Manage people* and a reason. The person is told, and
/// the security log records who corrected whose name.
///
/// # Errors
/// `access.forbidden` without the permission, `correction.own-profile` for one's own name,
/// `correction.reason-needed` for an empty reason,
/// `request.invalid` for a name that cannot be one or a reason that is too long,
/// `resource.not-found` for a person who is not there.
pub async fn correct_name<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: CorrectName,
) -> Result<Authorized<PersonDetail>, AppError> {
    let correction = ctx
        .correction(&input.session_token, input.person, input.reason.as_deref())
        .await?;
    let name = clean_name(&input.name)?;
    let whose = correction.whose.id;
    if correction.whose.display_name != name {
        let value = ctx.digest_change(
            member::person::NAME,
            Purpose::PersonName,
            &correction.whose.display_name,
            &name,
        )?;
        let event = ctx.correction_event(&correction, value)?;
        CorrectionWrite {
            event: &event,
            person: whose,
            value: Corrected::Name(name),
            mails: Vec::new(),
        }
        .record_change(ctx)
        .await?;
        ctx.note(
            SecurityAction::NameCorrected,
            Some(correction.actor.person.id),
            Some(entity(RecordKind::Person, whose)),
        )
        .await;
    }
    ctx.corrected(input.session_token, whose).await
}

/// Corrects the e-mail address of somebody else, with *Manage people* and a reason. It counts at
/// once (the administrator vouches for it); both addresses are told, the person is told in the app
/// too, and the security log records who corrected whose address. A change the person asked for
/// and did not confirm yet is dropped.
///
/// # Errors
/// As [`correct_name`], and `person.e-mail-taken` for an address another account has.
pub async fn correct_e_mail<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: CorrectEmail,
) -> Result<Authorized<PersonDetail>, AppError> {
    let correction = ctx
        .correction(&input.session_token, input.person, input.reason.as_deref())
        .await?;
    let email = EmailAddress::parse(&input.email).ok_or(ErrorCode::InvalidRequest)?;
    let whose = correction.whose.id;
    let old = address_of(&correction.whose).to_owned();
    if old != email.as_str() {
        let value = ctx.digest_change(
            member::person::EMAIL,
            Purpose::Address,
            &old,
            email.as_str(),
        )?;
        let event = ctx.correction_event(&correction, value)?;
        let at = ctx.now();
        let told = [old, email.as_str().to_owned()].map(|address| MailJob::EmailChanged {
            email: address,
            at,
            by: ChangedBy::Administrator,
        });
        CorrectionWrite {
            event: &event,
            person: whose,
            value: Corrected::Email(email.as_str().to_owned()),
            mails: told.into(),
        }
        .record_change(ctx)
        .await?;
        ctx.note(
            SecurityAction::EmailCorrected,
            Some(correction.actor.person.id),
            Some(entity(RecordKind::Person, whose)),
        )
        .await;
    }
    ctx.corrected(input.session_token, whose).await
}

impl<S: AuthStore> Ctx<'_, S> {
    /// Sends the e-mail of a change of address (the work of a job of the kind `mail.send`): the
    /// link that makes it the person's, or, for an address with an account, the word that it has
    /// one. The link is made now and works for an hour from now; it replaces the one that waited,
    /// for a taken address too, so a taken address leaves the trace a free one does.
    pub(crate) async fn send_e_mail_change_mail(
        &self,
        person: UserId,
        address: &str,
        link: InvitationId,
    ) -> Result<(), AppError> {
        let mail = LinkMail {
            address,
            link,
            lasts: lasts(),
            exists: MailText {
                subject: Message::from(message::MAIL_E_MAIL_CHANGE_EXISTS_SUBJECT),
                text: message::mail_e_mail_change_exists_text(self.app_address(app_paths::SIGN_IN)),
            },
            capped: Capped::Always,
        };
        self.send_link_mail(mail, |made| async move {
            let change = EmailChange {
                person,
                token_digest: self
                    .services
                    .digests
                    .secret(Purpose::EmailChange, &made.secret)?,
                new_email: address.to_owned(),
                asked_at: self.now(),
                expires_at: made.expires_at,
            };
            let event = self.event(
                EventActor::System,
                entity(RecordKind::Person, person),
                vec![changed("e-mail-change-link")],
            )?;
            self.store()
                .record_change(&event, move |transaction| {
                    Box::pin(async move { S::put_e_mail_change(transaction, &change).await })
                })
                .await
                .map_err(|error| change_failed::<S>(&error, Stops::NONE))?;
            Ok(MailText {
                subject: Message::from(message::MAIL_E_MAIL_CHANGE_SUBJECT),
                text: message::mail_e_mail_change_text(message::MailEMailChangeText {
                    address: &self.app_address(&made.path(app_paths::E_MAIL_CHANGE)),
                    hours: LINK_HOURS,
                }),
            })
        })
        .await
    }

    /// Sends the note that the address of an account changed (the work of a job of the kind
    /// `mail.send`), with the moment and what to do if it was not the person.
    pub(crate) async fn send_e_mail_changed_mail(
        &self,
        address: &str,
        at: Timestamp,
        changed_by: ChangedBy,
    ) -> Result<(), AppError> {
        let layers = self.setting_layers_of(None).await?;
        let Some(server) = self.mail_server_if_on(&layers).await? else {
            return Ok(());
        };
        let moment = Region::of(&layers).moment(at);
        let sessions = self.app_address(app_paths::SESSIONS);
        let text = match changed_by {
            ChangedBy::Themselves => {
                message::mail_e_mail_changed_text(message::MailEMailChangedText {
                    address: &sessions,
                    moment: &moment,
                })
            }
            ChangedBy::Administrator => {
                message::mail_e_mail_corrected_text(message::MailEMailCorrectedText {
                    address: &sessions,
                    moment: &moment,
                })
            }
        };
        let message = MailMessage {
            to: address.to_owned(),
            subject: Message::from(message::MAIL_E_MAIL_CHANGED_SUBJECT).render(),
            text: text.render(),
        };
        self.services
            .mailer
            .send(&server, &message)
            .await
            .map_err(send_failure_for_job)
    }

    /// Deletes the changes of addresses whose link ran out: one step of the job that clears
    /// sessions.
    pub(crate) async fn delete_old_e_mail_changes(&self) -> Result<Step, AppError> {
        let now = self.now();
        let event = self.event(
            EventActor::System,
            crate::auth::fixed_entity(RecordKind::Person, "old-e-mail-changes")?,
            vec![changed("old-e-mail-changes")],
        )?;
        let deleted = self
            .store()
            .record_change(&event, move |transaction| {
                Box::pin(async move {
                    S::delete_old_e_mail_changes(transaction, now, KEEP_STEP_ROWS).await
                })
            })
            .await
            .map_err(|error| change_failed::<S>(&error, Stops::NONE))?;
        Ok(Step::of(deleted, KEEP_STEP_ROWS))
    }
}

//! Whose work times a request is about, and what the reader may do with them: the one rule of who
//! reads and who changes another person's work times (ADR-0030, decision 2 and amendment M8).
//!
//! - One's own, always, in a browser: no access key reaches work times.
//! - Another person's are read as *Who sees people* allows for the kind *Work times*: *Hidden*
//!   answers exactly like a person that does not exist, so nobody learns who has work times.
//! - Another person's usual work days, work hours and away times are changed with *Enter work times
//!   for others* and nothing else. Whoever holds it also reads them, by name. Nobody but the person
//!   reads or writes the note of an away time.
//! - A share is set by a person who decides who has which role in the project (`shares.rs`).

use std::sync::Arc;

use bb_core::ErrorCode;
use bb_core::access::PeopleLevel;
use bb_core::id::UserId;
use bb_core::people_kinds::PeopleKind;
use bb_policy::{Evaluation, PeopleContext, Permission, PersonRef, Place};

use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx};

/// The work times of one person, as one reader may use them.
pub(super) struct Whose {
    /// The person whose work times they are.
    pub(super) person: UserId,
    /// Whether they are the reader's own.
    pub(super) own: bool,
    /// What the reader may do for the whole organization.
    pub(super) evaluation: Arc<Evaluation>,
}

impl Whose {
    /// How the reader is shown the people of work times.
    pub(super) fn level(&self) -> PeopleLevel {
        self.evaluation
            .people_level(PeopleKind::WorkTimes, PeopleContext::WorkTimes)
    }

    /// Whether the reader may enter usual work days, work hours and away times for the person.
    pub(super) fn may_change(&self) -> bool {
        self.own || self.evaluation.allows(Permission::EnterWorkTimesForOthers)
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// Who asks about work times: a person in a browser. No access key reads work times, so a key
    /// is turned away with `auth.access-key-not-allowed`.
    ///
    /// # Errors
    /// The refusals of signing in.
    pub(super) async fn work_times_actor(&self, session_token: &str) -> Result<Actor, AppError> {
        self.session_actor(session_token).await
    }

    /// Who changes the work times of `person` (the actor's own when `None`): one's own are a
    /// record of the person in a browser, which no access key reaches; another person's are judged
    /// by *Enter work times for others*, which no key holds either (ADR-0025, amendment M8), so a
    /// key is refused there like anybody who lacks it.
    ///
    /// # Errors
    /// The refusals of signing in, and `auth.access-key-not-allowed` for a key and one's own.
    pub(super) async fn work_times_changer(
        &self,
        session_token: &str,
        person: Option<UserId>,
    ) -> Result<Actor, AppError> {
        match person {
            None => self.session_actor(session_token).await,
            Some(_) => self.actor(session_token).await,
        }
    }

    /// The work times of `person` (the actor's own when `None`) as the actor may read them.
    ///
    /// # Errors
    /// `resource.not-found` for another person whom the reader is shown as *Hidden*, and for a
    /// person that does not exist.
    pub(super) async fn work_times_to_read(
        &self,
        actor: &Actor,
        person: Option<UserId>,
    ) -> Result<Whose, AppError> {
        let whose = self.work_times_of(actor, person).await?;
        if !whose.own && whose.level() == PeopleLevel::Hidden {
            return Err(ErrorCode::NotFound.into());
        }
        self.known_work_times_person(&whose).await?;
        Ok(whose)
    }

    /// The work times of `person` (the actor's own when `None`) as the actor may change them.
    ///
    /// # Errors
    /// `resource.not-found` for another person whom the reader is shown as *Hidden* and may not
    /// enter work times for, and for a person that does not exist; `access.forbidden` for a reader
    /// who sees the person and may not enter their work times.
    pub(super) async fn work_times_to_change(
        &self,
        actor: &Actor,
        person: Option<UserId>,
    ) -> Result<Whose, AppError> {
        let whose = self.work_times_of(actor, person).await?;
        if whose.own {
            return Ok(whose);
        }
        if !whose.may_change() {
            return Err(if whose.level() == PeopleLevel::Hidden {
                ErrorCode::NotFound
            } else {
                ErrorCode::Forbidden
            }
            .into());
        }
        self.known_work_times_person(&whose).await?;
        Ok(whose)
    }

    /// The person of the work times as the reader sees them.
    pub(super) async fn work_times_person_of(&self, whose: &Whose) -> Result<PersonRef, AppError> {
        self.person_ref(
            &whose.evaluation,
            PeopleKind::WorkTimes,
            PeopleContext::WorkTimes,
            whose.person,
        )
        .await
    }

    async fn work_times_of(
        &self,
        actor: &Actor,
        person: Option<UserId>,
    ) -> Result<Whose, AppError> {
        let person = person.unwrap_or(actor.person.id);
        Ok(Whose {
            person,
            // A key never reaches its owner's own work times: it is judged like anybody else.
            own: person == actor.person.id && actor.key.is_none(),
            evaluation: self.evaluation_of(actor, Place::Organization).await?,
        })
    }

    /// Fails like a missing person when the person does not exist.
    async fn known_work_times_person(&self, whose: &Whose) -> Result<(), AppError> {
        if whose.own || self.person_known(whose.person).await?.is_some() {
            Ok(())
        } else {
            Err(ErrorCode::NotFound.into())
        }
    }
}

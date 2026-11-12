//! Whose work times a request is about, and what the reader may do with them: the one rule of who
//! reads and who changes another person's work times (ADR-0030, decision 2 and amendment M8).
//!
//! - One's own, always, in a browser: no access key reaches work times.
//! - Another person's are read as *Who sees people* allows for the kind *Work times*, by the most
//!   open level the reader has in the projects where the person has a role. A reader who shares no
//!   project with the person is shown them as *Hidden*, which answers exactly like a person that
//!   does not exist, so nobody learns who has work times.
//! - Another person's usual work days, work hours and away times are changed with *Enter work times
//!   for others* and nothing else. Whoever holds it also reads them, by name, whatever the projects.
//!   Nobody but the person reads or writes the note of an away time.
//! - A share is set by a person who decides who has which role in the project (`shares.rs`).

use std::sync::Arc;

use bb_core::ErrorCode;
use bb_core::access::PeopleLevel;
use bb_core::id::{ProjectId, UserId};
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
    /// How the reader is shown the person: the most open level of the kind *Work times* that they
    /// have where the person has a role, and *Names* for a holder of *Enter work times for others*.
    level: PeopleLevel,
    /// The rights the person is named with: those of the reader in the project that gave the level.
    naming: Arc<Evaluation>,
    /// Where `naming` counts.
    context: PeopleContext,
}

impl Whose {
    /// How the reader is shown the person.
    pub(super) const fn level(&self) -> PeopleLevel {
        self.level
    }

    /// Whether the reader may enter usual work days, work hours and away times for the person.
    pub(super) fn may_change(&self) -> bool {
        self.own || self.evaluation.allows(Permission::EnterWorkTimesForOthers)
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// Who asks about work times: a person in a browser. No access key reaches work times, so a key
    /// is turned away with `auth.access-key-not-allowed`.
    ///
    /// # Errors
    /// The refusals of signing in.
    pub(super) async fn work_times_actor(&self, session_token: &str) -> Result<Actor, AppError> {
        self.session_actor(session_token).await
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
            &whose.naming,
            PeopleKind::WorkTimes,
            whose.context,
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
        let evaluation = self.evaluation_of(actor, Place::Organization).await?;
        let own = person == actor.person.id;
        let (level, naming, context) =
            if own || evaluation.allows(Permission::EnterWorkTimesForOthers) {
                (
                    PeopleLevel::Names,
                    Arc::clone(&evaluation),
                    PeopleContext::WorkTimes,
                )
            } else {
                self.shared_level(actor, person, &evaluation).await?
            };
        Ok(Whose {
            person,
            own,
            evaluation,
            level,
            naming,
            context,
        })
    }

    /// The most open level of the kind *Work times* the reader has in the projects where the person
    /// has a role, with the rights of the reader in that project to name the person by. With no
    /// project in common the person is *Hidden* to the reader.
    async fn shared_level(
        &self,
        actor: &Actor,
        person: UserId,
        organization: &Arc<Evaluation>,
    ) -> Result<(PeopleLevel, Arc<Evaluation>, PeopleContext), AppError> {
        let projects: Vec<ProjectId> = self.project_tree_now().await?.projects().collect();
        self.evaluate_projects(actor.person.id, &projects).await?;
        let pairs: Vec<(UserId, ProjectId)> =
            projects.iter().map(|project| (person, *project)).collect();
        self.evaluate_people_in(&pairs).await?;
        let mut best = (
            PeopleLevel::Hidden,
            Arc::clone(organization),
            PeopleContext::Ordinary,
        );
        for project in projects {
            if !self.has_a_role_at(person, Place::Project(project)).await? {
                continue;
            }
            let held = self.evaluation_of(actor, Place::Project(project)).await?;
            let level = held.people_level(PeopleKind::WorkTimes, PeopleContext::Ordinary);
            if level > best.0 {
                best = (level, held, PeopleContext::Ordinary);
            }
        }
        Ok(best)
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

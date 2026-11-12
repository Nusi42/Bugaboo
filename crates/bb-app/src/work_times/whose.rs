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

use std::collections::{HashMap, HashSet};
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
}

/// How an evaluation shows people of the kind *Work times*: the one reading of the level, for the
/// reader of a person, the share list of a project and the minutes of the plan alike. A holder of
/// *Enter work times for others* is shown names (ADR-0030, amendment M8).
pub(super) fn work_times_level(held: &Evaluation) -> PeopleLevel {
    held.people_level(PeopleKind::WorkTimes, PeopleContext::WorkTimes)
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
        let evaluation = self.evaluation_of(actor, Place::Organization).await?;
        let own = person == actor.person.id;
        let (level, naming) = if own || evaluation.allows(Permission::EnterWorkTimesForOthers) {
            (PeopleLevel::Names, Arc::clone(&evaluation))
        } else {
            self.shared_level(actor, person, &evaluation).await?
        };
        Ok(Whose {
            person,
            own,
            evaluation,
            level,
            naming,
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
    ) -> Result<(PeopleLevel, Arc<Evaluation>), AppError> {
        Ok(self
            .shared_levels(actor, &[person], organization)
            .await?
            .remove(&person)
            .unwrap_or_else(|| (PeopleLevel::Hidden, Arc::clone(organization))))
    }

    /// The same for many people at once: the projects of all of them from one tree and one read of
    /// their rights, and the reader evaluated only there, whatever the number of people.
    async fn shared_levels(
        &self,
        actor: &Actor,
        people: &[UserId],
        organization: &Arc<Evaluation>,
    ) -> Result<HashMap<UserId, (PeopleLevel, Arc<Evaluation>)>, AppError> {
        let projects: Vec<ProjectId> = self.project_tree_now().await?.projects().collect();
        let pairs: Vec<(UserId, ProjectId)> = people
            .iter()
            .flat_map(|person| projects.iter().map(|project| (*person, *project)))
            .collect();
        self.evaluate_people_in(&pairs).await?;
        let mut theirs: HashMap<UserId, Vec<ProjectId>> = HashMap::new();
        for (person, project) in pairs {
            if self.has_a_role_at(person, Place::Project(project)).await? {
                theirs.entry(person).or_default().push(project);
            }
        }
        let mut all: Vec<ProjectId> = theirs.values().flatten().copied().collect();
        all.sort_unstable();
        all.dedup();
        self.evaluate_projects(actor.person.id, &all).await?;
        let mut found = HashMap::new();
        for person in people {
            let mut best = (PeopleLevel::Hidden, Arc::clone(organization));
            for project in theirs.get(person).into_iter().flatten() {
                let held = self.evaluation_of(actor, Place::Project(*project)).await?;
                let level = work_times_level(&held);
                if level > best.0 {
                    best = (level, held);
                }
            }
            found.insert(*person, best);
        }
        Ok(found)
    }

    /// Which of these people's work times the actor may know, by the one rule of this module: their
    /// own; everybody's with *Enter work times for others*; else the most open shared level, which
    /// is *Hidden* with no project in common. An access key knows nobody's: no key reaches work
    /// times (architecture §12).
    pub(crate) async fn work_times_known(
        &self,
        actor: &Actor,
        people: &[UserId],
    ) -> Result<HashSet<UserId>, AppError> {
        if actor.key.is_some() {
            return Ok(HashSet::new());
        }
        let organization = self.evaluation_of(actor, Place::Organization).await?;
        if organization.allows(Permission::EnterWorkTimesForOthers) {
            return Ok(people.iter().copied().collect());
        }
        let others: Vec<UserId> = people
            .iter()
            .copied()
            .filter(|person| *person != actor.person.id)
            .collect();
        let levels = self.shared_levels(actor, &others, &organization).await?;
        Ok(people
            .iter()
            .copied()
            .filter(|person| {
                *person == actor.person.id
                    || levels
                        .get(person)
                        .is_some_and(|(level, _)| *level != PeopleLevel::Hidden)
            })
            .collect())
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

//! Who may do what: the bridge between the use cases and the policy engine (SEC-01, SEC-03,
//! ADR-0004, ADR-0025).
//!
//! `bb-policy` decides without I/O; this module loads what it needs through the port
//! [`PolicyStore`] (one call per person and place), keeps the evaluations for the length of one
//! request in the [`Ctx`], and gives the use cases three things: who is asking ([`Actor`]), the
//! evaluation at a place ([`Ctx::evaluate`]) and the check that stops a request that may not do
//! what it asks ([`Ctx::require`]). A project that a person may not know about answers
//! `resource.not_found`, exactly like one that does not exist.

mod starting;
mod store;

use std::sync::Arc;

use bb_core::ErrorCode;
use bb_core::access::Assignment;
use bb_core::id::{ProjectId, UserId};
use bb_core::people_kinds::PeopleKind;
use bb_core::person::Person;
use bb_policy::guards::{Refusal, ensure_no_escalation};
use bb_policy::{Evaluation, PeopleContext, Place, Target, evaluate, resolve};

pub use bb_policy::{Authorized, Masked, Permission, PersonRef, StartingRole, guards};
pub use starting::{MakeStartingRoles, make_starting_roles};
pub use store::{PolicyStore, RoleRecord};

use serde::Serialize;

use crate::auth::{AppError, AuthStore, Ctx, internal};

/// Who asks: the signed-in person behind a session.
#[derive(Clone, Debug)]
pub(crate) struct Actor {
    /// The person.
    pub(crate) person: Person,
}

impl From<Refusal> for AppError {
    fn from(refusal: Refusal) -> Self {
        Self::Refused(match refusal {
            Refusal::WouldLockOut => ErrorCode::WouldLockOut,
            Refusal::TooMuch => ErrorCode::TooMuch,
        })
    }
}

/// A project as a person who may see it, or who decides who has which role there, sees it; a
/// project they may not know about is not named (ADR-0025, decision 12).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "shown", rename_all = "snake_case")]
pub enum ProjectLabel {
    /// The person may know the project.
    Visible {
        /// The ID of the project.
        id: ProjectId,
        /// Its short name.
        short_name: String,
        /// Its name.
        name: String,
    },
    /// A project the person cannot see: shown as *a project you can't see*.
    Hidden,
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The signed-in person behind a cookie.
    pub(crate) async fn actor(&self, token_text: &str) -> Result<Actor, AppError> {
        let record = self.active(token_text).await?;
        Ok(Actor {
            person: record.person,
        })
    }

    /// What `person` may do at `place`, from the policy engine. The result is kept for the rest
    /// of the request, so asking again costs nothing; the next request asks again.
    ///
    /// # Errors
    /// `resource.not_found` when the place is a project that does not exist.
    pub(crate) async fn evaluate(
        &self,
        person: UserId,
        place: Place,
    ) -> Result<Arc<Evaluation>, AppError> {
        if let Some(found) = self.cached_evaluation(person, place) {
            return Ok(found);
        }
        let inputs = self
            .store()
            .policy_inputs(person, place)
            .await
            .map_err(|error| internal::<S>(&error))?
            .ok_or(AppError::Refused(ErrorCode::NotFound))?;
        let evaluation = Arc::new(evaluate(&inputs));
        self.remember_evaluation(person, place, &evaluation);
        Ok(evaluation)
    }

    fn cached_evaluation(&self, person: UserId, place: Place) -> Option<Arc<Evaluation>> {
        // A lock that a panic poisoned still holds a map that is whole: it only ever gets whole
        // insertions.
        let cache = self
            .evaluations
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        cache.get(&(person, place)).cloned()
    }

    fn remember_evaluation(&self, person: UserId, place: Place, evaluation: &Arc<Evaluation>) {
        let mut cache = self
            .evaluations
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        cache.insert((person, place), Arc::clone(evaluation));
    }

    /// The evaluation of the actor at `place`, when they hold `permission` there.
    ///
    /// # Errors
    /// `resource.not_found` for a project the person may not know about (or that does not exist),
    /// `access.forbidden` when they know the place but lack the permission.
    pub(crate) async fn require(
        &self,
        actor: &Actor,
        permission: Permission,
        place: Place,
    ) -> Result<Arc<Evaluation>, AppError> {
        let evaluation = self.evaluate(actor.person.id, place).await?;
        if evaluation.allows(permission) {
            return Ok(evaluation);
        }
        if matches!(place, Place::Project(_)) && !evaluation.knows_project() {
            return Err(AppError::Refused(ErrorCode::NotFound));
        }
        Err(AppError::Refused(ErrorCode::Forbidden))
    }

    /// Whether setup is open: nobody can manage roles for the whole organization yet
    /// (AUTH-09, ADR-0025 decision 5).
    pub(crate) async fn setup_is_open(&self) -> Result<bool, AppError> {
        let state = self
            .store()
            .organization_state()
            .await
            .map_err(|error| internal::<S>(&error))?;
        Ok(!guards::anybody_manages_roles(&state))
    }

    /// How `viewer` sees `person` as `kind` at `place` (PRIV-12): a name, the label of their
    /// first role there, or *Someone*. A person that does not exist is *Someone*.
    pub(crate) async fn person_ref(
        &self,
        viewer: &Evaluation,
        kind: PeopleKind,
        context: PeopleContext,
        person: UserId,
    ) -> Result<PersonRef, AppError> {
        let Some(found) = self
            .store()
            .person(person)
            .await
            .map_err(|error| internal::<S>(&error))?
        else {
            return Ok(PersonRef::Someone);
        };
        // The role label is the person's own role at the place the viewer looks at.
        let role = match self.evaluate(person, viewer.place()).await {
            Ok(theirs) => theirs.first_role_name().map(str::to_owned),
            Err(_) => None,
        };
        Ok(resolve(
            viewer,
            kind,
            context,
            Target {
                id: found.id,
                name: &found.display_name,
                role: role.as_deref(),
                removed: None,
            },
        ))
    }

    /// The project as the actor may name it.
    pub(crate) async fn project_label(
        &self,
        actor: &Actor,
        project: ProjectId,
    ) -> Result<ProjectLabel, AppError> {
        let Ok(evaluation) = self
            .evaluate(actor.person.id, Place::Project(project))
            .await
        else {
            return Ok(ProjectLabel::Hidden);
        };
        if !evaluation.knows_project() {
            return Ok(ProjectLabel::Hidden);
        }
        Ok(self
            .store()
            .project(project)
            .await
            .map_err(|error| internal::<S>(&error))?
            .map_or(ProjectLabel::Hidden, |found| ProjectLabel::Visible {
                id: found.id,
                short_name: found.short_name.to_string(),
                name: found.name,
            }))
    }

    /// Whether the actor may give this row: they hold every permission of its role at the place
    /// it is given (ADR-0025, decision 6; organization permissions count nowhere in a project and
    /// are left out there). A row of *No access* gives nothing.
    ///
    /// # Errors
    /// `access.too_much` when they hold less than the role.
    pub(crate) async fn ensure_may_give(
        &self,
        actor: &Actor,
        row: &Assignment,
    ) -> Result<(), AppError> {
        let Some(role) = row.role else {
            return Ok(());
        };
        let Some(record) = self
            .store()
            .role(role)
            .await
            .map_err(|error| internal::<S>(&error))?
        else {
            return Ok(());
        };
        let place = row.project.map_or(Place::Organization, Place::Project);
        let held = self.evaluate(actor.person.id, place).await?;
        ensure_no_escalation(&held, &record.permissions)?;
        Ok(())
    }
}

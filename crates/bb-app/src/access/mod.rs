//! Who may do what: the bridge between the use cases and the policy engine (SEC-01, SEC-03,
//! ADR-0004, ADR-0025).
//!
//! `bb-policy` decides without I/O; this module loads what it needs through the port
//! [`PolicyStore`] (one call per person and place), keeps the evaluations for the length of one
//! request in the [`Ctx`], and gives the use cases three things: who is asking ([`Actor`]), the
//! evaluation at a place ([`Ctx::evaluate`]) and the check that stops a request that may not do
//! what it asks ([`Ctx::require`]). A project that a person may not know about answers
//! `resource.not_found`, exactly like one that does not exist.

mod candidates;
mod starting;
mod store;
mod visibility;

use std::sync::Arc;

use bb_auth::looks_like_access_key;
use bb_core::ErrorCode;
use bb_core::access::{AppliesTo, Assignment};
use bb_core::id::{AccessKeyId, ProjectId, UserId};
use bb_core::people_kinds::PeopleKind;
use bb_core::person::Person;
use bb_policy::guards::{Refusal, ensure_no_escalation};
use bb_policy::{Evaluation, Narrowing, PeopleContext, Target, evaluate, resolve};

pub use bb_policy::{
    Authorized, Masked, NOT_FOR_KEYS, Permission, PersonRef, Place, StartingRole, Tree, guards,
};
pub use candidates::{PeopleToCheckQuery, PersonPage, people_to_check};
pub use starting::{MakeStartingRoles, make_starting_roles};
pub use store::{LevelRules, PersonName, PolicyStore, RoleRecord};
pub use visibility::{
    FieldPermissions, FieldPermissionsQuery, LevelCell, LevelOrigin, LevelRow, LevelTable,
    PeopleVisibility, PeopleVisibilityQuery, ROLES_MAX, field_permissions, people_visibility,
};

use serde::Serialize;

use crate::auth::{AppError, AuthStore, Ctx, internal};

/// Who asks: the signed-in person behind a session, or the owner of the access key a script
/// came with.
#[derive(Clone, Debug)]
pub(crate) struct Actor {
    /// The person.
    pub(crate) person: Person,
    /// The access key behind the request, if the actor is a script: what it narrows its owner to.
    pub(crate) key: Option<KeyInUse>,
}

/// An access key in use (AUTH-07): the owner does everything as themselves, narrowed by this at
/// every request.
#[derive(Clone, Debug)]
pub(crate) struct KeyInUse {
    /// The key.
    pub(crate) id: AccessKeyId,
    /// What it narrows the owner's permissions to: its choice and its projects.
    pub(crate) narrowing: Narrowing,
}

impl Actor {
    /// What the access key behind the request narrows its owner to, if there is one. The one
    /// place that tells it: every decision about what the actor sees (the list, the search, the
    /// trail, the references, the preview of a move, the re-check of a move) is made with it.
    pub(crate) fn key_scope(&self) -> Option<&Narrowing> {
        self.key.as_ref().map(|key| &key.narrowing)
    }
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
    /// Who asks: the signed-in person behind a cookie, or the owner of an access key (`bb_...`,
    /// sent as `Authorization: Bearer`). Use cases that judge a permission start here; what
    /// belongs to the owner's own sign-in data starts at `active` and so never opens for a key.
    pub(crate) async fn actor(&self, token_text: &str) -> Result<Actor, AppError> {
        if looks_like_access_key(token_text) {
            return self.key_actor(token_text).await;
        }
        let record = self.active(token_text).await?;
        Ok(Actor {
            person: record.person,
            key: None,
        })
    }

    /// What `actor` may do at `place`: the evaluation of their person, narrowed by the access
    /// key if they came with one, at every request (a key never widens anything and follows its
    /// owner's roles at once, ADR-0025, decision 8). Every judgment about the actor goes through
    /// here or [`Self::require`], never through [`Self::evaluate`] with their ID.
    ///
    /// # Errors
    /// `resource.not_found` when the place is a project that does not exist.
    pub(crate) async fn evaluation_of(
        &self,
        actor: &Actor,
        place: Place,
    ) -> Result<Arc<Evaluation>, AppError> {
        let owners = self.evaluate(actor.person.id, place).await?;
        Ok(match &actor.key {
            Some(key) => Arc::new(owners.narrowed(&key.narrowing)),
            None => owners,
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
        let evaluation = self.evaluation_of(actor, place).await?;
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
        let Ok(evaluation) = self.evaluation_of(actor, Place::Project(project)).await else {
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
}

/// What `person` holds at `place`, read inside a change after the policy lock: what a rule of the
/// change is judged on, not what was stored when the request began.
///
/// # Errors
/// [`Refusal::TooMuch`] (as the error of the store) when the place is gone, or the error of the
/// store.
pub(crate) async fn held_in<S: AuthStore>(
    transaction: &mut S::Transaction,
    person: UserId,
    place: Place,
) -> Result<Evaluation, S::Error> {
    match S::policy_inputs_in(transaction, person, place).await? {
        Some(inputs) => Ok(evaluate(&inputs)),
        None => Err(S::refused(Refusal::TooMuch)),
    }
}

/// Whether `giver` may give this row: they hold every permission of its role at the place it is
/// given (ADR-0025, decision 6; organization permissions count nowhere in a project and are left
/// out there). A row for a project *and its sub-projects* reaches the sub-projects that are made
/// later as well as those that exist, so the giver must hold the role through rows that reach
/// sub-projects themselves (a row for the project alone does not count), and in each existing
/// sub-project too (a *No access* there counts). A lead for one project cannot make themselves
/// lead of everything below it. A row of *No access* gives nothing.
///
/// It runs inside the change, after the policy lock, so it judges what is stored now and not
/// what was stored when the request began.
///
/// # Errors
/// [`Refusal::TooMuch`] (as the error of the store) when they hold less than the role somewhere
/// the row reaches or the place does not exist, or the error of the store.
pub(crate) async fn ensure_may_give_in<S: AuthStore>(
    transaction: &mut S::Transaction,
    giver: UserId,
    row: &Assignment,
) -> Result<(), S::Error> {
    let Some(role) = row.role else {
        return Ok(());
    };
    let Some(record) = S::role_in(transaction, role).await? else {
        return Ok(());
    };
    let mut places = vec![row.project.map_or(Place::Organization, Place::Project)];
    let reaches_below = row.applies_to == AppliesTo::ProjectAndSubProjects;
    if let (Some(project), true) = (row.project, reaches_below) {
        places.extend(
            S::sub_projects_in(transaction, project)
                .await?
                .into_iter()
                .map(Place::Project),
        );
    }
    for (at, place) in places.into_iter().enumerate() {
        // A place that is gone is not one the giver holds anything in.
        let Some(mut inputs) = S::policy_inputs_in(transaction, giver, place).await? else {
            return Err(S::refused(Refusal::TooMuch));
        };
        if at == 0 && reaches_below {
            // Their rows for this project alone do not reach what will be made below it.
            inputs.rows.retain(|held| {
                !(held.project == row.project && held.applies_to == AppliesTo::Project)
            });
        }
        ensure_no_escalation(&evaluate(&inputs), &record.permissions).map_err(S::refused)?;
    }
    Ok(())
}

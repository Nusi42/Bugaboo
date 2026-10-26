//! Who may do what: the bridge between the use cases and the policy engine (SEC-01, SEC-03,
//! ADR-0004, ADR-0025).
//!
//! `bb-policy` decides without I/O; this module loads what it needs through the port
//! [`PolicyStore`] (one call per person and place), keeps the evaluations for the length of one
//! request in the [`Ctx`], and gives the use cases three things: who is asking ([`Actor`]), the
//! evaluation at a place ([`Ctx::evaluate`]) and the check that stops a request that may not do
//! what it asks ([`Ctx::require`]). A project that a person may not know about answers
//! `resource.not-found`, exactly like one that does not exist.

mod candidates;
mod changers;
mod own_loss;
mod starting;
mod store;
mod visibility;

use std::sync::Arc;

use bb_auth::looks_like_access_key;
use bb_core::ErrorCode;
use bb_core::access::{AppliesTo, Assignment};
use bb_core::id::{AccessKeyId, GroupId, ProjectId, UserId};
use bb_core::people_kinds::PeopleKind;
use bb_core::person::Person;
use bb_policy::guards::{Refusal, ensure_no_escalation};
use bb_policy::{Evaluation, Narrowing, PeopleContext, Target, evaluate, resolve};

pub use bb_policy::{
    Authorized, Masked, NOT_FOR_KEYS, Permission, PersonRef, Place, StartingRole, Tree, guards,
};
pub(crate) use candidates::{Members, names_other_people, position_of};
pub use candidates::{PeopleToCheckPage, PeopleToCheckQuery, people_to_check};
pub use changers::{ChangingRole, WhoMayChange, WhoMayChangeQuery, who_may_change};
pub use own_loss::{OwnChange, OwnLoss, OwnLossQuery, own_loss};
pub use starting::{CreateStartingRoles, create_starting_roles};
pub use store::{PlaceLevels, PolicyStore, RoleRecord};
pub use visibility::{
    FieldPermissions, FieldPermissionsQuery, LevelCell, LevelOrigin, LevelRow, LevelTable,
    PeopleVisibility, PeopleVisibilityQuery, ROLES_MAX, field_permissions, people_visibility,
};

use serde::Serialize;

use crate::auth::{AppError, AuthStore, Ctx, if_there, internal, locked};
use crate::projects::{Leveled, sub_projects_in};

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
            Refusal::CannotLift => ErrorCode::CannotLift,
            Refusal::InUse => ErrorCode::RoleInUse,
        })
    }
}

/// What `person` may do at `place`, read from the store and decided by the policy engine; `None`
/// when the place is a project that does not exist. The one place that loads what the engine needs
/// and asks it: `Ctx::evaluate` keeps the answer for the request, and the kinds of record that
/// the editing use cases know (`EditableRecords`) have no context and ask here.
pub(crate) async fn evaluation_in<S: AuthStore>(
    store: &S,
    person: UserId,
    place: Place,
) -> Result<Option<Evaluation>, AppError> {
    Ok(store
        .policy_inputs(person, place)
        .await
        .map_err(|error| internal::<S>(&error))?
        .map(|inputs| evaluate(&inputs)))
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

impl Masked for ProjectLabel {}

impl ProjectLabel {
    /// The project as a name the person may use: the one place that builds it.
    pub(crate) fn of(project: &bb_core::project::Project) -> Self {
        Self::Visible {
            id: project.id,
            short_name: project.short_name.to_string(),
            name: project.name.clone(),
        }
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// Who asks: the signed-in person behind a cookie, or the owner of an access key (`bb_...`,
    /// sent as `Authorization: Bearer`). Use cases that judge a permission start here; what
    /// belongs to the owner's own sign-in data starts at `active` and so never opens for a key.
    pub(crate) async fn actor(&self, token_text: &str) -> Result<Actor, AppError> {
        if let Some(acting) = &self.acting {
            return Ok(acting.clone());
        }
        if looks_like_access_key(token_text) {
            return self.key_actor(token_text).await;
        }
        self.session_actor(token_text).await
    }

    /// The signed-in person behind a cookie as the actor, never the owner of an access key: what
    /// belongs to an open editor (a lock, a draft) belongs to a person in a browser tab (ADR-0019).
    pub(crate) async fn session_actor(&self, token_text: &str) -> Result<Actor, AppError> {
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
    /// `resource.not-found` when the place is a project that does not exist.
    pub(crate) async fn evaluation_of(
        &self,
        actor: &Actor,
        place: Place,
    ) -> Result<Arc<Evaluation>, AppError> {
        let owners = self.evaluate(actor.person.id, place).await?;
        Ok(owners.narrowed_by(actor.key_scope()))
    }

    /// Whether `person` may see `issue` in the project it is in: the question of who may be told
    /// about it (a mention, a notice), judged on the person and not on the actor of the request.
    /// It is the same rule as for naming a person on the issue (`may_be_assigned`, which
    /// `may_be_named` and `may_name` build on), so there is one statement of who may see an issue.
    /// It does not ask whether the writer may name the person (`may_name_person`): a mention is a
    /// name the writer typed already, and only the people who may see the issue are told.
    ///
    /// # Errors
    /// An internal error when the engine fails.
    pub(crate) async fn person_sees_issue(
        &self,
        person: UserId,
        issue: &bb_core::issue::Issue,
    ) -> Result<bool, AppError> {
        self.may_be_assigned(person, issue.project, issue.reported_by)
            .await
    }

    /// What `person` may do at `place`, from the policy engine. The result is kept for the rest
    /// of the request, so asking again costs nothing; the next request asks again.
    ///
    /// # Errors
    /// `resource.not-found` when the place is a project that does not exist.
    pub(crate) async fn evaluate(
        &self,
        person: UserId,
        place: Place,
    ) -> Result<Arc<Evaluation>, AppError> {
        if let Some(found) = self.cached_evaluation(person, place) {
            return Ok(found);
        }
        let evaluation = Arc::new(
            evaluation_in(self.store(), person, place)
                .await?
                .ok_or(AppError::Refused(ErrorCode::NotFound))?,
        );
        self.remember_evaluation(person, place, &evaluation);
        Ok(evaluation)
    }

    /// Keeps the evaluation of `person` at each of `projects` for the rest of the request, from
    /// one read of their rows, the levels of their roles and the tree of the projects. Each
    /// `evaluate` of a place reads all of that again (some fifteen statements), which a page of
    /// notifications in many projects pays for every project; the decision is the same
    /// ([`Policy::evaluate_with`] is what a preview of a move asks too). Nothing is read when
    /// fewer than two of the places are not kept yet. What was read is given back, so that the
    /// caller does not read the projects once more.
    ///
    /// # Errors
    /// An internal error when the store fails.
    pub(crate) async fn evaluate_projects(
        &self,
        person: UserId,
        projects: &[ProjectId],
    ) -> Result<Option<Leveled>, AppError> {
        let mut wanted: Vec<ProjectId> = projects
            .iter()
            .copied()
            .filter(|project| {
                self.cached_evaluation(person, Place::Project(*project))
                    .is_none()
            })
            .collect();
        wanted.sort();
        wanted.dedup();
        if wanted.len() < 2 {
            return Ok(None);
        }
        let shown = self.shown_with_levels(person, None).await?;
        for (project, found) in shown.evaluations_in(&wanted) {
            self.remember_evaluation(person, Place::Project(project), &Arc::new(found));
        }
        Ok(Some(shown))
    }

    fn cached_evaluation(&self, person: UserId, place: Place) -> Option<Arc<Evaluation>> {
        // A lock that a panic poisoned still holds a map that is whole: it only ever gets whole
        // insertions.
        let cache = locked(&self.evaluations);
        cache.get(&(person, place)).cloned()
    }

    fn remember_evaluation(&self, person: UserId, place: Place, evaluation: &Arc<Evaluation>) {
        let mut cache = locked(&self.evaluations);
        cache.insert((person, place), Arc::clone(evaluation));
    }

    /// The evaluation of the actor at `place`, when they hold `permission` there.
    ///
    /// # Errors
    /// `resource.not-found` for a project the person may not know about (or that does not exist),
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

    /// The person with this ID, looked up once for the request.
    pub(crate) async fn person_known(&self, id: UserId) -> Result<Option<Person>, AppError> {
        let known = locked(&self.known.people).get(&id).cloned();
        if let Some(known) = known {
            return Ok(known);
        }
        let found = self
            .store()
            .person(id)
            .await
            .map_err(|error| internal::<S>(&error))?;
        locked(&self.known.people).insert(id, found.clone());
        Ok(found)
    }

    /// Reads the people with these IDs in one go, so the lookups of each of them one by one
    /// (`person_known`) find them read already.
    pub(crate) async fn preload_people(&self, ids: &[UserId]) -> Result<(), AppError> {
        let wanted: Vec<UserId> = {
            let known = locked(&self.known.people);
            ids.iter()
                .copied()
                .filter(|id| !known.contains_key(id))
                .collect()
        };
        if wanted.is_empty() {
            return Ok(());
        }
        let found = self
            .store()
            .people_of(&wanted)
            .await
            .map_err(|error| internal::<S>(&error))?;
        let mut known = locked(&self.known.people);
        for id in wanted {
            let person = found.iter().find(|person| person.id == id).cloned();
            known.insert(id, person);
        }
        Ok(())
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
        let Some(found) = self.person_known(person).await? else {
            return Ok(PersonRef::Someone);
        };
        // The role label is the person's own role at the place the viewer looks at. It is only
        // shown to a viewer who sees people by their role, so only then is it looked for: the
        // evaluation of the rights of each different person on a page was the biggest cost of a
        // page of notifications (M3-F113, PERF-01).
        let role = if viewer.shows_role_label(kind, context, person) {
            if_there(self.evaluate(person, viewer.place()).await)?
                .and_then(|theirs| theirs.first_role_name().map(str::to_owned))
        } else {
            None
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

    /// The place as the actor may name it: the project's label, or `None` for the whole
    /// organization.
    pub(crate) async fn place_label(
        &self,
        actor: &Actor,
        place: impl Into<Place>,
    ) -> Result<Option<ProjectLabel>, AppError> {
        match place.into() {
            Place::Organization => Ok(None),
            Place::Project(project) => self.project_label(actor, project).await.map(Some),
        }
    }

    /// The project as the actor may name it.
    pub(crate) async fn project_label(
        &self,
        actor: &Actor,
        project: ProjectId,
    ) -> Result<ProjectLabel, AppError> {
        let Some(evaluation) = if_there(self.evaluation_of(actor, Place::Project(project)).await)?
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
            .as_ref()
            .map_or(ProjectLabel::Hidden, ProjectLabel::of))
    }
}

/// What `person` holds at `place`, read inside a change after the policy lock: what a rule of the
/// change is judged on, not what was stored when the request began. `None` when the place is gone.
///
/// # Errors
/// The error of the store.
async fn held_if_there_in<S: AuthStore>(
    transaction: &mut S::Transaction,
    person: UserId,
    place: Place,
) -> Result<Option<Evaluation>, S::Error> {
    Ok(S::policy_inputs_in(transaction, person, place)
        .await?
        .map(|inputs| evaluate(&inputs)))
}

/// [`held_if_there_in`], for a rule that holds nobody to a place that is gone.
///
/// It evaluates the person without any access key, so every caller must be a use case that needs
/// a permission in [`NOT_FOR_KEYS`] (a key never reaches it); the test
/// `what_the_change_rules_judge_without_the_key_needs_a_permission_keys_never_hold` lists them.
///
/// # Errors
/// [`Refusal::TooMuch`] (as the error of the store) when the place is gone, or the error of the
/// store.
pub(crate) async fn held_in<S: AuthStore>(
    transaction: &mut S::Transaction,
    person: UserId,
    place: Place,
) -> Result<Evaluation, S::Error> {
    held_if_there_in::<S>(transaction, person, place)
        .await?
        .ok_or_else(|| S::refused(Refusal::TooMuch))
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
/// what was stored when the request began. It evaluates the giver without any access key, so every
/// caller must be a use case that needs a permission in [`NOT_FOR_KEYS`] (see [`held_in`]).
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
    let mut places = vec![Place::from(row.project)];
    let reaches_below = row.applies_to == AppliesTo::ProjectAndSubProjects;
    if let (Some(project), true) = (row.project, reaches_below) {
        places.extend(
            sub_projects_in::<S>(transaction, project)
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

/// Whether `actor` may lift the *No access* rows of `group`, which taking people out of the group
/// or deleting it does for them: they must decide who has which role in each project where it has
/// one, as for taking the row itself away (ADR-0025, decision 6). It runs inside the change, after
/// the policy lock, so it judges what is stored now.
///
/// # Errors
/// [`Refusal::CannotLift`] (as the error of the store) when they may not decide in such a project
/// (or the project is gone), or the error of the store.
pub(crate) async fn ensure_may_lift_in<S: AuthStore>(
    transaction: &mut S::Transaction,
    actor: UserId,
    group: GroupId,
) -> Result<(), S::Error> {
    for row in S::assignments_of_group_in(transaction, group).await? {
        let (None, Some(project)) = (row.role, row.project) else {
            continue;
        };
        let may = held_if_there_in::<S>(transaction, actor, Place::Project(project))
            .await?
            .is_some_and(|held| held.allows(Permission::DecideRoles));
        if !may {
            return Err(S::refused(Refusal::CannotLift));
        }
    }
    Ok(())
}

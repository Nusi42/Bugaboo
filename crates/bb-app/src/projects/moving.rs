//! Moving a project with a preview (PRJ-03, *Show, then do*).
//!
//! Moving project X puts X and everything in it under another parent. Before it happens, a
//! person looks at the preview: who gains or loses access or changes role, and which fields and
//! kinds of people information change for whom. It is the difference of two evaluations of the
//! same engine, one for the tree as it is and one for the tree as it would be
//! ([`bb_policy::preview_move`]). The move itself runs inside the change, after the policy lock,
//! and judges the same thing again on what is stored now:
//!
//! - the person still may move in the old and in the new parent, and sees the project;
//! - the project does not go inside itself;
//! - **no *No access* is lifted** (ADR-0025, decision 6, M2-F23): a move after which somebody
//!   could take away a *No access* that applies to them, and could not before, is refused;
//! - the preview the person looked at is still the preview of the project, the two parents, who
//!   has which role and the levels the roles set: a tree that changed meanwhile asks the person to
//!   look again.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use bb_core::ErrorCode;
use bb_core::event::{FieldChange, RecordKind};
use bb_core::id::ProjectId;
use bb_core::people_kinds::PeopleKind;
use bb_core::project::Project;
use bb_core::sign_in::SecurityAction;
use bb_core::version::{counted_up, next};
use bb_policy::{
    Access, Authorized, Change, Evaluation, MoveRefusal, MoveRequest, PeopleContext, Place, Policy,
    Preview, Tree, check_move, preview_move as difference,
};
use serde_json::json;

use super::Why;
use super::types::{
    FieldLevelChange, LiftView, MovePreview, MovePreviewQuery, MoveProject, PREVIEW_CHANGES_MAX,
    PREVIEW_PEOPLE_MAX, PeopleLevelChange, PersonChange, ProjectChange, ProjectRef, ProjectView,
    Standing,
};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, by, entity, internal};

/// The version of a preview: it names the project, both parents and what the move would change,
/// so it is another one as soon as one of them changes. A digest, not a secret: the server makes
/// it again when the person moves, and only compares.
fn version_of(
    projects: &BTreeMap<ProjectId, Project>,
    project: &Project,
    to: Option<ProjectId>,
    preview: &Preview,
) -> String {
    let version = |id: Option<ProjectId>| {
        id.and_then(|id| projects.get(&id))
            .map(|found| found.version)
    };
    let text = format!(
        "{} {} {:?} {:?} {:?} {preview:?}",
        project.id,
        project.version,
        project.parent.map(|parent| (parent, version(Some(parent)))),
        to.map(|parent| (parent, version(Some(parent)))),
        to.and_then(|parent| projects.get(&parent))
            .map(|found| &found.name),
    );
    bb_core::hex::encode(&bb_auth::stable_id("move-preview", &text))
}

/// A move that may be asked for, with what it is judged on.
struct Checked {
    project: Project,
    viewer: Arc<Evaluation>,
    projects: Vec<Project>,
    policy: Policy,
}

/// What a refused move says.
const fn code_of(refusal: MoveRefusal) -> ErrorCode {
    match refusal {
        MoveRefusal::NotFound => ErrorCode::NotFound,
        MoveRefusal::Forbidden => ErrorCode::Forbidden,
        MoveRefusal::IntoItself => ErrorCode::MoveIntoItself,
        MoveRefusal::SameParent => ErrorCode::InvalidRequest,
        MoveRefusal::TooDeep => ErrorCode::ProjectTooDeep,
    }
}

/// What a move would do, from the tree and the policy as they are.
struct Judged {
    tree: Tree,
    preview: Preview,
    version: String,
}

fn judge(
    policy: &Policy,
    projects: &[Project],
    project: &Project,
    to: Option<ProjectId>,
) -> Judged {
    let tree = Tree::of(projects);
    let preview = difference(policy, &tree, project.id, to);
    let by_id: BTreeMap<ProjectId, Project> = projects
        .iter()
        .map(|known| (known.id, known.clone()))
        .collect();
    Judged {
        version: version_of(&by_id, project, to, &preview),
        tree,
        preview,
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// What a person has to be allowed before a move is even looked at (the one set of rules of
    /// [`check_move`], the same that the move itself is held to inside its change): they see the
    /// project, they may move in the new parent and in the old one, and it does not go inside
    /// itself. A project the person may not see, and a parent they may not know about, answer like
    /// ones that do not exist. Gives the project, the way the person sees it, and what the
    /// preview and the move are judged on.
    async fn checked_move(
        &self,
        actor: &Actor,
        project: ProjectId,
        to: Option<ProjectId>,
    ) -> Result<Checked, AppError> {
        let viewer = self.evaluation_of(actor, Place::Project(project)).await?;
        let projects = self
            .store()
            .project_tree()
            .await
            .map_err(|error| internal::<S>(&error))?;
        let policy = self
            .store()
            .policy_all()
            .await
            .map_err(|error| internal::<S>(&error))?;
        let asked = MoveRequest {
            person: actor.person.id,
            key: actor.key_scope(),
            project,
            to,
        };
        check_move(&policy, &Tree::of(&projects), &asked)
            .map_err(|refusal| AppError::Refused(code_of(refusal)))?;
        let found = projects
            .iter()
            .find(|known| known.id == project)
            .cloned()
            .ok_or(AppError::Refused(ErrorCode::NotFound))?;
        Ok(Checked {
            project: found,
            viewer,
            projects,
            policy,
        })
    }

    /// The preview as the person who asks may read it: people as they may see them, and only
    /// the projects they see.
    async fn move_preview_view(
        &self,
        actor: &Actor,
        viewer: &Evaluation,
        project: &Project,
        to: Option<&Project>,
        judged: &Judged,
        policy: &Policy,
    ) -> Result<MovePreview, AppError> {
        let tree = &judged.tree;
        let seen: BTreeSet<ProjectId> = policy.visible_as(actor.person.id, actor.key_scope(), tree);
        let projects: BTreeMap<ProjectId, Project> = {
            let all = self
                .store()
                .project_tree()
                .await
                .map_err(|error| internal::<S>(&error))?;
            all.into_iter().map(|known| (known.id, known)).collect()
        };
        let shown_ref = |id: ProjectId| {
            seen.contains(&id)
                .then(|| projects.get(&id).map(ProjectRef::from))
                .flatten()
        };
        let mut people: Vec<PersonChange> = Vec::new();
        let field_names: std::collections::HashMap<bb_core::id::FieldId, String> = self
            .field_catalogue()
            .await?
            .iter()
            .map(|definition| (definition.field.id, definition.field.name.clone()))
            .collect();
        let mut truncated = false;
        let mut listed = 0_usize;
        let mut at = 0;
        while at < judged.preview.changes.len() {
            let person = judged.preview.changes[at].person;
            let end = judged.preview.changes[at..]
                .iter()
                .position(|change| change.person != person)
                .map_or(judged.preview.changes.len(), |offset| at + offset);
            let mut rows = Vec::new();
            for change in &judged.preview.changes[at..end] {
                if let Some(named) = shown_ref(change.project) {
                    rows.push(project_change(named, change, &field_names));
                }
            }
            at = end;
            if rows.is_empty() {
                continue;
            }
            if people.len() >= PREVIEW_PEOPLE_MAX || listed + rows.len() > PREVIEW_CHANGES_MAX {
                truncated = true;
                break;
            }
            listed += rows.len();
            people.push(PersonChange {
                person: self
                    .person_ref(
                        viewer,
                        PeopleKind::ProjectMembers,
                        PeopleContext::Management,
                        person,
                    )
                    .await?,
                projects: rows,
            });
        }
        let mut lifts = Vec::new();
        for lift in &judged.preview.lifts {
            if let Some(named) = shown_ref(lift.project) {
                lifts.push(LiftView {
                    person: self
                        .person_ref(
                            viewer,
                            PeopleKind::ProjectMembers,
                            PeopleContext::Management,
                            lift.person,
                        )
                        .await?,
                    project: named,
                });
            }
        }
        Ok(MovePreview {
            project: ProjectRef::from(project),
            to: to.and_then(|target| shown_ref(target.id)),
            preview: judged.version.clone(),
            blocked: !judged.preview.lifts.is_empty(),
            blocked_out_of_sight: judged
                .preview
                .lifts
                .iter()
                .any(|lift| !seen.contains(&lift.project)),
            people,
            lifts,
            truncated,
        })
    }
}

/// What a person has in a project, from what the engine says.
fn standing(access: &Access) -> Standing {
    Standing {
        sees: access.sees,
        held_back: access.held_back,
        roles: access.roles.clone(),
    }
}

/// What one change of the engine says, as a line of the preview.
fn project_change(
    project: ProjectRef,
    change: &Change,
    field_names: &std::collections::HashMap<bb_core::id::FieldId, String>,
) -> ProjectChange {
    let (before, after) = (&change.before, &change.after);
    ProjectChange {
        project,
        now: standing(before),
        after: standing(after),
        gains: after
            .permissions
            .difference(&before.permissions)
            .copied()
            .collect(),
        loses: before
            .permissions
            .difference(&after.permissions)
            .copied()
            .collect(),
        fields: before
            .fields
            .iter()
            .filter_map(|(field, level)| {
                let now = after.fields.get(field)?;
                (now != level).then_some(FieldLevelChange {
                    field: *field,
                    name: field.own().and_then(|id| field_names.get(&id).cloned()),
                    before: *level,
                    after: *now,
                })
            })
            .collect(),
        people: before
            .people
            .iter()
            .filter_map(|(kind, level)| {
                let now = after.people.get(kind)?;
                (now != level).then_some(PeopleLevelChange {
                    kind: *kind,
                    before: *level,
                    after: *now,
                })
            })
            .collect(),
    }
}

/// What moving a project would do, before anything happens: who gains or loses access or changes
/// role, which fields and kinds of people information change for whom, and whether the move
/// would be refused. The answer holds the *version* of the preview, which the move asks for.
///
/// # Errors
/// `resource.not_found` for a project or a new parent the person may not know about,
/// `access.forbidden` without *Move projects* in the old and the new parent,
/// `project.move_into_itself` for a move below itself, `project.too_deep` for a move that pushes
/// what is inside past the depth of a tree, `request.invalid` for a move to where the
/// project is.
pub async fn preview_move<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: MovePreviewQuery,
) -> Result<Authorized<MovePreview>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let Checked {
        project,
        viewer,
        projects,
        policy,
    } = ctx.checked_move(&actor, input.project, input.to).await?;
    let target = input
        .to
        .and_then(|id| projects.iter().find(|known| known.id == id));
    let judged = judge(&policy, &projects, &project, input.to);
    let view = ctx
        .move_preview_view(&actor, &viewer, &project, target, &judged, &policy)
        .await?;
    Ok(viewer.authorize(view))
}

/// Moves a project, if nothing the preview was made from has changed. A move after which somebody
/// could take away a *No access* that applies to them, and could not before, is refused.
///
/// # Errors
/// What [`preview_move`] answers, `project.preview_changed` when the projects, who has which role
/// or the levels of the roles changed since the preview, `project.move_lifts_no_access` for a
/// move that lifts a *No access*, `edit.version_changed` when somebody changed the project first.
pub async fn move_project<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: MoveProject,
) -> Result<Authorized<ProjectView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let Checked {
        project, viewer, ..
    } = ctx.checked_move(&actor, input.project, input.to).await?;
    let shown_parent = match input.to {
        Some(to) => ctx
            .evaluation_of(&actor, Place::Project(to))
            .await?
            .sees_project()
            .then_some(to),
        None => None,
    };
    let id = project.id;
    let (to, person, expected) = (input.to, actor.person.id, input.preview);
    let key = actor.key_scope().cloned();
    let event = ctx.event(
        by(person),
        entity(RecordKind::Project, id),
        vec![
            FieldChange {
                field: "parent".to_owned(),
                before: Some(json!(project.parent.map(|parent| parent.to_string()))),
                after: Some(json!(to.map(|parent| parent.to_string()))),
            },
            counted_up(project.version),
        ],
    )?;
    let why = Why::default();
    let stopped = why.clone();
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::lock_policy(transaction).await?;
                // Everything is judged again on what is stored now, after the lock: who has
                // which role, the tree, and what the person looked at.
                let Some(now) = S::project_in(transaction, id).await? else {
                    return Err(stopped.stop::<S>(ErrorCode::NotFound));
                };
                let projects = S::project_tree_in(transaction).await?;
                let policy = S::policy_in(transaction).await?;
                let judged = judge(&policy, &projects, &now, to);
                let asked = MoveRequest {
                    person,
                    key: key.as_ref(),
                    project: id,
                    to,
                };
                check_move(&policy, &judged.tree, &asked)
                    .map_err(|refusal| stopped.stop::<S>(code_of(refusal)))?;
                // The rule of M2-F23: a move lifts no *No access*. It comes before the look at
                // the preview, so no preview, stale or made up, gets around it.
                if !judged.preview.lifts.is_empty() {
                    return Err(stopped.stop::<S>(ErrorCode::MoveLiftsNoAccess));
                }
                if judged.version != expected {
                    return Err(stopped.stop::<S>(ErrorCode::MovePreviewChanged));
                }
                if S::move_project(transaction, id, to, now.version).await? {
                    Ok(())
                } else {
                    Err(stopped.stop::<S>(ErrorCode::VersionChanged))
                }
            })
        })
        .await
        .map_err(|error| why.answer::<S>(&error, ErrorCode::VersionChanged))?;
    ctx.note(
        SecurityAction::ProjectMoved,
        Some(actor.person.id),
        Some(entity(RecordKind::Project, id)),
    )
    .await;
    Ok(viewer.authorize(ProjectView {
        id,
        parent: shown_parent,
        short_name: project.short_name,
        name: project.name,
        version: next(project.version),
    }))
}

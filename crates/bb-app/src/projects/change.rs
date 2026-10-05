//! Changing a project: its name, and its short name (PRJ-01, M2-Q4). The old short name stays with
//! the project, so its old addresses keep working, and only that project can take it back.

use std::sync::Arc;

use bb_core::ErrorCode;
use bb_core::event::FieldChange;
use bb_core::project::{Project, ProjectAlias, ShortName};
use bb_core::sign_in::SecurityAction;
use bb_policy::{Authorized, Evaluation, Permission, Place};
use serde_json::json;

use super::Why;
use super::types::{ChangeProject, ProjectView};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, by, entity, internal};
use crate::people::clean_name;

/// What a change of a project asks for, checked against what the person may do and what is stored.
struct Plan {
    /// The project as it is.
    current: Project,
    /// What it will be called.
    name: String,
    /// The short name it will have.
    short_name: ShortName,
}

impl Plan {
    fn renames(&self) -> bool {
        self.name != self.current.name
    }

    fn changes_short_name(&self) -> bool {
        self.short_name != self.current.short_name
    }

    /// What the event says: the fields that change, before and after.
    fn field_changes(&self) -> Vec<FieldChange> {
        let mut fields = Vec::new();
        if self.renames() {
            fields.push(FieldChange {
                field: "name".to_owned(),
                before: Some(json!(self.current.name)),
                after: Some(json!(self.name)),
            });
        }
        if self.changes_short_name() {
            fields.push(FieldChange {
                field: "short_name".to_owned(),
                before: Some(json!(self.current.short_name.as_str())),
                after: Some(json!(self.short_name.as_str())),
            });
        }
        fields
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// What the person asks for, when they may ask it: the right to change the project, the right
    /// to make projects where it sits for a short name (only people who may make projects are
    /// told that a short name is taken), the version they looked at.
    async fn plan_change(
        &self,
        actor: &Actor,
        input: &ChangeProject,
    ) -> Result<(Arc<Evaluation>, Plan), AppError> {
        let id = input.project;
        let access = self
            .require(actor, Permission::ChangeProjectSettings, Place::Project(id))
            .await?;
        if !access.sees_project() {
            return Err(AppError::Refused(ErrorCode::NotFound));
        }
        if input.name.is_none() && input.short_name.is_none() {
            return Err(ErrorCode::InvalidRequest.into());
        }
        let name = input.name.as_deref().map(clean_name).transpose()?;
        let short_name = input
            .short_name
            .as_deref()
            .map(|text| ShortName::new(text.trim()).map_err(|_| ErrorCode::InvalidRequest))
            .transpose()?;
        let current = self
            .store()
            .project(id)
            .await
            .map_err(|error| internal::<S>(&error))?
            .ok_or(AppError::Refused(ErrorCode::NotFound))?;
        if current.version != input.version {
            return Err(ErrorCode::VersionChanged.into());
        }
        let plan = Plan {
            name: name.unwrap_or_else(|| current.name.clone()),
            short_name: short_name.unwrap_or_else(|| current.short_name.clone()),
            current,
        };
        if plan.changes_short_name()
            && !self
                .may_make_projects_beside(actor, plan.current.parent)
                .await?
        {
            return Err(ErrorCode::Forbidden.into());
        }
        Ok((access, plan))
    }
}

/// Changes the name or the short name of a project. A short name that is changed leaves the old
/// one with the project (`/p/OLD` keeps opening it).
///
/// # Errors
/// `access.forbidden` without *Change the project's name, short name and settings* (and, for a
/// short name, without *Make projects* or *Make sub-projects* where the project sits: only
/// people who may make projects are told that a short name is taken), `resource.not_found` for a
/// project the person may not know about, `request.invalid` for a name or short name that
/// cannot be one or when nothing is asked, `edit.version_changed` when somebody changed the
/// project since it was looked at, `name.taken` for a short name that is used by another project
/// now or was once.
pub async fn change_project<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ChangeProject,
) -> Result<Authorized<ProjectView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let (access, plan) = ctx.plan_change(&actor, &input).await?;
    let shown = ctx.shown_to(actor.person.id).await?;
    let view = |version: i64| ProjectView {
        id: plan.current.id,
        parent: shown.shown_parent(&plan.current),
        short_name: plan.short_name.clone(),
        name: plan.name.clone(),
        version,
    };
    let field_changes = plan.field_changes();
    if field_changes.is_empty() {
        return Ok(access.authorize(view(plan.current.version)));
    }
    let id = plan.current.id;
    let event = ctx.event(by(actor.person.id), entity("project", id)?, field_changes)?;
    let why = Why::default();
    let stopped = why.clone();
    let (renames, retired) = (plan.changes_short_name(), plan.current.short_name.clone());
    let (name, short_name, version) = (
        plan.name.clone(),
        plan.short_name.clone(),
        plan.current.version,
    );
    let retired_at = ctx.now();
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::lock_policy(transaction).await?;
                // Everything below is judged on what is stored now: somebody may have changed
                // the project, or taken the short name, since the request began.
                let Some(now) = S::project_in(transaction, id).await? else {
                    return Err(stopped.stop::<S>(ErrorCode::NotFound));
                };
                if now.version != version {
                    return Err(stopped.stop::<S>(ErrorCode::VersionChanged));
                }
                if renames {
                    match S::short_name_owner_in(transaction, &short_name).await? {
                        Some(owner) if owner != id => {
                            return Err(stopped.stop::<S>(ErrorCode::NameTaken));
                        }
                        // The project takes back a short name it had before.
                        Some(_) => S::delete_project_alias(transaction, &short_name).await?,
                        None => {}
                    }
                    let alias = ProjectAlias {
                        short_name: retired,
                        project: id,
                        retired_at,
                    };
                    S::insert_project_alias(transaction, &alias).await?;
                }
                if S::update_project(transaction, id, &name, &short_name, version).await? {
                    Ok(())
                } else {
                    Err(stopped.stop::<S>(ErrorCode::VersionChanged))
                }
            })
        })
        .await
        .map_err(|error| why.answer::<S>(&error, ErrorCode::NameTaken))?;
    if plan.changes_short_name() {
        ctx.note(
            SecurityAction::ProjectShortNameChanged,
            Some(actor.person.id),
            Some(entity("project", id)?),
        )
        .await;
    }
    if plan.renames() {
        ctx.note(
            SecurityAction::ProjectRenamed,
            Some(actor.person.id),
            Some(entity("project", id)?),
        )
        .await;
    }
    Ok(access.authorize(view(plan.current.version + 1)))
}

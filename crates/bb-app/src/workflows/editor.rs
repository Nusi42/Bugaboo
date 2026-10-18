//! Reading workflows for an editor: the workflow of a place with the roles, groups and fields its
//! rules can name, and the list of workflows to choose from (WF-02, ADR-0031).
//!
//! Everything here needs *Manage workflows* at the place. What the editor offers is limited to
//! what the person may know: the groups only for somebody who manages groups, the own fields only
//! the ones they may know at the place.

use bb_core::fields::{Field, FieldKey};
use bb_core::id::{GroupId, IssueTypeId, ProjectId};
use bb_core::own_fields::FieldFor;
use bb_core::workflow::{Workflow, WorkflowContent, WorkflowPlace};
use bb_policy::{Authorized, Evaluation, Permission, Place};

use super::resolve::InForce;
use super::types::{
    EditorChange, EditorGroup, EditorRole, RuleField, StatusView, WORKFLOWS_LISTED_MAX,
    WorkflowEditor, WorkflowEditorQuery, WorkflowList, WorkflowListItem, WorkflowSlot,
    WorkflowsQuery,
};
use crate::access::{Actor, ROLES_MAX};
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::editing::EditorPlace;

/// The place a workflow is asked for.
pub(super) type Where = EditorPlace;

impl Where {
    /// Where a workflow for exactly this place is used.
    pub(super) fn workflow_place(&self) -> WorkflowPlace {
        WorkflowPlace {
            project: self.project.as_ref().map(|project| project.id),
            issue_type: self.issue_type,
        }
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The place of a request for an editor: the project must exist for the person.
    pub(super) async fn workflow_where(
        &self,
        project: Option<&str>,
        issue_type: Option<IssueTypeId>,
    ) -> Result<Where, AppError> {
        self.editor_place(project, issue_type, ()).await
    }

    /// The groups *Who can do it* can name: all of them for somebody who manages groups, and the
    /// ones the workflow names already for anybody else.
    pub(super) async fn groups_offered(
        &self,
        actor: &Actor,
        content: &WorkflowContent,
    ) -> Result<Vec<EditorGroup>, AppError> {
        let organization = self.evaluation_of(actor, Place::Organization).await?;
        let named: Vec<GroupId> = content
            .changes
            .iter()
            .flat_map(|change| change.rules.who.groups.iter().copied())
            .collect();
        let every = organization.allows(Permission::ManageGroups);
        Ok(self
            .store()
            .groups()
            .await
            .map_err(|error| internal::<S>(&error))?
            .into_iter()
            .filter(|group| every || named.contains(&group.id))
            .map(|group| EditorGroup {
                id: group.id,
                name: group.name,
            })
            .collect())
    }

    /// The fields the rules can name at a place: the built-in fields a rule can ask about, and the
    /// own fields of issues that the person may know there.
    async fn rule_fields(
        &self,
        actor: &Actor,
        held: &Evaluation,
        project: Option<ProjectId>,
    ) -> Result<Vec<RuleField>, AppError> {
        let names = self.known_field_names(actor, held, project).await?;
        let mut fields: Vec<RuleField> = [Field::Description, Field::AssignedTo]
            .into_iter()
            .map(|field| RuleField {
                field: FieldKey::BuiltIn(field),
                name: None,
            })
            .collect();
        for definition in self.field_catalogue().await?.iter() {
            let key = FieldKey::Own(definition.field.id);
            if definition.field.applies_to == FieldFor::Issues
                && definition.field.retired_at.is_none()
                && names.contains_key(&key)
            {
                fields.push(RuleField {
                    field: key,
                    name: names.get(&key).cloned(),
                });
            }
        }
        Ok(fields)
    }

    /// The editor view of a workflow that counts at a place (or is being edited from it).
    pub(super) async fn editor_of(
        &self,
        actor: &Actor,
        held: &Evaluation,
        at: &Where,
        in_force: &InForce,
    ) -> Result<WorkflowEditor, AppError> {
        let content = &in_force.content;
        let slot = in_force
            .used
            .as_ref()
            .zip(in_force.workflow.as_ref())
            .filter(|(used, _)| used.place() == at.workflow_place())
            .map(|(_, workflow)| WorkflowSlot {
                id: workflow.id,
                version: workflow.version,
            });
        let inherited_from = match (&in_force.used, &slot) {
            (Some(used), None) => Some(
                self.inherited_from_place(actor, used.project, used.issue_type)
                    .await?,
            ),
            _ => None,
        };
        let roles = self
            .store()
            .roles()
            .await
            .map_err(|error| internal::<S>(&error))?
            .into_iter()
            .take(ROLES_MAX)
            .map(|record| EditorRole {
                id: record.role.id,
                name: record.role.name,
            })
            .collect();
        Ok(WorkflowEditor {
            project: at.project.clone(),
            issue_type: match at.issue_type {
                Some(id) => self.issue_type_ref(actor, id).await?,
                None => None,
            },
            workflow: slot,
            inherited_from,
            name: in_force
                .workflow
                .as_ref()
                .map_or_else(starting_name, |workflow| workflow.name.clone()),
            statuses: content.statuses.iter().map(StatusView::from).collect(),
            changes: content
                .changes
                .iter()
                .map(|change| EditorChange {
                    from: change.from.clone(),
                    to: change.to.clone(),
                    button: change.button.clone(),
                    rules: change.rules.clone(),
                })
                .collect(),
            roles,
            groups: self.groups_offered(actor, content).await?,
            fields: self
                .rule_fields(actor, held, at.project.as_ref().map(|project| project.id))
                .await?,
        })
    }

    /// The editor view of the workflow that counts at a place.
    pub(super) async fn editor_at(
        &self,
        actor: &Actor,
        held: &Evaluation,
        at: &Where,
    ) -> Result<WorkflowEditor, AppError> {
        let directory = self.workflow_directory().await?;
        let in_force = directory.detail(&at.chain, at.issue_type);
        self.editor_of(actor, held, at, &in_force).await
    }
}

/// The name of the starting workflow, in the language of the catalog.
pub(super) fn starting_name() -> String {
    bb_core::workflow::starter::SIMPLE.title()
}

/// Shows the workflow of a place to somebody who may change it: the workflow that counts there,
/// with the roles, groups and fields its rules can name. Needs *Manage workflows* at the place.
///
/// # Errors
/// `resource.not_found` for a project the person may not know about, `access.forbidden` without
/// the permission.
pub async fn workflow_editor<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: WorkflowEditorQuery,
) -> Result<Authorized<WorkflowEditor>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    ctx.ensure_type_known(&actor, input.issue_type).await?;
    let at = ctx
        .workflow_where(input.project.as_deref(), input.issue_type)
        .await?;
    let held = ctx
        .require(&actor, Permission::ManageWorkflows, at.place())
        .await?;
    let view = ctx.editor_at(&actor, &held, &at).await?;
    Ok(held.authorize(view))
}

/// The workflows there are, by name, for choosing one for a place. Needs *Manage workflows* at the
/// place.
///
/// # Errors
/// `resource.not_found` for a project the person may not know about, `access.forbidden` without
/// the permission.
pub async fn workflows<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: WorkflowsQuery,
) -> Result<Authorized<WorkflowList>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let at = ctx.workflow_where(input.project.as_deref(), None).await?;
    let held = ctx
        .require(&actor, Permission::ManageWorkflows, at.place())
        .await?;
    // Only workflows the person could put to use at this place: the ones that sit at the place or
    // above it. A workflow of a project they do not manage elsewhere is not listed.
    let mut headers = Vec::new();
    for header in ctx
        .store()
        .workflow_headers()
        .await
        .map_err(|error| internal::<S>(&error))?
    {
        if ctx.may_adopt_workflow(&actor, header.id, &at.chain).await? {
            headers.push(header);
        }
    }
    let truncated = headers.len() > WORKFLOWS_LISTED_MAX;
    let workflows = headers
        .into_iter()
        .take(WORKFLOWS_LISTED_MAX)
        .map(|Workflow { id, name, version }| WorkflowListItem { id, name, version })
        .collect();
    Ok(held.authorize(WorkflowList {
        workflows,
        truncated,
    }))
}

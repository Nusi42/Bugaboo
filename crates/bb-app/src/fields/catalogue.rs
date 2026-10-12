//! Reading the catalogue of own fields (CF-01, CF-06): the list a person sees, one field, and
//! where a field is used.
//!
//! A field is shown to a person who may see it in some issue type, or who manages fields. A field
//! that is hidden from them is not in any answer, and asking for it answers `resource.not_found`,
//! exactly like a field that does not exist.

use std::collections::HashMap;

use bb_core::ErrorCode;
use bb_core::fields::FieldKey;
use bb_core::id::{FieldId, ProjectId};
use bb_core::issue::IssueType;
use bb_core::own_fields::{FieldType, Scope, USAGE_PROJECTS_MAX};
use bb_policy::{Authorized, Evaluation, Permission, Place};

use super::manage::current_default;
use super::types::{
    ChoiceView, Definition, FieldPage, FieldQuery, FieldTypeInfo, FieldTypePage, FieldTypesQuery,
    FieldUsage, FieldView, FieldsQuery, ScopeView,
};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::issues::IssueTypeRef;
use crate::projects::ProjectRef;

/// The projects the viewer may know about, found once for a whole answer.
type Seen = HashMap<ProjectId, Option<ProjectRef>>;

impl<S: AuthStore> Ctx<'_, S> {
    /// The project as the viewer may see it: `None` when it is not there or they may not see it.
    pub(super) async fn visible_project(
        &self,
        actor: &Actor,
        project: ProjectId,
        seen: &mut Seen,
    ) -> Result<Option<ProjectRef>, AppError> {
        if let Some(known) = seen.get(&project) {
            return Ok(known.clone());
        }
        let found = match self.store().project(project).await {
            Ok(Some(found)) => {
                let held = self.evaluation_of(actor, Place::Project(project)).await?;
                held.sees_project().then(|| ProjectRef::from(&found))
            }
            Ok(None) => None,
            Err(error) => return Err(internal::<S>(&error)),
        };
        seen.insert(project, found.clone());
        Ok(found)
    }

    /// Whether the viewer may know the field at a place: they may see it somewhere (or manage
    /// fields), and it reaches the place. `chain` is the projects from the top down to the place,
    /// or none for the organization, where a field limited to projects counts for a viewer who
    /// sees one of them (or manages fields).
    pub(crate) async fn field_in_view(
        &self,
        actor: &Actor,
        held: &Evaluation,
        chain: Option<&[ProjectId]>,
        definition: &Definition,
        seen: &mut Seen,
    ) -> Result<bool, AppError> {
        if !may_know(held, definition.field.id) {
            return Ok(false);
        }
        let projects = &definition.scope.projects;
        if projects.is_empty() {
            return Ok(true);
        }
        if let Some(chain) = chain {
            return Ok(projects.iter().any(|project| chain.contains(project)));
        }
        if held.allows(Permission::ManageFields) {
            return Ok(true);
        }
        for project in projects {
            if self.visible_project(actor, *project, seen).await?.is_some() {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// The names of the own fields the viewer may know at a place (the screens that list fields
    /// of the catalogue next to the own ones show no other name).
    pub(crate) async fn known_field_names(
        &self,
        actor: &Actor,
        held: &Evaluation,
        project: Option<ProjectId>,
    ) -> Result<HashMap<FieldKey, String>, AppError> {
        let chain = match project {
            Some(project) => Some(self.chain_of(project).await?),
            None => None,
        };
        let mut seen = Seen::new();
        let mut names = HashMap::new();
        for definition in self.field_catalogue().await?.iter() {
            if self
                .field_in_view(actor, held, chain.as_deref(), definition, &mut seen)
                .await?
            {
                names.insert(
                    FieldKey::Own(definition.field.id),
                    definition.field.name.clone(),
                );
            }
        }
        Ok(names)
    }

    /// Where a field is used, as the viewer may know it: the projects they see, and the types.
    async fn scope_view(
        &self,
        actor: &Actor,
        scope: &Scope,
        types: &[IssueType],
        seen: &mut Seen,
    ) -> Result<ScopeView, AppError> {
        let mut projects = Vec::new();
        for project in &scope.projects {
            if let Some(visible) = self.visible_project(actor, *project, seen).await? {
                projects.push(visible);
            }
        }
        Ok(ScopeView {
            whole_organization: scope.projects.is_empty(),
            projects,
            issue_types: types
                .iter()
                .filter(|found| scope.issue_types.contains(&found.id))
                .map(IssueTypeRef::from)
                .collect(),
        })
    }

    /// A field that was just made or changed, as the person who did it sees it.
    pub(super) async fn field_shown(
        &self,
        actor: &Actor,
        held: &Evaluation,
        definition: &Definition,
    ) -> Result<FieldView, AppError> {
        let types = self
            .store()
            .issue_types()
            .await
            .map_err(|error| internal::<S>(&error))?;
        self.field_view(actor, held, definition, &types, &mut Seen::default())
            .await
    }

    /// The field as the viewer may see it.
    pub(super) async fn field_view(
        &self,
        actor: &Actor,
        held: &Evaluation,
        definition: &Definition,
        types: &[IssueType],
        seen: &mut Seen,
    ) -> Result<FieldView, AppError> {
        let field = &definition.field;
        Ok(FieldView {
            id: field.id,
            name: field.name.clone(),
            help: field.help.clone(),
            kind: field.kind,
            applies_to: field.applies_to,
            rules: field.rules.clone(),
            default_value: current_default(definition).and_then(|value| value.to_json(field.kind)),
            look: field.look,
            never_to_assistant: field.never_to_assistant,
            retired: field.retired_at.is_some(),
            scope: self
                .scope_view(actor, &definition.scope, types, seen)
                .await?,
            choices: definition.choices.iter().map(ChoiceView::from).collect(),
            may_manage: held.allows(Permission::ManageFields),
            version: field.version,
        })
    }

    /// The issue types, for the names in a scope.
    async fn types_for_scope(&self) -> Result<Vec<IssueType>, AppError> {
        self.store()
            .issue_types()
            .await
            .map_err(|error| internal::<S>(&error))
    }
}

/// Whether the viewer may know the field exists: they manage fields, or they may see it in some
/// issue type.
///
/// *Manage fields* counts, because a field manager reads every definition on the *Fields* settings
/// page anyway; it gives no right to read a value. The rule is written in ADR-0027, amendment WP-05
/// (*Names of own fields*).
pub(crate) fn may_know(held: &Evaluation, field: FieldId) -> bool {
    held.allows(Permission::ManageFields)
        || held.field_mask().can_see_somewhere(FieldKey::Own(field))
}

/// Lists the own fields the person sees, in the order they are offered in, for the organization or
/// for one project: only the fields that are used there.
///
/// # Errors
/// `resource.not_found` for a project the person may not know about.
pub async fn fields<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: FieldsQuery,
) -> Result<Authorized<FieldPage>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let (place, chain) = match &input.project {
        Some(address) => {
            let project = ctx
                .find_project(address)
                .await?
                .map(|found| found.project)
                .ok_or(AppError::Refused(ErrorCode::NotFound))?;
            let chain = ctx
                .store()
                .project_chain(project.id)
                .await
                .map_err(|error| internal::<S>(&error))?
                .ok_or(AppError::Refused(ErrorCode::NotFound))?;
            (Place::Project(project.id), Some(chain))
        }
        None => (Place::Organization, None),
    };
    let held = ctx.evaluation_of(&actor, place).await?;
    if matches!(place, Place::Project(_)) && !held.sees_project() {
        return Err(AppError::Refused(ErrorCode::NotFound));
    }
    let types = ctx.types_for_scope().await?;
    let mut seen = Seen::new();
    let mut items = Vec::new();
    for definition in ctx.field_catalogue().await?.iter() {
        let field = &definition.field;
        let wanted = (input.retired || field.retired_at.is_none())
            && input.applies_to.is_none_or(|kind| kind == field.applies_to)
            && ctx
                .field_in_view(&actor, &held, chain.as_deref(), definition, &mut seen)
                .await?;
        if wanted {
            items.push(
                ctx.field_view(&actor, &held, definition, &types, &mut seen)
                    .await?,
            );
        }
    }
    Ok(held.authorize(FieldPage { fields: items }))
}

/// The kinds of information a field can hold, for the screen that makes a field.
///
/// # Errors
/// `auth.not_signed_in` without a session.
pub async fn field_types<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: FieldTypesQuery,
) -> Result<Authorized<FieldTypePage>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let held = ctx.evaluation_of(&actor, Place::Organization).await?;
    Ok(held.authorize(FieldTypePage {
        kinds: FieldType::ALL
            .iter()
            .copied()
            .map(FieldTypeInfo::of)
            .collect(),
    }))
}

/// One own field.
///
/// # Errors
/// `resource.not_found` for a field that does not exist or that the person may not see.
pub async fn field<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: FieldQuery,
) -> Result<Authorized<FieldView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let held = ctx.evaluation_of(&actor, Place::Organization).await?;
    let definition = ctx
        .field_catalogue()
        .await?
        .iter()
        .find(|definition| definition.field.id == input.field)
        .cloned()
        .filter(|definition| may_know(&held, definition.field.id))
        .ok_or(AppError::Refused(ErrorCode::NotFound))?;
    let types = ctx.types_for_scope().await?;
    let view = ctx
        .field_view(&actor, &held, &definition, &types, &mut Seen::new())
        .await?;
    Ok(held.authorize(view))
}

/// Where an own field is used now, so a change can be judged before it is made (*Where it's
/// used*): the projects the person sees, where it is limited to or where a value is set, and the
/// issue types.
///
/// # Errors
/// `access.forbidden` without *Manage fields*, `resource.not_found` for a field that is not
/// there.
pub async fn field_usage<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: FieldQuery,
) -> Result<Authorized<FieldUsage>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let held = ctx
        .require(&actor, Permission::ManageFields, Place::Organization)
        .await?;
    let definition = ctx
        .field_catalogue()
        .await?
        .iter()
        .find(|definition| definition.field.id == input.field)
        .cloned()
        .ok_or(AppError::Refused(ErrorCode::NotFound))?;
    let mut seen = Seen::new();
    let mut ids: Vec<ProjectId> = ctx
        .store()
        .projects_with_values(input.field)
        .await
        .map_err(|error| internal::<S>(&error))?;
    for project in &definition.scope.projects {
        if !ids.contains(project) {
            ids.push(*project);
        }
    }
    ids.sort();
    let mut projects = Vec::new();
    for project in ids {
        if let Some(visible) = ctx.visible_project(&actor, project, &mut seen).await? {
            projects.push(visible);
        }
    }
    let project_count = projects.len();
    projects.truncate(USAGE_PROJECTS_MAX);
    Ok(held.authorize(FieldUsage {
        whole_organization: definition.scope.projects.is_empty(),
        project_count,
        truncated: project_count > projects.len(),
        projects,
    }))
}

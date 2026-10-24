//! Reading layouts: the form one person sees (`form_layout`) and the layout an editor changes
//! (`layout_editor`). Both start from the layout that counts at the place (ADR-0027, decision 2).

use std::collections::{BTreeSet, HashMap};

use bb_core::ErrorCode;
use bb_core::access::FieldLevel;
use bb_core::event::RecordKind;
use bb_core::fields::FieldKey;
use bb_core::id::{IssueTypeId, ProjectId, RoleId};
use bb_core::layout::{Layout, LayoutContent, LayoutKind, LayoutPlace, built_in_fields, find};
use bb_policy::{Authorized, Evaluation, LevelSubject as _, Permission, Place};

use super::effective::{Viewer, effective};
use super::types::{
    EditorField, EditorRole, EditorSection, FormLayout, FormLayoutQuery, InheritedFrom,
    LayoutEditor, LayoutEditorQuery, LayoutSlot,
};
use crate::access::{Actor, ROLES_MAX};
use crate::auth::{AppError, AuthStore, Ctx, if_there, internal};
use crate::editing::EditorPlace;
use crate::fields::{Definition, Item, must_be_asked_for};
use crate::issues::IssueTypeRef;
use crate::projects::ProjectRef;

impl EditorPlace<LayoutKind> {
    /// Where a layout for exactly this place is set.
    pub(super) fn layout_place(&self) -> LayoutPlace {
        LayoutPlace {
            project: self.project.as_ref().map(|project| project.id),
            issue_type: self.issue_type,
            kind: self.kind,
        }
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The place of a request for an editor: the project must exist for the person.
    pub(super) async fn layout_where(
        &self,
        project: Option<&str>,
        issue_type: Option<IssueTypeId>,
        kind: LayoutKind,
    ) -> Result<EditorPlace<LayoutKind>, AppError> {
        self.editor_place(project, issue_type, kind).await
    }

    /// The own fields of issues that reach the place and the issue type, retired ones left out, in
    /// the order they are offered in. The organization has no project, so only the issue type
    /// limits what reaches it.
    pub(super) async fn own_fields_for(
        &self,
        at: &EditorPlace<LayoutKind>,
    ) -> Result<Vec<FieldKey>, AppError> {
        let issues = Item {
            kind: RecordKind::Issue,
            id: None,
            chain: at.chain.clone(),
            issue_type: at.issue_type,
            reported_by: None,
        };
        if at.project.is_some() {
            return self.own_fields_at(&issues, |_| true).await;
        }
        self.own_fields_reaching(&issues, |scope| scope.reaches_type(at.issue_type), |_| true)
            .await
    }

    /// Every role, in the order of roles.
    pub(super) async fn all_roles(&self) -> Result<Vec<EditorRole>, AppError> {
        Ok(self
            .store()
            .roles()
            .await
            .map_err(|error| internal::<S>(&error))?
            .into_iter()
            .map(|record| EditorRole {
                id: record.role.id,
                name: record.role.name,
            })
            .collect())
    }

    /// The layout that counts at the place: the nearest one (ADR-0027), else the starting layout.
    /// Also the header it was found by, when there is one.
    pub(super) async fn layout_in_force(
        &self,
        at: &EditorPlace<LayoutKind>,
        roles: &[RoleId],
    ) -> Result<(Option<Layout>, LayoutContent), AppError> {
        let headers = self
            .store()
            .layout_headers(at.kind)
            .await
            .map_err(|error| internal::<S>(&error))?;
        if let Some(header) = find(&at.chain, at.issue_type, &headers) {
            let stored = self
                .store()
                .stored_layout(header.id)
                .await
                .map_err(|error| internal::<S>(&error))?
                .ok_or(AppError::Refused(ErrorCode::NotFound))?;
            return Ok((Some(header.clone()), stored.content));
        }
        let own = self.own_fields_for(at).await?;
        Ok((None, LayoutContent::starting(at.kind, &own, roles)))
    }
}

/// Which fields a form shows to the person who asks, in which sections: what the issue page and
/// the new-issue form read through the one hook of the browser.
///
/// # Errors
/// `resource.not-found` for a project the person may not know about.
pub async fn form_layout<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: FormLayoutQuery,
) -> Result<Authorized<FormLayout>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    ctx.ensure_type_known(&actor, input.issue_type).await?;
    let at = ctx
        .layout_where(Some(&input.project), input.issue_type, input.kind)
        .await?;
    let held = ctx.evaluation_of(&actor, at.place()).await?;
    if !held.sees_project() {
        return Err(AppError::Refused(ErrorCode::NotFound));
    }
    let everyone: Vec<RoleId> = ctx.all_roles().await?.iter().map(|role| role.id).collect();
    let (_, content) = ctx.layout_in_force(&at, &everyone).await?;
    let mask = held.field_mask_for(at.issue_type);
    let item = Item {
        kind: RecordKind::Issue,
        id: None,
        chain: at.chain.clone(),
        issue_type: at.issue_type,
        reported_by: None,
    };
    let own: BTreeSet<FieldKey> = match at.kind {
        LayoutKind::IssuePage => ctx.visible_own_fields(&mask, &item).await?,
        LayoutKind::NewIssue => ctx.changeable_own_fields(&mask, &item).await?,
    }
    .into_iter()
    .collect();
    let required = required_among(&ctx.field_catalogue().await?, &own);
    let roles: Vec<RoleId> = held.roles().iter().map(|role| role.id).collect();
    let sections = effective(
        &content,
        at.kind,
        &Viewer {
            roles: &roles,
            mask: &mask,
            own: &own,
            required: &required,
        },
    );
    Ok(held.authorize(FormLayout {
        kind: at.kind,
        sections,
    }))
}

/// The own fields of `among` that a new item must be asked for (`must_be_asked_for`: required and
/// without a default).
fn required_among(definitions: &[Definition], among: &BTreeSet<FieldKey>) -> Vec<FieldKey> {
    definitions
        .iter()
        .filter(|definition| must_be_asked_for(definition))
        .map(|definition| FieldKey::Own(definition.field.id))
        .filter(|field| among.contains(field))
        .collect()
}

impl<S: AuthStore> Ctx<'_, S> {
    /// Whether the roles' level for a field is too low for this kind of form: they may not see it
    /// (on the new-issue form: not change it). Field permissions win over the layout.
    async fn roles_limited(
        &self,
        at: &EditorPlace<LayoutKind>,
        roles: &[EditorRole],
        fields: &[FieldKey],
    ) -> Result<HashMap<FieldKey, Vec<RoleId>>, AppError> {
        let ids: Vec<RoleId> = roles.iter().map(|role| role.id).collect();
        let levels = self
            .store()
            .level_rules(ids.clone(), at.project.as_ref().map(|project| project.id))
            .await
            .map_err(|error| internal::<S>(&error))?
            .ok_or(AppError::Refused(ErrorCode::NotFound))?;
        let needed = match at.kind {
            LayoutKind::IssuePage => FieldLevel::CanSee,
            LayoutKind::NewIssue => FieldLevel::CanChange,
        };
        Ok(fields
            .iter()
            .map(|field| {
                let limited = ids
                    .iter()
                    .copied()
                    .filter(|role| {
                        FieldKey::level_for(
                            &levels.rules,
                            &levels.chain,
                            *role,
                            *field,
                            at.issue_type,
                        )
                        .is_none_or(|(_, _, level)| level < needed)
                    })
                    .collect();
                (*field, limited)
            })
            .collect())
    }

    /// The editor view of a place: the layout in force, with the marks "field permissions win" and
    /// the fields that could be added. Own fields the editor may not know are left out.
    pub(super) async fn editor_view(
        &self,
        actor: &Actor,
        held: &Evaluation,
        at: &EditorPlace<LayoutKind>,
    ) -> Result<LayoutEditor, AppError> {
        let every_role = self.all_roles().await?;
        let ids: Vec<RoleId> = every_role.iter().map(|role| role.id).collect();
        let (header, content) = self.layout_in_force(at, &ids).await?;
        // The roles, what each is shown, and where field permissions win are what *Decide what
        // roles see* shows on the *Field permissions* page: whoever does not hold it gets none of
        // them, and the editor does not offer the roles (ADR-0027).
        let choose_roles = held.allows(Permission::DecideWhatRolesSee);
        let roles: Vec<EditorRole> = if choose_roles {
            offered_roles(&every_role).to_vec()
        } else {
            Vec::new()
        };
        let names = self
            .known_field_names(actor, held, at.project.as_ref().map(|project| project.id))
            .await?;
        let definitions = self.field_catalogue().await?;
        let required: BTreeSet<FieldKey> = required_among(
            &definitions,
            &names.keys().copied().collect::<BTreeSet<FieldKey>>(),
        )
        .into_iter()
        .collect();
        let reaching = self.own_fields_for(at).await?;
        let reachable: BTreeSet<FieldKey> = reaching.iter().copied().collect();
        let known = |field: FieldKey| match field {
            FieldKey::BuiltIn(_) => true,
            FieldKey::Own(_) => names.contains_key(&field),
        };
        let placed: BTreeSet<FieldKey> = content.placed().map(|placed| placed.field).collect();
        let offered: Vec<FieldKey> = built_in_fields(at.kind)
            .iter()
            .copied()
            .map(FieldKey::BuiltIn)
            .chain(
                reaching
                    .into_iter()
                    .filter(|field| names.contains_key(field)),
            )
            .filter(|field| !placed.contains(field))
            .collect();
        let every: Vec<FieldKey> = placed
            .iter()
            .copied()
            .chain(offered.iter().copied())
            .collect();
        let limited = if choose_roles {
            self.roles_limited(at, &roles, &every).await?
        } else {
            HashMap::new()
        };
        let field_of = |field: FieldKey, shown_to: Vec<RoleId>| EditorField {
            field,
            name: names.get(&field).cloned(),
            roles: if choose_roles { shown_to } else { Vec::new() },
            limited_for: limited.get(&field).cloned().unwrap_or_default(),
            required: at.kind == LayoutKind::NewIssue && required.contains(&field),
        };
        let sections = content
            .sections
            .iter()
            .map(|section| EditorSection {
                tab: section.tab.clone(),
                title: section.title.clone(),
                fields: section
                    .fields
                    .iter()
                    .filter(|placed| known(placed.field))
                    // An own field the layout lists but that no longer reaches the place (retired,
                    // or limited elsewhere) is not offered for editing.
                    .filter(|placed| {
                        placed.field.own().is_none() || reachable.contains(&placed.field)
                    })
                    .map(|placed| field_of(placed.field, placed.roles.clone()))
                    .collect(),
            })
            .collect();
        let exact = header
            .as_ref()
            .filter(|found| found.place() == at.layout_place());
        let inherited_from = match (&header, exact) {
            (Some(found), None) => Some(self.inherited_from(actor, found).await?),
            _ => None,
        };
        Ok(LayoutEditor {
            kind: at.kind,
            project: at.project.clone(),
            issue_type: match at.issue_type {
                Some(id) => self.issue_type_ref(actor, id).await?,
                None => None,
            },
            layout: exact.map(|found| LayoutSlot {
                id: found.id,
                version: found.version,
            }),
            inherited_from,
            sections,
            available: offered
                .into_iter()
                .map(|field| field_of(field, Vec::new()))
                .collect(),
            roles,
            choose_roles,
        })
    }

    /// The issue type a screen names, if the viewer may know it (`offers/known.rs`).
    pub(crate) async fn issue_type_ref(
        &self,
        actor: &Actor,
        id: IssueTypeId,
    ) -> Result<Option<IssueTypeRef>, AppError> {
        Ok(if_there(self.known_type(actor, id).await)?
            .as_ref()
            .map(IssueTypeRef::from))
    }

    /// Where an inherited layout is set, as the editor may know it.
    async fn inherited_from(
        &self,
        actor: &Actor,
        header: &Layout,
    ) -> Result<InheritedFrom, AppError> {
        self.inherited_from_place(actor, header.project, header.issue_type)
            .await
    }

    /// Where an inherited layout or workflow is set (the project and the issue type of its row), as
    /// the editor may know it: a project they cannot see is not named.
    pub(crate) async fn inherited_from_place(
        &self,
        actor: &Actor,
        set_for: Option<ProjectId>,
        issue_type: Option<IssueTypeId>,
    ) -> Result<InheritedFrom, AppError> {
        let project = match set_for {
            Some(id) => {
                let held = self.evaluation_of(actor, Place::Project(id)).await?;
                if held.sees_project() {
                    self.store()
                        .project(id)
                        .await
                        .map_err(|error| internal::<S>(&error))?
                        .as_ref()
                        .map(ProjectRef::from)
                } else {
                    None
                }
            }
            None => None,
        };
        Ok(InheritedFrom {
            whole_organization: set_for.is_none(),
            project,
            issue_type: match issue_type {
                Some(id) => self.issue_type_ref(actor, id).await?,
                None => None,
            },
        })
    }
}

/// Shows the layout of a place to somebody who may change it: the layout that counts there, with
/// the roles it is shown to, the marks where field permissions win, and the fields that could be
/// added. Needs *Manage issue types, kinds of relation and layouts* at the place.
///
/// # Errors
/// `resource.not-found` for a project the person may not know about, `access.forbidden` without
/// the permission.
pub async fn layout_editor<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: LayoutEditorQuery,
) -> Result<Authorized<LayoutEditor>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    ctx.ensure_type_known(&actor, input.issue_type).await?;
    let at = ctx
        .layout_where(input.project.as_deref(), input.issue_type, input.kind)
        .await?;
    let held = ctx
        .require(&actor, Permission::ManageIssueTypes, at.place())
        .await?;
    let view = ctx.editor_view(&actor, &held, &at).await?;
    Ok(held.authorize(view))
}

/// The roles the editor offers to tick: at most [`ROLES_MAX`]. The one definition, for what the
/// editor is shown and for what a save may change.
pub(super) fn offered_roles(every_role: &[EditorRole]) -> &[EditorRole] {
    &every_role[..every_role.len().min(ROLES_MAX)]
}

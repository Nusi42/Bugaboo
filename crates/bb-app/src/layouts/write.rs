//! Changing layouts: making the layout of a place (*Customize this form*), saving its sections
//! under the edit lock, and taking it away so the place uses the layout it inherits again.
//!
//! Everything needs *Manage issue types, kinds of relation and layouts* at the place: for a
//! project, there; for the whole organization, at the organization.

use std::collections::{BTreeSet, HashMap};

use bb_core::ErrorCode;
use bb_core::edit::{EditWhat, LAYOUT_PART};
use bb_core::event::{FieldChange, RecordKind};
use bb_core::fields::FieldKey;
use bb_core::id::{IssueTypeId, LayoutId, RoleId};
use bb_core::layout::{Layout, LayoutContent, PlacedField, SectionContent};
use bb_core::technical::TechnicalName;
use bb_core::version::{FIRST_VERSION, counted_up};
use bb_policy::{Authorized, Permission};
use serde_json::{Value, json};

use super::read::Where;
use super::types::{DeleteLayout, LayoutEditor, MakeLayout, SaveLayout, SectionInput};
use crate::auth::{AppError, AuthStore, Ctx, Done, by, entity, internal, set_to};
use crate::editing::{TextSave, Written};
use crate::member::layout as member;
use crate::projects::Why;

/// The sections of a layout as an event and a draft write them: the one text of the content.
fn summary(content: &LayoutContent) -> Value {
    json!(
        content
            .sections
            .iter()
            .map(|section| json!({
                "tab": section.tab,
                "title": section.title,
                "fields": section.fields.iter().map(|placed| json!({
                    "field": placed.field.code(),
                    "roles": placed.roles.iter().map(ToString::to_string).collect::<Vec<_>>(),
                })).collect::<Vec<_>>(),
            }))
            .collect::<Vec<_>>()
    )
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The place a stored layout is set for, with what is needed to judge and to show it.
    async fn layout_at(&self, layout: &Layout) -> Result<Where, AppError> {
        let project = match layout.project {
            Some(id) => Some(
                self.store()
                    .project(id)
                    .await
                    .map_err(|error| internal::<S>(&error))?
                    .ok_or(AppError::Refused(ErrorCode::NotFound))?,
            ),
            None => None,
        };
        self.layout_where(
            project
                .as_ref()
                .map(|found| found.id.to_string())
                .as_deref(),
            layout.issue_type,
            layout.kind,
        )
        .await
    }

    /// The layout the editor sends, as content: the roles exist, an own field is one the editor may
    /// know and that reaches the place. Returns what was sent, and what is to be stored: that
    /// with the fields the editor cannot know put back where they were (the editor has no row for
    /// them, so they cannot have been removed). Only what was sent may be shown back to the
    /// editor, as the draft; the rest is for the write and the event.
    ///
    /// A person who may not *Decide what roles see* cannot tick roles: a field that is on the layout
    /// keeps the roles it has, and a field that is new is shown to every role.
    async fn content_from(
        &self,
        at: &Where,
        sections: &[SectionInput],
        before: &LayoutContent,
        known: &BTreeSet<FieldKey>,
        may_choose_roles: bool,
    ) -> Result<(LayoutContent, LayoutContent), AppError> {
        let roles: Vec<RoleId> = self.all_roles().await?.iter().map(|role| role.id).collect();
        let reach: BTreeSet<FieldKey> = self.own_fields_for(at).await?.into_iter().collect();
        let kept: HashMap<FieldKey, &Vec<RoleId>> = before
            .placed()
            .map(|placed| (placed.field, &placed.roles))
            .collect();
        let mut sent = LayoutContent {
            sections: Vec::with_capacity(sections.len()),
        };
        for section in sections {
            let mut fields = Vec::with_capacity(section.fields.len());
            for input in &section.fields {
                if input.field.own().is_some()
                    && !(known.contains(&input.field) && reach.contains(&input.field))
                {
                    return Err(ErrorCode::FieldNotHere.into());
                }
                let mut roles_of = if may_choose_roles {
                    if input.roles.iter().any(|role| !roles.contains(role)) {
                        return Err(ErrorCode::InvalidRequest.into());
                    }
                    input.roles.clone()
                } else {
                    kept.get(&input.field)
                        .map_or_else(|| roles.clone(), |stored| (*stored).clone())
                };
                roles_of.sort_unstable();
                roles_of.dedup();
                fields.push(PlacedField {
                    field: input.field,
                    roles: roles_of,
                });
            }
            sent.sections.push(SectionContent {
                tab: section.tab.trim().to_owned(),
                title: section.title.trim().to_owned(),
                fields,
            });
        }
        let mut stored = sent.clone();
        keep_unknown_fields(&mut stored, before, known);
        stored
            .check(at.kind)
            .map_err(|_| AppError::from(ErrorCode::InvalidRequest))?;
        Ok((sent, stored))
    }

    /// The own fields the person may know at the place (`may_know`), and so may place.
    async fn known_own_fields(
        &self,
        actor: &crate::access::Actor,
        held: &bb_policy::Evaluation,
        at: &Where,
    ) -> Result<BTreeSet<FieldKey>, AppError> {
        Ok(self
            .known_field_names(actor, held, at.project.as_ref().map(|project| project.id))
            .await?
            .into_keys()
            .collect())
    }
}

/// Puts the own fields of `before` that the editor does not know into `content` again, each in the
/// section it was in (the last section when that one is gone), so a save of somebody who cannot see
/// a field never takes it away.
fn keep_unknown_fields(
    content: &mut LayoutContent,
    before: &LayoutContent,
    known: &BTreeSet<FieldKey>,
) {
    for (at, section) in before.sections.iter().enumerate() {
        for placed in &section.fields {
            if placed.field.own().is_none() || known.contains(&placed.field) {
                continue;
            }
            if content.sections.is_empty() {
                content.sections.push(SectionContent {
                    tab: section.tab.clone(),
                    title: section.title.clone(),
                    fields: Vec::new(),
                });
            }
            let last = content.sections.len() - 1;
            content.sections[at.min(last)].fields.push(placed.clone());
        }
    }
}

/// Makes the layout of a place as a copy of the one that counts there now, so the place can change
/// it (*Customize this form*). A place that has one is shown as it is.
///
/// # Errors
/// `resource.not_found` for a project the person may not know about, `access.forbidden` without
/// *Manage issue types, kinds of relation and layouts* at the place, `edit.version_changed` when
/// somebody made the layout at the same moment.
pub async fn make_layout<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: MakeLayout,
) -> Result<Authorized<LayoutEditor>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let at = ctx
        .layout_where(input.project.as_deref(), input.issue_type, input.kind)
        .await?;
    let held = ctx
        .require(&actor, Permission::ManageIssueTypes, at.place())
        .await?;
    let ids: Vec<RoleId> = ctx.all_roles().await?.iter().map(|role| role.id).collect();
    let (header, content) = ctx.layout_in_force(&at, &ids).await?;
    if header.is_some_and(|found| found.place() == at.layout_place()) {
        return Ok(held.authorize(ctx.editor_view(&actor, &held, &at).await?));
    }
    // The copy is stored as it is, so it must be one a save would accept too.
    content
        .check(at.kind)
        .map_err(|_| AppError::from(ErrorCode::InvalidRequest))?;
    let layout = Layout {
        id: LayoutId::generate(&mut ctx.ids())
            .map_err(|error| AppError::Internal(error.to_string()))?,
        project: at.project.as_ref().map(|project| project.id),
        issue_type: at.issue_type,
        kind: at.kind,
        version: FIRST_VERSION,
    };
    let event = ctx.event(
        by(actor.person.id),
        entity(RecordKind::Layout, layout.id),
        vec![
            set_to(member::KIND, json!(layout.kind.code())),
            set_to(
                member::PROJECT,
                json!(layout.project.map(|project| project.to_string())),
            ),
            set_to(
                member::ISSUE_TYPE,
                json!(
                    layout
                        .issue_type
                        .map(|issue_type: IssueTypeId| issue_type.to_string())
                ),
            ),
            set_to(member::SECTIONS, summary(&content)),
        ],
    )?;
    let why = Why::default();
    let (stopped, made, copied, place) = (why.clone(), layout.clone(), content, layout.place());
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::lock_policy(transaction).await?;
                let headers = S::layout_headers_in(transaction, place.kind).await?;
                if headers.iter().any(|found| found.place() == place) {
                    return Err(stopped.stop::<S>(ErrorCode::VersionChanged));
                }
                S::insert_layout(transaction, &made, &copied).await
            })
        })
        .await
        .map_err(|error| why.answer::<S>(&error, ErrorCode::VersionChanged))?;
    Ok(held.authorize(ctx.editor_view(&actor, &held, &at).await?))
}

/// Saves the sections and fields of a layout, under the edit lock the person holds. A layout never
/// widens permissions: it only says where a field shows and for which roles, so nothing is checked
/// about levels here; they are applied when the form is read.
///
/// # Errors
/// `resource.not_found` for a layout that is not there or a project the person may not know about,
/// `access.forbidden` without the permission, `request.invalid` for a layout that breaks a rule
/// (twice the same field, too many sections, a role that does not exist), `field.not_here` for an
/// own field that does not reach the place or that the person may not know; `edit.lock_lost`,
/// `edit.taken_over` and `edit.version_changed` for a lock that is gone, was taken over, or a layout
/// that changed since (the sections are kept as the draft of the person in all three).
pub async fn save_layout<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: SaveLayout,
) -> Result<Authorized<LayoutEditor>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let stored = ctx
        .store()
        .stored_layout(input.layout)
        .await
        .map_err(|error| internal::<S>(&error))?
        .ok_or(AppError::Refused(ErrorCode::NotFound))?;
    let at = ctx.layout_at(&stored.layout).await?;
    let held = ctx
        .require(&actor, Permission::ManageIssueTypes, at.place())
        .await?;
    let known = ctx.known_own_fields(&actor, &held, &at).await?;
    let (sent, content) = ctx
        .content_from(
            &at,
            &input.sections,
            &stored.content,
            &known,
            held.allows(Permission::DecideWhatRolesSee),
        )
        .await?;
    let id = input.layout;
    let record = entity(RecordKind::Layout, id);
    let event = ctx.event(
        by(actor.person.id),
        record.clone(),
        vec![
            FieldChange {
                field: member::SECTIONS.to_owned(),
                before: Some(summary(&stored.content)),
                after: Some(summary(&content)),
            },
            counted_up(input.version),
        ],
    )?;
    let save = TextSave {
        what: EditWhat {
            record,
            part: TechnicalName::new(LAYOUT_PART)
                .map_err(|error| AppError::Internal(error.to_string()))?,
        },
        token: input.lock_token,
        // The draft is only what the person sent: the fields they may not know stay out of it.
        text: summary(&sent).to_string(),
        version: input.version,
    };
    let (version, written) = (input.version, content);
    ctx.save_text(
        actor.person.id,
        at.place(),
        &save,
        &event,
        move |transaction| {
            Box::pin(async move {
                if S::replace_layout(transaction, id, &written, version).await? {
                    Ok(Written::Saved(()))
                } else {
                    Ok(Written::Stale)
                }
            })
        },
    )
    .await?;
    Ok(held.authorize(ctx.editor_view(&actor, &held, &at).await?))
}

/// Takes the layout of a place away: the place uses the layout it inherits again, or the starting
/// layout.
///
/// # Errors
/// `resource.not_found` for a layout that is not there or a project the person may not know about,
/// `access.forbidden` without the permission, `edit.version_changed` when the layout changed since
/// the person looked at it, `edit.in_progress` while somebody else holds its edit lock. A lock of
/// the person's own is let go in the same change.
pub async fn delete_layout<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: DeleteLayout,
) -> Result<Done, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let stored = ctx
        .store()
        .stored_layout(input.layout)
        .await
        .map_err(|error| internal::<S>(&error))?
        .ok_or(AppError::Refused(ErrorCode::NotFound))?;
    let at = ctx.layout_at(&stored.layout).await?;
    ctx.require(&actor, Permission::ManageIssueTypes, at.place())
        .await?;
    let (id, version) = (input.layout, input.version);
    let record = entity(RecordKind::Layout, id);
    let event = ctx.event(
        by(actor.person.id),
        record.clone(),
        vec![
            FieldChange {
                field: member::SECTIONS.to_owned(),
                before: Some(summary(&stored.content)),
                after: None,
            },
            counted_up(version),
        ],
    )?;
    let why = Why::default();
    let stopped = why.clone();
    let (what, me, now) = (
        EditWhat {
            record: record.clone(),
            part: TechnicalName::new(LAYOUT_PART)
                .map_err(|error| AppError::Internal(error.to_string()))?,
        },
        actor.person.id,
        ctx.now(),
    );
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                match S::edit_lock_in(transaction, what.clone()).await? {
                    Some(lock) if lock.holder != me && !lock.has_expired(now) => {
                        return Err(stopped.stop::<S>(ErrorCode::EditInProgress));
                    }
                    Some(_) => {
                        S::remove_edit_lock(transaction, &what).await?;
                    }
                    None => {}
                }
                if S::delete_layout(transaction, id, version).await? {
                    Ok(())
                } else {
                    Err(stopped.stop::<S>(ErrorCode::VersionChanged))
                }
            })
        })
        .await
        .map_err(|error| {
            why.answer::<S>(&error, ErrorCode::VersionChanged).about(
                &record,
                at.place(),
                actor.person.id,
            )
        })?;
    Ok(Done)
}

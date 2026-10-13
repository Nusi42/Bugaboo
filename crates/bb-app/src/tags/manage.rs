//! Looking after tags (TAG-01, TAG-03): the list, a tag's color and projects, renaming and
//! combining tags across all issues.
//!
//! Anybody who may change an issue makes a tag by putting it on (`issue.rs`); the people with
//! *Manage labels and tags* for the whole organization look after them here. Renaming and combining
//! change every issue that has the tag, with one event for each issue (`rewrite.rs`), and ask for
//! the version of the tag the person looked at, so a tag that somebody changed meanwhile asks for
//! a new look.

use std::collections::BTreeSet;

use bb_core::ErrorCode;
use bb_core::event::{FieldChange, RecordKind};
use bb_core::id::{EntityId, ProjectId};
use bb_core::paging::window;
use bb_core::tags::{TAG_NAME_MAX, TAG_PROJECTS_MAX, Tag, TagProject, name_key};
use bb_policy::{Authorized, Evaluation, Permission, Place};
use serde_json::json;

use super::rewrite::{Replacement, Rewrite, Sorted};
use super::types::{
    ChangeTag, CombineTags, TagChangePreview, TagChangePreviewQuery, TagPage, TagView, TagsQuery,
};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, by, entity, internal, set_to};
use crate::input::clean_text;
use crate::member::tag as member;
use crate::projects::{ProjectRef, Why};
use std::sync::Arc;

/// The tags with their limits, as the person who looks after them reads them.
struct Catalogue {
    tags: Vec<Tag>,
    limits: Vec<TagProject>,
}

impl Catalogue {
    fn tag(&self, id: bb_core::id::TagId) -> Result<&Tag, ErrorCode> {
        self.tags
            .iter()
            .find(|tag| tag.id == id)
            .ok_or(ErrorCode::NotFound)
    }

    fn projects_of(&self, id: bb_core::id::TagId) -> BTreeSet<ProjectId> {
        self.limits
            .iter()
            .filter(|limit| limit.tag == id)
            .map(|limit| limit.project)
            .collect()
    }

    /// Whether another tag has this name.
    fn name_taken(&self, name: &str, except: bb_core::id::TagId) -> bool {
        let key = name_key(name);
        self.tags
            .iter()
            .any(|tag| tag.id != except && name_key(&tag.name) == key)
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// What the person who looks after tags needs to judge a change: they hold *Manage labels and
    /// tags* for the whole organization.
    async fn tag_manager(&self, actor: &Actor) -> Result<Arc<Evaluation>, AppError> {
        self.require(actor, Permission::ManageLabels, Place::Organization)
            .await
    }

    async fn tag_catalogue(&self) -> Result<Catalogue, AppError> {
        let tags = self
            .store()
            .tags()
            .await
            .map_err(|error| internal::<S>(&error))?;
        let limits = self
            .store()
            .tag_projects()
            .await
            .map_err(|error| internal::<S>(&error))?;
        Ok(Catalogue { tags, limits })
    }

    /// A tag as the person sees it: the projects it is limited to, as far as they see them.
    fn tag_view(catalogue: &Catalogue, shown: &crate::projects::Shown, tag: &Tag) -> TagView {
        let limited = catalogue.projects_of(tag.id);
        TagView {
            id: tag.id,
            name: tag.name.clone(),
            color: tag.color,
            whole_organization: limited.is_empty(),
            projects: limited
                .iter()
                .filter_map(|project| shown.reference(*project))
                .collect::<Vec<ProjectRef>>(),
            version: tag.version,
        }
    }
}

/// Lists the tags of the organization, by name, a page at a time.
///
/// # Errors
/// `access.forbidden` without *Manage labels and tags*, `request.invalid` for a cursor that is not
/// one.
pub async fn tags<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: TagsQuery,
) -> Result<Authorized<TagPage>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let held = ctx.tag_manager(&actor).await?;
    let catalogue = ctx.tag_catalogue().await?;
    let shown = ctx.shown_to(&actor).await?;
    let page = window(catalogue.tags.len(), input.cursor.as_deref(), input.limit)
        .ok_or(AppError::Refused(ErrorCode::InvalidRequest))?;
    Ok(held.authorize(TagPage {
        items: catalogue.tags[page.start..page.end]
            .iter()
            .map(|tag| Ctx::<S>::tag_view(&catalogue, &shown, tag))
            .collect(),
        next_cursor: page.next,
    }))
}

/// What renaming a tag, or combining it with another, would do to the issues the person sees.
///
/// # Errors
/// `access.forbidden` without *Manage labels and tags*, `resource.not_found` for a tag that is not
/// there, `request.invalid` for a name that cannot be one or for nothing asked, `name.taken` for
/// a name another tag has.
pub async fn tag_change_preview<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: TagChangePreviewQuery,
) -> Result<Authorized<TagChangePreview>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let held = ctx.tag_manager(&actor).await?;
    let catalogue = ctx.tag_catalogue().await?;
    let tag = catalogue.tag(input.tag)?;
    let rewrite = match (&input.name, input.into) {
        (Some(name), None) => {
            let name = clean_text(name, TAG_NAME_MAX)?;
            if catalogue.name_taken(&name, tag.id) {
                return Err(ErrorCode::NameTaken.into());
            }
            Rewrite {
                sorted: Sorted::Tag,
                from: tag.id.into(),
                rename: Some(name),
                into: None,
            }
        }
        (None, Some(into)) => {
            let into = catalogue.tag(into)?;
            if into.id == tag.id {
                return Err(ErrorCode::InvalidRequest.into());
            }
            Rewrite {
                sorted: Sorted::Tag,
                from: tag.id.into(),
                rename: None,
                into: Some(Replacement {
                    id: into.id.into(),
                    name: into.name.clone(),
                }),
            }
        }
        _ => return Err(ErrorCode::InvalidRequest.into()),
    };
    let preview = ctx.preview_rewrite(&actor, rewrite).await?;
    Ok(held.authorize(preview))
}

/// What a change of a tag changes, for its event: one member for each thing that differs.
fn differences(
    tag: &Tag,
    name: &str,
    color: bb_core::own_fields::ChoiceColor,
    before: &BTreeSet<ProjectId>,
    after: &BTreeSet<ProjectId>,
) -> Vec<FieldChange> {
    let mut changes = Vec::new();
    if tag.name != name {
        changes.push(FieldChange {
            field: member::NAME.to_owned(),
            before: Some(json!(tag.name)),
            after: Some(json!(name)),
        });
    }
    if tag.color != color {
        changes.push(FieldChange {
            field: member::COLOR.to_owned(),
            before: Some(json!(tag.color.code())),
            after: Some(json!(color.code())),
        });
    }
    if before != after {
        let ids = |set: &BTreeSet<ProjectId>| {
            json!(set.iter().map(ToString::to_string).collect::<Vec<_>>())
        };
        changes.push(FieldChange {
            field: member::PROJECTS.to_owned(),
            before: Some(ids(before)),
            after: Some(ids(after)),
        });
    }
    changes
}

/// Changes a tag: its name (every issue that has it records the new name), its color, and the
/// projects it is limited to. What is left out stays as it is.
///
/// # Errors
/// `access.forbidden` without *Manage labels and tags*, `resource.not_found` for a tag that is not
/// there or a project that is not, `request.invalid` for a name that cannot be one or nothing
/// asked, `name.taken` for a name another tag has, `edit.version_changed` when somebody changed
/// the tag since it was looked at.
pub async fn change_tag<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ChangeTag,
) -> Result<Authorized<TagView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let held = ctx.tag_manager(&actor).await?;
    if input.name.is_none() && input.color.is_none() && input.projects.is_none() {
        return Err(ErrorCode::InvalidRequest.into());
    }
    let catalogue = ctx.tag_catalogue().await?;
    let tag = catalogue.tag(input.tag)?.clone();
    if tag.version != input.version {
        return Err(ErrorCode::VersionChanged.into());
    }
    let name = match &input.name {
        Some(name) => clean_text(name, TAG_NAME_MAX)?,
        None => tag.name.clone(),
    };
    if catalogue.name_taken(&name, tag.id) {
        return Err(ErrorCode::NameTaken.into());
    }
    let color = input.color.unwrap_or(tag.color);
    let before = catalogue.projects_of(tag.id);
    let after: BTreeSet<ProjectId> = match &input.projects {
        Some(projects) => projects.iter().copied().collect(),
        None => before.clone(),
    };
    if after.len() > TAG_PROJECTS_MAX {
        return Err(ErrorCode::InvalidRequest.into());
    }
    let shown = ctx.shown_to(&actor).await?;
    if after.iter().any(|project| shown.get(*project).is_none()) {
        return Err(ErrorCode::NotFound.into());
    }
    let changes = differences(&tag, &name, color, &before, &after);
    let rewrite = (tag.name != name).then(|| Rewrite {
        sorted: Sorted::Tag,
        from: tag.id.into(),
        rename: Some(name.clone()),
        into: None,
    });
    let event = ctx.event(
        by(actor.person.id),
        entity(RecordKind::Tag, tag.id),
        changes,
    )?;
    let events = ctx.event_maker(by(actor.person.id));
    let (id, version, why) = (tag.id, tag.version, Why::default());
    let (stopped, written, projects) = (why.clone(), name.clone(), after.clone());
    let replace_projects = input.projects.is_some();
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::lock_tags_and_labels(transaction).await?;
                // Judged again on what is stored now: another tag may have taken the name, and the
                // issues that have the tag are the ones that have it now.
                let now = S::tags_in(transaction).await?;
                if now
                    .iter()
                    .any(|other| other.id != id && name_key(&other.name) == name_key(&written))
                {
                    return Err(stopped.stop::<S>(ErrorCode::NameTaken));
                }
                if let Some(rewrite) = &rewrite {
                    rewrite.append_events::<S>(transaction, &events).await?;
                }
                if !S::update_tag(transaction, id, &written, color, version).await? {
                    return Err(stopped.stop::<S>(ErrorCode::VersionChanged));
                }
                if replace_projects {
                    let listed: Vec<ProjectId> = projects.iter().copied().collect();
                    S::replace_tag_projects(transaction, id, &listed).await?;
                }
                Ok(())
            })
        })
        .await
        .map_err(|error| why.answer::<S>(&error, ErrorCode::VersionChanged))?;
    let catalogue = ctx.tag_catalogue().await?;
    let stored = catalogue.tag(tag.id)?;
    Ok(held.authorize(Ctx::<S>::tag_view(&catalogue, &shown, stored)))
}

/// Moves every issue from a tag to another one, then takes the first tag away. The tag that stays
/// is limited to the projects of both, or to none when either was for every project.
///
/// # Errors
/// `access.forbidden` without *Manage labels and tags*, `resource.not_found` for a tag that is not
/// there, `request.invalid` for the same tag twice, `edit.version_changed` when somebody changed
/// one of the tags since it was looked at.
pub async fn combine_tags<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: CombineTags,
) -> Result<Authorized<TagView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let held = ctx.tag_manager(&actor).await?;
    let catalogue = ctx.tag_catalogue().await?;
    let (from, into) = (
        catalogue.tag(input.tag)?.clone(),
        catalogue.tag(input.into)?.clone(),
    );
    if from.id == into.id {
        return Err(ErrorCode::InvalidRequest.into());
    }
    if from.version != input.version || into.version != input.into_version {
        return Err(ErrorCode::VersionChanged.into());
    }
    let rewrite = Rewrite {
        sorted: Sorted::Tag,
        from: from.id.into(),
        rename: None,
        into: Some(Replacement {
            id: into.id.into(),
            name: into.name.clone(),
        }),
    };
    let (limited_from, limited_into) = (
        catalogue.projects_of(from.id),
        catalogue.projects_of(into.id),
    );
    let reach: BTreeSet<ProjectId> = if limited_from.is_empty() || limited_into.is_empty() {
        BTreeSet::new()
    } else {
        limited_from.union(&limited_into).copied().collect()
    };
    let event = ctx.event(
        by(actor.person.id),
        entity(RecordKind::Tag, from.id),
        vec![set_to(
            member::COMBINED_INTO,
            json!(EntityId::from(into.id).to_string()),
        )],
    )?;
    let events = ctx.event_maker(by(actor.person.id));
    let (ids, versions, why) = (
        (from.id, into.id),
        (from.version, into.version),
        Why::default(),
    );
    let stopped = why.clone();
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::lock_tags_and_labels(transaction).await?;
                let now = S::tags_in(transaction).await?;
                let current = |id| now.iter().find(|tag| tag.id == id).map(|tag| tag.version);
                if current(ids.0) != Some(versions.0) || current(ids.1) != Some(versions.1) {
                    return Err(stopped.stop::<S>(ErrorCode::VersionChanged));
                }
                rewrite.append_events::<S>(transaction, &events).await?;
                S::retarget_issue_tags(transaction, ids.0, ids.1).await?;
                S::delete_tag(transaction, ids.0).await?;
                let listed: Vec<ProjectId> = reach.iter().copied().collect();
                S::replace_tag_projects(transaction, ids.1, &listed).await?;
                Ok(())
            })
        })
        .await
        .map_err(|error| why.answer::<S>(&error, ErrorCode::VersionChanged))?;
    let catalogue = ctx.tag_catalogue().await?;
    let shown = ctx.shown_to(&actor).await?;
    let kept = catalogue.tag(into.id)?;
    Ok(held.authorize(Ctx::<S>::tag_view(&catalogue, &shown, kept)))
}

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
use bb_core::id::{ProjectId, TagId};
use bb_core::names::name_among;
use bb_core::paging::window;
use bb_core::tags::{TAG_PROJECTS_MAX, Tag, TagProject};
use bb_policy::{Authorized, Evaluation, Permission, Place};
use serde_json::json;

use super::refs::NameRef;
use super::rewrite::{Combining, Replacement, Rewrite, Sorted};
use super::store::{TagLimit, TagUse};
use super::types::{
    ChangeTag, CombineTags, TagChangePreview, TagChangePreviewQuery, TagPage, TagView, TagsQuery,
};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, by, entity, internal};
use crate::input::clean_tag_name;
use crate::member::tag as member;
use crate::projects::{ProjectRef, Why};
use std::sync::Arc;

/// The tags with their limits, as the person who looks after them reads them.
pub(crate) struct Catalogue {
    pub(crate) tags: Vec<Tag>,
    limits: Vec<TagProject>,
}

impl Catalogue {
    fn tag(&self, id: TagId) -> Result<&Tag, ErrorCode> {
        self.tags
            .iter()
            .find(|tag| tag.id == id)
            .ok_or(ErrorCode::NotFound)
    }

    fn projects_of(&self, id: TagId) -> BTreeSet<ProjectId> {
        self.limits
            .iter()
            .filter(|limit| limit.tag == id)
            .map(|limit| limit.project)
            .collect()
    }
}

/// Whether a tag other than `except` has this name, without regard to capitals and to the spaces
/// between the words. The use cases ask it on what they read before the lock, and again inside the
/// change on what is stored now.
fn tag_name_taken(tags: &[Tag], name: &str, except: TagId) -> bool {
    name_among(
        tags.iter()
            .filter(|tag| tag.id != except)
            .map(|tag| tag.name.as_str()),
        name,
    )
}

/// Whether a person may know a tag (SEC-06): it is limited to a project they see, or it is used
/// on an issue they read (`seen`), or it has no limit and no issue has it any more, also none in
/// the trash (`anywhere` is false), so it can still be cleaned up. A tag that only issues or
/// projects they do not see know is not there for them. The one definition, for the lists and for
/// the history.
fn tag_is_known(each: &TagUse) -> bool {
    each.limit == TagLimit::Visible || each.seen || (each.limit == TagLimit::None && !each.anywhere)
}

impl<S: AuthStore> Ctx<'_, S> {
    /// What the person who looks after tags needs to judge a change: they hold *Manage labels and
    /// tags* for the whole organization.
    pub(super) async fn tag_manager(&self, actor: &Actor) -> Result<Arc<Evaluation>, AppError> {
        self.require(actor, Permission::ManageLabels, Place::Organization)
            .await
    }

    pub(crate) async fn tag_catalogue(&self) -> Result<Catalogue, AppError> {
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
        let projects = catalogue
            .projects_of(tag.id)
            .iter()
            .filter_map(|project| shown.reference(*project))
            .collect::<Vec<ProjectRef>>();
        TagView {
            id: tag.id,
            name: tag.name.clone(),
            color: tag.color,
            // A limit to projects the person does not see is not shown, so such a tag reads as
            // for every project to them (ADR-0027: names of hidden projects are not leaked).
            whole_organization: projects.is_empty(),
            projects,
            version: tag.version,
        }
    }

    /// The tags the person may know of: one limited to a project they see, one used on an issue they
    /// read, and an unlimited one no issue has any more, also none in the trash (so it can still be cleaned up). A tag that only issues
    /// or projects they do not see know is not there for them (SEC-06).
    pub(crate) async fn known_tags(
        &self,
        shown: &crate::projects::Shown,
    ) -> Result<BTreeSet<TagId>, AppError> {
        Ok(self
            .known_tag_uses(shown)
            .await?
            .into_iter()
            .map(|each| each.tag.id)
            .collect())
    }

    /// The tags the person may know, with what decided it, in one statement.
    async fn known_tag_uses(
        &self,
        shown: &crate::projects::Shown,
    ) -> Result<Vec<TagUse>, AppError> {
        let whole = shown.whole_sight();
        let visible: Vec<ProjectId> = whole
            .all
            .iter()
            .copied()
            .filter(|project| shown.get(*project).is_some())
            .collect();
        let uses = self
            .store()
            .tags_with_use(&shown.issue_sight(), &whole, &visible)
            .await
            .map_err(|error| internal::<S>(&error))?;
        Ok(uses.into_iter().filter(tag_is_known).collect())
    }

    /// The names of the tags the person may know of, in one spelling (lower case): what the history
    /// of an issue may name. A name no known tag has is not told (a tag that is gone, or one that
    /// only projects the person does not see know).
    pub(crate) async fn known_tag_names(
        &self,
        shown: &crate::projects::Shown,
    ) -> Result<BTreeSet<String>, AppError> {
        Ok(self
            .known_tag_uses(shown)
            .await?
            .into_iter()
            .map(|each| each.tag.name.to_lowercase())
            .collect())
    }

    /// A tag the person may know of, or `resource.not-found`.
    async fn known_tag<'a>(
        &self,
        catalogue: &'a Catalogue,
        shown: &crate::projects::Shown,
        id: TagId,
    ) -> Result<&'a Tag, AppError> {
        let known = self.known_tags(shown).await?;
        if !known.contains(&id) {
            return Err(ErrorCode::NotFound.into());
        }
        Ok(catalogue.tag(id)?)
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
    let known = ctx.known_tags(&shown).await?;
    let listed: Vec<&Tag> = catalogue
        .tags
        .iter()
        .filter(|tag| known.contains(&tag.id))
        .collect();
    let page = window(listed.len(), input.cursor.as_deref(), input.limit)
        .ok_or(AppError::Refused(ErrorCode::InvalidRequest))?;
    Ok(held.authorize(TagPage {
        items: listed[page.start..page.end]
            .iter()
            .map(|tag| Ctx::<S>::tag_view(&catalogue, &shown, tag))
            .collect(),
        next_cursor: page.next,
    }))
}

/// What renaming a tag, or combining it with another, would do to the issues the person sees.
///
/// # Errors
/// `access.forbidden` without *Manage labels and tags*, `resource.not-found` for a tag that is not
/// there, `request.invalid` for a name that cannot be one or for nothing asked, `name.taken` for
/// a name another tag has.
pub async fn tag_change_preview<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: TagChangePreviewQuery,
) -> Result<Authorized<TagChangePreview>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let held = ctx.tag_manager(&actor).await?;
    let catalogue = ctx.tag_catalogue().await?;
    let shown = ctx.shown_to(&actor).await?;
    let tag = ctx.known_tag(&catalogue, &shown, input.tag).await?;
    let rewrite = match (&input.name, input.into) {
        (Some(name), None) => {
            let name = clean_tag_name(name)?;
            if tag_name_taken(&catalogue.tags, &name, tag.id) {
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
            let into = ctx.known_tag(&catalogue, &shown, into).await?;
            if into.id == tag.id {
                return Err(ErrorCode::InvalidRequest.into());
            }
            Rewrite {
                sorted: Sorted::Tag,
                from: tag.id.into(),
                rename: None,
                into: Some(Replacement {
                    id: into.id.into(),
                    named: NameRef::plain(into.name.clone()),
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
/// `access.forbidden` without *Manage labels and tags*, `resource.not-found` for a tag that is not
/// there or a project that is not, `request.invalid` for a name that cannot be one or nothing
/// asked, `name.taken` for a name another tag has, `edit.version-changed` when somebody changed
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
    let shown = ctx.shown_to(&actor).await?;
    let tag = ctx.known_tag(&catalogue, &shown, input.tag).await?.clone();
    if tag.version != input.version {
        return Err(AppError::stale(
            entity(RecordKind::Tag, tag.id),
            Place::Organization,
            actor.person.id,
        ));
    }
    let name = match &input.name {
        Some(name) => clean_tag_name(name)?,
        None => tag.name.clone(),
    };
    if tag_name_taken(&catalogue.tags, &name, tag.id) {
        return Err(ErrorCode::NameTaken.into());
    }
    let color = input.color.unwrap_or(tag.color);
    let before = catalogue.projects_of(tag.id);
    let after: BTreeSet<ProjectId> = match &input.projects {
        Some(projects) => {
            let wanted: BTreeSet<ProjectId> = projects.iter().copied().collect();
            if wanted.iter().any(|project| shown.get(*project).is_none()) {
                return Err(ErrorCode::NotFound.into());
            }
            // The limits on projects the person does not see stay, whatever they send.
            wanted
                .into_iter()
                .chain(
                    before
                        .iter()
                        .copied()
                        .filter(|project| shown.get(*project).is_none()),
                )
                .collect()
        }
        None => before.clone(),
    };
    if after.len() > TAG_PROJECTS_MAX {
        return Err(ErrorCode::InvalidRequest.into());
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
                if tag_name_taken(&now, &written, id) {
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
        .map_err(|error| {
            why.answer::<S>(&error, ErrorCode::VersionChanged).about(
                &entity(RecordKind::Tag, id),
                Place::Organization,
                actor.person.id,
            )
        })?;
    let catalogue = ctx.tag_catalogue().await?;
    let stored = catalogue.tag(tag.id)?;
    Ok(held.authorize(Ctx::<S>::tag_view(&catalogue, &shown, stored)))
}

/// Moves every issue from a tag to another one, then takes the first tag away. The tag that stays
/// is limited to the projects of both, or to none when either was for every project.
///
/// # Errors
/// `access.forbidden` without *Manage labels and tags*, `resource.not-found` for a tag that is not
/// there, `request.invalid` for the same tag twice, `edit.version-changed` when somebody changed
/// one of the tags since it was looked at.
pub async fn combine_tags<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: CombineTags,
) -> Result<Authorized<TagView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let held = ctx.tag_manager(&actor).await?;
    let catalogue = ctx.tag_catalogue().await?;
    let shown = ctx.shown_to(&actor).await?;
    let (from, into) = (
        ctx.known_tag(&catalogue, &shown, input.tag).await?.clone(),
        ctx.known_tag(&catalogue, &shown, input.into).await?.clone(),
    );
    if from.id == into.id {
        return Err(ErrorCode::InvalidRequest.into());
    }
    if from.version != input.version || into.version != input.into_version {
        // The record that changed is the one to tell about: *Use mine* sends its version next.
        let changed = if into.version == input.into_version {
            from.id
        } else {
            into.id
        };
        return Err(AppError::stale(
            entity(RecordKind::Tag, changed),
            Place::Organization,
            actor.person.id,
        ));
    }
    let rewrite = Rewrite {
        sorted: Sorted::Tag,
        from: from.id.into(),
        rename: None,
        into: Some(Replacement {
            id: into.id.into(),
            named: NameRef::plain(into.name.clone()),
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
    let Combining { event, events, why } =
        ctx.combine_events(&actor, Sorted::Tag, from.id.into(), into.id.into())?;
    let (ids, versions) = ((from.id, into.id), (from.version, into.version));
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
        .map_err(|error| {
            why.answer::<S>(&error, ErrorCode::VersionChanged).about(
                &entity(RecordKind::Tag, ids.0),
                Place::Organization,
                actor.person.id,
            )
        })?;
    let catalogue = ctx.tag_catalogue().await?;
    let kept = catalogue.tag(into.id)?;
    Ok(held.authorize(Ctx::<S>::tag_view(&catalogue, &shown, kept)))
}

//! Reading what comments hold: every text of a comment, and what the writer may name after `@`
//! (ISS-05, M3-Q3, M3-Q7).

use bb_auth::looks_like_access_key;
use bb_core::ErrorCode;
use bb_core::access::PeopleLevel;
use bb_core::comment::handle_of;
use bb_core::paging::{page_len, window};
use bb_core::people_kinds::PeopleKind;
use bb_policy::{Authorized, PeopleContext, Permission};

use super::resolve::PEOPLE_LOOK_AT;
use super::types::{
    CommentTextView, CommentVersions, CommentVersionsQuery, MentionChoice, MentionChoices,
    MentionChoicesQuery,
};
use crate::auth::{AppError, AuthStore, Ctx, internal};

impl<S: AuthStore> Ctx<'_, S> {
    /// The person whose *Mentions* are asked for. They are the person's own inbox, so only a
    /// session reaches them: an access key is turned away like it is from the rest of the person's
    /// own data (ADR-0025, decision 8).
    pub(crate) async fn inbox_owner(&self, token: &str) -> Result<crate::access::Actor, AppError> {
        if looks_like_access_key(token) {
            return Err(self.key_on_sign_in_data(token).await);
        }
        self.session_actor(token).await
    }
}

/// Every text a comment had, the current one last. The people who may see the comment may see its
/// earlier texts.
///
/// # Errors
/// `resource.not-found` for a comment that does not exist, is in the trash or whose issue the
/// person may not see.
pub async fn comment_versions<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: CommentVersionsQuery,
) -> Result<Authorized<CommentVersions>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let (comment, issue, held) = ctx.visible_comment(&actor, input.comment, false).await?;
    let earlier = ctx
        .store()
        .comment_versions(comment.id)
        .await
        .map_err(|error| internal::<S>(&error))?;
    let mut items = Vec::with_capacity(earlier.len() + 1);
    for version in &earlier {
        let html = ctx.html_of(&actor, &held, &issue, &version.text).await?;
        items.push(CommentTextView::of(version, html));
    }
    let html = ctx.html_of(&actor, &held, &issue, &comment.text).await?;
    items.push(CommentTextView::current(&comment, html));
    let page = window(items.len(), input.cursor.as_deref(), input.limit)
        .ok_or(AppError::Refused(ErrorCode::InvalidRequest))?;
    Ok(held.authorize(CommentVersions {
        comment: comment.id,
        items: items[page.start..page.end].to_vec(),
        next_cursor: page.next,
    }))
}

/// What the writer may name after `@` in a comment under the issue, filtered by the start of a
/// name: people and groups for someone who sees names, groups and roles for someone who sees roles
/// only, nothing for someone who sees no people. A name two things share is not offered, because
/// the text could not tell them apart.
///
/// # Errors
/// `resource.not-found` for an issue that does not exist, is in the trash or that the person may
/// not see.
pub async fn mention_choices<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: MentionChoicesQuery,
) -> Result<Authorized<MentionChoices>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let (issue, held) = ctx.visible_issue(&actor, &input.issue).await?;
    if !held.allows(Permission::Comment) {
        return Err(ErrorCode::Forbidden.into());
    }
    let level = held.people_level(PeopleKind::Mentions, PeopleContext::Ordinary);
    let search = input
        .search
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty());
    // What was typed after the `@` is the start of a handle (`ana.si`), so a name is offered when
    // its handle starts with it.
    let wanted = search.map(|text| text.trim_start_matches('@').to_lowercase());
    let matches = |name: &str| {
        handle_of(name).is_some_and(|handle| {
            wanted
                .as_deref()
                .is_none_or(|start| handle.starts_with(start))
        })
    };
    let failed = |error| internal::<S>(&error);
    // (target, handle, name), in the order they are offered.
    let mut offered: Vec<MentionChoice> = Vec::new();
    if level == PeopleLevel::Names {
        // People are found by their handle (M3-F70): the one they chose, not their name.
        let people = ctx
            .store()
            .handle_choices(wanted.as_deref().unwrap_or_default(), PEOPLE_LOOK_AT)
            .await
            .map_err(failed)?;
        for (person, handle) in people {
            if person.id != actor.person.id && ctx.person_sees_issue(person.id, &issue).await? {
                offered.push(MentionChoice {
                    target: bb_core::comment::MentionTarget::Person,
                    handle,
                    name: person.display_name,
                });
            }
        }
    }
    if level != PeopleLevel::Hidden {
        for group in ctx.store().groups().await.map_err(failed)? {
            if matches(&group.name)
                && let Some(handle) = handle_of(&group.name)
            {
                offered.push(MentionChoice {
                    target: bb_core::comment::MentionTarget::Group,
                    handle,
                    name: group.name,
                });
            }
        }
    }
    if level == PeopleLevel::RoleOnly {
        for record in ctx.store().roles().await.map_err(failed)? {
            if matches(&record.role.name)
                && let Some(handle) = handle_of(&record.role.name)
            {
                offered.push(MentionChoice {
                    target: bb_core::comment::MentionTarget::Role,
                    handle,
                    name: record.role.name,
                });
            }
        }
    }
    // A handle that two things have is offered for neither: it would name nobody.
    let taken: Vec<String> = offered.iter().map(|choice| choice.handle.clone()).collect();
    offered.retain(|choice| {
        taken
            .iter()
            .filter(|handle| **handle == choice.handle)
            .count()
            == 1
    });
    offered.truncate(page_len(Some(input.limit)));
    Ok(held.authorize(MentionChoices {
        items: offered,
        next_cursor: None,
    }))
}

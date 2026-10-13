//! Writing, changing and deleting comments (ISS-05).
//!
//! A comment is Markdown under an issue. Writing one needs the permission *Comment* and the right
//! to see the issue. Changing the text is the save of a long text: it is edited under a lock, saved
//! with the lock token and the version the editor started from (`Ctx::save_text`), and every
//! earlier text is kept. Only the author changes a text. Deleting puts the comment into the trash
//! (*Undo* takes it out); the author, and whoever may change issues, may.

use bb_core::ErrorCode;
use bb_core::comment::{Comment, CommentVersion, TEXT_PART};
use bb_core::edit::EditWhat;
use bb_core::event::{FieldChange, RecordKind};
use bb_core::id::CommentId;
use bb_core::technical::TechnicalName;
use bb_core::version::{FIRST_VERSION, counted_up, next};
use bb_policy::{Authorized, Permission, Place};
use serde_json::json;

use super::types::{AddComment, CommentView, DeleteComment, RestoreComment, SaveComment};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, Done, by, entity, internal, set_to};
use crate::editing::{TextSave, Written};
use crate::follow::started_following;
use crate::input::clean_long_text;
use crate::member::comment as member;
use crate::projects::Why;

/// The text as it is kept, or `request.invalid`: a long text that is not empty.
fn clean_comment(text: &str) -> Result<String, AppError> {
    let kept = clean_long_text(text)?;
    if kept.trim().is_empty() {
        return Err(ErrorCode::InvalidRequest.into());
    }
    Ok(kept)
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The comment as the actor sees it, whose evaluation at the project of its issue is `held`.
    pub(crate) async fn comment_view(
        &self,
        actor: &Actor,
        held: &bb_policy::Evaluation,
        issue: &bb_core::issue::Issue,
        comment: &Comment,
    ) -> Result<CommentView, AppError> {
        let author = self
            .person_ref(
                held,
                bb_core::people_kinds::PeopleKind::Authors,
                bb_policy::PeopleContext::Ordinary,
                comment.author,
            )
            .await?;
        let mine = comment.author == actor.person.id;
        Ok(CommentView {
            id: comment.id,
            issue: comment.issue,
            author,
            text: comment.text.clone(),
            html: self.html_of(actor, held, issue, &comment.text).await?,
            created_at: comment.created_at,
            edited_at: comment.edited_at,
            version: comment.version,
            may_edit: mine && held.allows(Permission::Comment),
            may_delete: mine && held.allows(Permission::Comment)
                || held.allows(Permission::ChangeIssues),
        })
    }

    /// The comment and its issue for an actor who may see them, with what they may do there. A
    /// comment that does not exist, is in the trash (`trashed` says whether that is what is
    /// wanted) or whose issue the actor may not see answers `resource.not_found`.
    pub(crate) async fn visible_comment(
        &self,
        actor: &Actor,
        id: CommentId,
        trashed: bool,
    ) -> Result<
        (
            Comment,
            bb_core::issue::Issue,
            std::sync::Arc<bb_policy::Evaluation>,
        ),
        AppError,
    > {
        let not_found = || AppError::Refused(ErrorCode::NotFound);
        let comment = self
            .store()
            .comment(id)
            .await
            .map_err(|error| internal::<S>(&error))?
            .filter(|comment| comment.deleted_at.is_some() == trashed)
            .ok_or_else(not_found)?;
        let (issue, held) = self
            .visible_issue(actor, &comment.issue.to_string())
            .await?;
        Ok((comment, issue, held))
    }
}

/// Writes a comment under an issue.
///
/// # Errors
/// `resource.not_found` for an issue that does not exist, is in the trash or that the person may
/// not see, `access.forbidden` without the permission *Comment*, `request.invalid` for a text that
/// is empty, too long or holds control characters.
pub async fn add_comment<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: AddComment,
) -> Result<Authorized<CommentView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let (issue, held) = ctx.visible_issue(&actor, &input.issue).await?;
    if !held.allows(Permission::Comment) {
        return Err(ErrorCode::Forbidden.into());
    }
    let text = clean_comment(&input.text)?;
    let tokens = bb_markdown::render(&text, &bb_markdown::Options::default()).tokens;
    let resolution = ctx
        .resolve_tokens(&actor, &held, &issue, &text, &tokens)
        .await?;
    let now = ctx.now();
    let comment = Comment {
        id: CommentId::generate(&mut ctx.ids())
            .map_err(|error| AppError::Internal(error.to_string()))?,
        issue: issue.id,
        author: actor.person.id,
        text,
        created_at: now,
        edited_at: None,
        deleted_at: None,
        version: FIRST_VERSION,
    };
    let mentions = ctx
        .mentions_of_text(comment.id, comment.author, &issue, &resolution.named)
        .await?;
    let followers = started_following(issue.id, None, &[], Some(comment.author), now);
    let event = ctx.event(
        by(actor.person.id),
        entity(RecordKind::Comment, comment.id),
        vec![
            set_to(member::ISSUE, json!(issue.id.to_string())),
            set_to(member::TEXT, json!(comment.text)),
        ],
    )?;
    let stored = comment.clone();
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::insert_comment(transaction, &stored).await?;
                S::replace_mentions(transaction, stored.id, &mentions).await?;
                S::add_followers(transaction, &followers).await
            })
        })
        .await
        .map_err(|error| crate::auth::change_failed::<S>(&error, crate::auth::Stops::NONE))?;
    let view = ctx.comment_view(&actor, &held, &issue, &comment).await?;
    Ok(held.authorize(view))
}

/// Saves the new text of a comment. The editor holds the lock on the text and started from
/// `version`; the earlier text is kept as a version of the comment.
///
/// # Errors
/// `resource.not_found` for a comment that does not exist, is in the trash or whose issue the
/// person may not see, `access.forbidden` for a comment someone else wrote or without the
/// permission *Comment*, `request.invalid` for a text that is empty, too long or holds control
/// characters, `edit.lock_lost`, `edit.taken_over` and `edit.version_changed` as `Ctx::save_text`
/// says; in all three the text stays as the person's draft.
pub async fn save_comment<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: SaveComment,
) -> Result<Authorized<CommentView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let (comment, issue, held) = ctx.visible_comment(&actor, input.comment, false).await?;
    if comment.author != actor.person.id || !held.allows(Permission::Comment) {
        return Err(ErrorCode::Forbidden.into());
    }
    let text = clean_comment(&input.text)?;
    let tokens = bb_markdown::render(&text, &bb_markdown::Options::default()).tokens;
    let resolution = ctx
        .resolve_tokens(&actor, &held, &issue, &text, &tokens)
        .await?;
    let mentions = ctx
        .mentions_of_text(comment.id, comment.author, &issue, &resolution.named)
        .await?;
    let record = entity(RecordKind::Comment, comment.id);
    let what = EditWhat {
        record: record.clone(),
        part: TechnicalName::new(TEXT_PART)
            .map_err(|error| AppError::Internal(error.to_string()))?,
    };
    let event = ctx.event(
        by(actor.person.id),
        record,
        vec![
            set_to(member::ISSUE, json!(issue.id.to_string())),
            FieldChange {
                field: member::TEXT.to_owned(),
                before: Some(json!(comment.text)),
                after: Some(json!(text)),
            },
            counted_up(input.version),
        ],
    )?;
    let save = TextSave {
        what,
        token: input.lock_token,
        text: text.clone(),
        version: input.version,
    };
    let (id, version, now) = (comment.id, input.version, ctx.now());
    ctx.save_text(
        actor.person.id,
        Place::Project(issue.project),
        &save,
        &event,
        move |transaction| {
            Box::pin(async move {
                let Some(stored) = S::comment_in(transaction, id).await? else {
                    return Ok(Written::Stale);
                };
                if stored.version != version || stored.deleted_at.is_some() {
                    return Ok(Written::Stale);
                }
                S::insert_comment_version(
                    transaction,
                    &CommentVersion {
                        comment: id,
                        version: stored.version,
                        text: stored.text.clone(),
                        saved_at: stored.edited_at.unwrap_or(stored.created_at),
                    },
                )
                .await?;
                if !S::update_comment_text(transaction, id, &text, now, version).await? {
                    return Ok(Written::Stale);
                }
                S::replace_mentions(transaction, id, &mentions).await?;
                Ok(Written::Saved(()))
            })
        },
    )
    .await?;
    let saved = Comment {
        text: save.text.clone(),
        edited_at: Some(now),
        version: next(version),
        ..comment
    };
    let fresh = ctx
        .store()
        .comment(saved.id)
        .await
        .map_err(|error| internal::<S>(&error))?
        .unwrap_or(saved);
    let view = ctx.comment_view(&actor, &held, &issue, &fresh).await?;
    Ok(held.authorize(view))
}

/// What putting a comment into the trash or taking it out comes to: who asks, the comment and its
/// issue, what the actor may do there, and the event.
struct Trashing {
    actor: Actor,
    comment: Comment,
    issue: bb_core::issue::Issue,
    held: std::sync::Arc<bb_policy::Evaluation>,
    event: bb_core::event::DomainEvent,
}

impl<S: AuthStore> Ctx<'_, S> {
    /// Checks that the actor may put the comment into the trash (`into_trash`) or take it out, and
    /// makes the event of it.
    async fn plan_trash(
        &self,
        session_token: &str,
        id: CommentId,
        into_trash: bool,
    ) -> Result<Trashing, AppError> {
        let actor = self.actor(session_token).await?;
        let (comment, issue, held) = self.visible_comment(&actor, id, !into_trash).await?;
        let mine = comment.author == actor.person.id && held.allows(Permission::Comment);
        if !mine && !held.allows(Permission::ChangeIssues) {
            return Err(ErrorCode::Forbidden.into());
        }
        let (before, after) = if into_trash {
            (None, Some(json!(self.now().to_string())))
        } else {
            (comment.deleted_at.map(|at| json!(at.to_string())), None)
        };
        let event = self.event(
            by(actor.person.id),
            entity(RecordKind::Comment, comment.id),
            vec![
                set_to(member::ISSUE, json!(issue.id.to_string())),
                FieldChange {
                    field: member::DELETED_AT.to_owned(),
                    before,
                    after,
                },
                counted_up(comment.version),
            ],
        )?;
        Ok(Trashing {
            actor,
            comment,
            issue,
            held,
            event,
        })
    }
}

/// Puts a comment into the trash. The author may, and whoever may change issues.
///
/// # Errors
/// `resource.not_found` for a comment that does not exist, is in the trash already or whose issue
/// the person may not see, `access.forbidden` for a comment of someone else without the right to
/// change issues.
pub async fn delete_comment<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: DeleteComment,
) -> Result<Done, AppError> {
    let plan = ctx
        .plan_trash(&input.session_token, input.comment, true)
        .await?;
    let (id, now, why) = (plan.comment.id, ctx.now(), Why::default());
    let stopped = why.clone();
    ctx.store()
        .record_change(&plan.event, move |transaction| {
            Box::pin(async move {
                if S::set_comment_deleted(transaction, id, Some(now)).await? {
                    Ok(())
                } else {
                    Err(stopped.stop::<S>(ErrorCode::NotFound))
                }
            })
        })
        .await
        .map_err(|error| why.answer::<S>(&error, ErrorCode::NotFound))?;
    Ok(Done)
}

/// Takes a comment out of the trash (*Undo*).
///
/// # Errors
/// `resource.not_found` for a comment that is not in the trash or whose issue the person may not
/// see, `access.forbidden` as for deleting.
pub async fn restore_comment<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: RestoreComment,
) -> Result<Authorized<CommentView>, AppError> {
    let plan = ctx
        .plan_trash(&input.session_token, input.comment, false)
        .await?;
    let (id, why) = (plan.comment.id, Why::default());
    let stopped = why.clone();
    ctx.store()
        .record_change(&plan.event, move |transaction| {
            Box::pin(async move {
                if S::set_comment_deleted(transaction, id, None).await? {
                    Ok(())
                } else {
                    Err(stopped.stop::<S>(ErrorCode::NotFound))
                }
            })
        })
        .await
        .map_err(|error| why.answer::<S>(&error, ErrorCode::NotFound))?;
    let restored = Comment {
        deleted_at: None,
        version: next(plan.comment.version),
        ..plan.comment
    };
    let view = ctx
        .comment_view(&plan.actor, &plan.held, &plan.issue, &restored)
        .await?;
    Ok(plan.held.authorize(view))
}

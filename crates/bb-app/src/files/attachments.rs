//! The attachments of an issue: listing them with their signed links, putting one into the trash
//! and taking it out again (ATT-01, M4-Q3), and what a person may add (ATT-03).
//!
//! A file is seen by whoever sees its issue, and a file on a comment by whoever sees the comment.
//! Whoever added a file may delete it while they may still add one there, and so may whoever may
//! change issues; *Undo* takes it out of the trash again.

use std::sync::Arc;

use bb_core::ErrorCode;
use bb_core::event::{FieldChange, RecordKind};
use bb_core::files::{Attachment, DownloadLink, LinkTarget, PreviewStatus, StoredFile};
use bb_core::id::{AttachmentId, UserId};
use bb_core::issue::Issue;
use bb_core::version::{counted_up, next};
use bb_policy::{Authorized, Evaluation, Permission};
use serde_json::json;

use super::member;
use super::types::{
    AttachmentView, DeleteAttachment, FileRules, FileRulesQuery, IssueFiles, IssueFilesQuery,
    RestoreAttachment,
};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, Done, by, entity, internal, set_to};
use crate::projects::Why;

/// Whether `person` may put the attachment into the trash or take it out again: whoever may change
/// issues, and the person who added it while they may still add it, which is the rule of comments
/// (a file of the issue itself follows the right to change the issue, a file of a comment the right
/// to comment). The one place that says it, for the button and for the change.
pub(super) fn may_remove(
    held: &Evaluation,
    issue: &Issue,
    attachment: &Attachment,
    person: UserId,
) -> bool {
    held.allows(Permission::ChangeIssues)
        || attachment.added_by == person
            && match attachment.comment {
                Some(_) => held.allows(Permission::Comment),
                None => held.may_change_issue(issue.reported_by),
            }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The signed link that opens `target` of `attachment` for `person` for five minutes.
    pub(crate) fn link_for(
        &self,
        attachment: AttachmentId,
        target: LinkTarget,
        person: UserId,
    ) -> Result<String, AppError> {
        DownloadLink::new(attachment, target, person, self.now())
            .and_then(|link| self.services.links.sign(&link.to_bytes()))
            .ok_or_else(|| AppError::Internal("a link could not be signed".to_owned()))
    }

    /// The attachment as the actor sees it, whose evaluation at the project of its issue is `held`.
    pub(crate) async fn attachment_view(
        &self,
        actor: &Actor,
        held: &Evaluation,
        issue: &Issue,
        attachment: &Attachment,
        file: &StoredFile,
    ) -> Result<AttachmentView, AppError> {
        let added_by = self
            .person_ref(
                held,
                bb_core::people_kinds::PeopleKind::Authors,
                bb_policy::PeopleContext::Ordinary,
                attachment.added_by,
            )
            .await?;
        let preview_link = if attachment.preview == PreviewStatus::Ready {
            Some(self.link_for(attachment.id, LinkTarget::Preview, actor.person.id)?)
        } else {
            None
        };
        Ok(AttachmentView {
            id: attachment.id,
            issue: attachment.issue,
            comment: attachment.comment,
            name: attachment.name.clone(),
            media_type: file.media_type.as_str().to_owned(),
            size: u64::try_from(file.size).unwrap_or(0),
            added_by,
            added_at: attachment.added_at,
            version: attachment.version,
            download_link: self.link_for(attachment.id, LinkTarget::File, actor.person.id)?,
            preview: attachment.preview,
            preview_link,
            may_delete: may_remove(held, issue, attachment, actor.person.id),
        })
    }

    /// The attachment and its issue for an actor who may see them. An attachment that does not
    /// exist, is in the trash (`trashed` says whether that is what is wanted) or whose issue or
    /// comment the actor may not see answers `resource.not_found`.
    pub(crate) async fn visible_attachment(
        &self,
        actor: &Actor,
        id: AttachmentId,
        trashed: bool,
    ) -> Result<(Attachment, Issue, Arc<Evaluation>), AppError> {
        let found = self
            .store()
            .attachment(id)
            .await
            .map_err(|error| internal::<S>(&error))?
            .filter(|attachment| attachment.deleted_at.is_some() == trashed);
        let Some(attachment) = found else {
            return Err(self.not_found_like_a_hidden_issue(actor).await);
        };
        let (issue, held) = self
            .visible_issue(actor, &attachment.issue.to_string())
            .await?;
        if let Some(comment) = attachment.comment {
            // A file on a comment that is in the trash is not shown, nor opened.
            self.visible_comment(actor, comment, false).await?;
        }
        Ok((attachment, issue, held))
    }
}

/// The files of an issue and of the comments under it that the person may see, oldest first.
///
/// # Errors
/// `resource.not_found` for an issue that does not exist or that the person may not see.
pub async fn issue_files<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: IssueFilesQuery,
) -> Result<Authorized<IssueFiles>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let (issue, held) = ctx.visible_issue(&actor, &input.issue).await?;
    let attachments = ctx
        .store()
        .attachments_of_issue(issue.id)
        .await
        .map_err(|error| internal::<S>(&error))?;
    let comments: Vec<_> = attachments
        .iter()
        .filter_map(|attachment| attachment.comment)
        .collect();
    let comments = ctx
        .store()
        .comments_by_ids(&comments)
        .await
        .map_err(|error| internal::<S>(&error))?;
    let hashes: Vec<_> = attachments
        .iter()
        .map(|attachment| attachment.file)
        .collect();
    let files = ctx
        .store()
        .stored_files(&hashes)
        .await
        .map_err(|error| internal::<S>(&error))?;
    let mut items = Vec::new();
    for attachment in &attachments {
        let on_a_deleted_comment = attachment.comment.is_some_and(|id| {
            comments
                .iter()
                .find(|comment| comment.id == id)
                .is_none_or(|comment| comment.deleted_at.is_some())
        });
        let Some(file) = files.iter().find(|file| file.hash == attachment.file) else {
            continue;
        };
        if !on_a_deleted_comment {
            items.push(
                ctx.attachment_view(&actor, &held, &issue, attachment, file)
                    .await?,
            );
        }
    }
    Ok(held.authorize(IssueFiles {
        items,
        next_cursor: None,
    }))
}

/// What the person may add to an issue, and the limits of its project.
///
/// # Errors
/// `resource.not_found` for an issue that does not exist or that the person may not see.
pub async fn file_rules<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: FileRulesQuery,
) -> Result<Authorized<FileRules>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let (issue, held) = ctx.visible_issue(&actor, &input.issue).await?;
    let limits = ctx.file_limits(issue.project).await?;
    Ok(held.authorize(FileRules {
        may_add: held.may_change_issue(issue.reported_by),
        limit_bytes: limits.max_bytes,
    }))
}

/// What putting an attachment into the trash or taking it out comes to.
struct Trashing {
    actor: Actor,
    attachment: Attachment,
    issue: Issue,
    held: Arc<Evaluation>,
    event: bb_core::event::DomainEvent,
}

impl<S: AuthStore> Ctx<'_, S> {
    /// Checks that the actor may put the attachment into the trash (`into_trash`) or take it out,
    /// and makes the event of it.
    async fn plan_attachment_trash(
        &self,
        session_token: &str,
        id: AttachmentId,
        into_trash: bool,
    ) -> Result<Trashing, AppError> {
        let actor = self.actor(session_token).await?;
        let (attachment, issue, held) = self.visible_attachment(&actor, id, !into_trash).await?;
        if !may_remove(&held, &issue, &attachment, actor.person.id) {
            // Whoever may not take a file out of the trash is not told that it is in it.
            return Err(if into_trash {
                ErrorCode::Forbidden
            } else {
                ErrorCode::NotFound
            }
            .into());
        }
        let (before, after) = if into_trash {
            (None, Some(json!(self.now().to_string())))
        } else {
            (attachment.deleted_at.map(|at| json!(at.to_string())), None)
        };
        let event = self.event(
            by(actor.person.id),
            entity(RecordKind::Attachment, attachment.id),
            vec![
                set_to(member::ISSUE, json!(issue.id.to_string())),
                FieldChange {
                    field: member::DELETED_AT.to_owned(),
                    before,
                    after,
                },
                counted_up(attachment.version),
            ],
        )?;
        Ok(Trashing {
            actor,
            attachment,
            issue,
            held,
            event,
        })
    }
}

/// Puts an attachment into the trash. The person who added it may while they may still add it, and
/// whoever may change issues.
///
/// # Errors
/// `resource.not_found` for an attachment that does not exist, is in the trash already or whose
/// issue the person may not see, `access.forbidden` for a file of someone else without the right
/// to change issues.
pub async fn delete_attachment<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: DeleteAttachment,
) -> Result<Done, AppError> {
    let plan = ctx
        .plan_attachment_trash(&input.session_token, input.attachment, true)
        .await?;
    let (id, now, why) = (plan.attachment.id, ctx.now(), Why::default());
    let stopped = why.clone();
    ctx.store()
        .record_change(&plan.event, move |transaction| {
            Box::pin(async move {
                if S::set_attachment_deleted(transaction, id, Some(now)).await? {
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

/// Takes an attachment out of the trash (*Undo*).
///
/// # Errors
/// `resource.not_found` for an attachment that is not in the trash or whose issue the person may
/// not see, `access.forbidden` as for deleting.
pub async fn restore_attachment<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: RestoreAttachment,
) -> Result<Authorized<AttachmentView>, AppError> {
    let plan = ctx
        .plan_attachment_trash(&input.session_token, input.attachment, false)
        .await?;
    let (id, why) = (plan.attachment.id, Why::default());
    let stopped = why.clone();
    ctx.store()
        .record_change(&plan.event, move |transaction| {
            Box::pin(async move {
                if S::set_attachment_deleted(transaction, id, None).await? {
                    Ok(())
                } else {
                    Err(stopped.stop::<S>(ErrorCode::NotFound))
                }
            })
        })
        .await
        .map_err(|error| why.answer::<S>(&error, ErrorCode::NotFound))?;
    let restored = Attachment {
        deleted_at: None,
        version: next(plan.attachment.version),
        ..plan.attachment
    };
    let file = ctx
        .store()
        .stored_file(restored.file)
        .await
        .map_err(|error| internal::<S>(&error))?
        .ok_or_else(|| AppError::Internal("an attachment without its file".to_owned()))?;
    let view = ctx
        .attachment_view(&plan.actor, &plan.held, &plan.issue, &restored, &file)
        .await?;
    Ok(plan.held.authorize(view))
}

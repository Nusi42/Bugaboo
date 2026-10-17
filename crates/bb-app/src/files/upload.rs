//! Uploads in parts (ATT-01, ADR-0032 §2): start with the name and the size, send parts with their
//! offset, finish. An interrupted upload goes on from the last part the server has.
//!
//! The size is checked when the upload starts and the real type when the first part comes, so a
//! file that is too big or of a type the organization never accepts is refused before its parts
//! are stored; both are checked once more when the upload is finished, on the whole file.

use std::sync::Arc;

use bb_core::ErrorCode;
use bb_core::event::RecordKind;
use bb_core::files::{
    ATTACHMENTS_PER_ISSUE_MAX, Attachment, OPEN_UPLOADS_MAX, PART_BYTES, PreviewKind, PreviewState,
    StoredFile, Upload, clean_file_name,
};
use bb_core::id::{AttachmentId, CommentId, JobId, ProjectId, UploadId};
use bb_core::version::FIRST_VERSION;
use bb_policy::{Authorized, Evaluation, Permission};
use jiff::Timestamp;
use serde_json::json;

use super::file_store::{Inspected, PartError};
use super::member;
use super::types::{
    AttachmentView, CancelUploadQuery, FinishUpload, SendPartQuery, StartUploadQuery,
    UploadStateQuery, UploadView,
};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, by, entity, internal, set_to};
use crate::jobs::{kind, kind_name};
use crate::projects::Why;

/// The bytes of a part, or a count, as the number the API says.
fn bytes_of(size: i64) -> u64 {
    u64::try_from(size).unwrap_or(0)
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The issue a file is for, when `actor` may add a file there now: whoever may change the
    /// issue adds files to it (a Guest only on the issues they reported), and the person who
    /// wrote a comment adds files to it while they may comment. An issue the actor may not see
    /// does not exist for them.
    pub(crate) async fn upload_target(
        &self,
        actor: &Actor,
        issue: &str,
        comment: Option<CommentId>,
    ) -> Result<(bb_core::issue::Issue, Arc<Evaluation>), AppError> {
        let (issue, held) = self.visible_issue(actor, issue).await?;
        match comment {
            Some(id) => {
                let (comment, _, _) = self.visible_comment(actor, id, false).await?;
                if comment.issue != issue.id {
                    return Err(self.not_found_like_a_hidden_issue(actor).await);
                }
                if comment.author != actor.person.id || !held.allows(Permission::Comment) {
                    return Err(ErrorCode::Forbidden.into());
                }
            }
            None => {
                if !held.may_change_issue(issue.reported_by) {
                    return Err(ErrorCode::Forbidden.into());
                }
            }
        }
        Ok((issue, held))
    }

    /// The upload of the actor. Somebody else's upload does not exist for them.
    async fn own_upload(&self, actor: &Actor, id: UploadId) -> Result<Upload, AppError> {
        let found = self
            .store()
            .upload(id)
            .await
            .map_err(|error| internal::<S>(&error))?
            .filter(|upload| upload.person == actor.person.id);
        match found {
            Some(upload) => Ok(upload),
            None => Err(self.not_found_like_a_hidden_issue(actor).await),
        }
    }

    /// Throws an upload away: its bytes and its row. A failure is logged and not reported again:
    /// the daily clean-up finds what is left.
    pub(crate) async fn drop_upload(&self, id: UploadId) {
        if let Err(error) = self.files().discard_upload(id).await {
            tracing::warn!(cause = %error, "the bytes of an upload could not be removed");
        }
        if let Err(error) = self.store().delete_upload_row(id).await {
            tracing::warn!(cause = %S::describe(&error), "the row of an upload could not be removed");
        }
    }

    async fn upload_view(&self, upload: &Upload) -> Result<UploadView, AppError> {
        let received = self
            .files()
            .received(upload.id)
            .await
            .map_err(|error| AppError::Internal(error.to_string()))?;
        Ok(UploadView {
            id: upload.id,
            size: bytes_of(upload.size),
            received,
            part_bytes: PART_BYTES as u64,
        })
    }
}

/// Starts an upload of a file to an issue, or to a comment under it.
///
/// # Errors
/// `resource.not_found` for an issue or comment that does not exist or that the person may not
/// see, `access.forbidden` without the right to add files there, `request.invalid` for a name
/// that is no file name or a file without bytes, `file.too_big` for a file over the limit of the
/// project, `file.type_blocked` for a type the organization never accepts, `limit.reached` when
/// the person has too many uploads open or the issue holds as many files as it may.
pub async fn start_upload<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: StartUploadQuery,
) -> Result<Authorized<UploadView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let (issue, held) = ctx
        .upload_target(&actor, &input.issue, input.comment)
        .await?;
    let name = clean_file_name(&input.name).ok_or(ErrorCode::InvalidRequest)?;
    let size = i64::try_from(input.size).map_err(|_| ErrorCode::InvalidRequest)?;
    if size == 0 {
        return Err(ErrorCode::InvalidRequest.into());
    }
    let limits = ctx.file_limits(issue.project).await?;
    if input.size > limits.max_bytes {
        return Err(ErrorCode::FileTooBig.into());
    }
    if limits.blocks(&name, None) {
        return Err(ErrorCode::FileTypeBlocked.into());
    }
    let open = ctx
        .store()
        .open_uploads_of(actor.person.id)
        .await
        .map_err(|error| internal::<S>(&error))?;
    let held_files = ctx
        .store()
        .attachments_of_issue(issue.id)
        .await
        .map_err(|error| internal::<S>(&error))?
        .len();
    if open >= OPEN_UPLOADS_MAX as u64 || held_files >= ATTACHMENTS_PER_ISSUE_MAX {
        return Err(ErrorCode::LimitReached.into());
    }
    let now = ctx.now();
    let upload = Upload {
        id: UploadId::generate(&mut ctx.ids())
            .map_err(|error| AppError::Internal(error.to_string()))?,
        issue: issue.id,
        comment: input.comment,
        person: actor.person.id,
        name,
        size,
        created_at: now,
        updated_at: now,
    };
    ctx.store()
        .insert_upload(&upload)
        .await
        .map_err(|error| internal::<S>(&error))?;
    if let Err(error) = ctx.files().begin_upload(upload.id).await {
        ctx.drop_upload(upload.id).await;
        return Err(AppError::Internal(error.to_string()));
    }
    let view = ctx.upload_view(&upload).await?;
    Ok(held.authorize(view))
}

/// How far an upload is, so an interrupted one goes on from there.
///
/// # Errors
/// `resource.not_found` for an upload that is not the person's or is gone.
pub async fn upload_state<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: UploadStateQuery,
) -> Result<Authorized<UploadView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let upload = ctx.own_upload(&actor, input.upload).await?;
    let (_, held) = ctx
        .upload_target(&actor, &upload.issue.to_string(), upload.comment)
        .await?;
    let view = ctx.upload_view(&upload).await?;
    Ok(held.authorize(view))
}

/// Adds the next part of an upload. It must start where the server's copy ends; the first part
/// also shows what the file really is.
///
/// # Errors
/// `resource.not_found` for an upload that is not the person's or is gone, `access.forbidden` when
/// the person may not add files there any more, `request.invalid` for an empty part, a part that is
/// too big or that goes past the size the upload started with, `file.wrong_part` for a part that
/// does not start where the file stands (the answer to `upload_state` says where), and
/// `file.type_blocked` when the content is of a type the organization never accepts: the upload
/// is then gone.
pub async fn send_part<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: SendPartQuery,
) -> Result<Authorized<UploadView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let upload = ctx.own_upload(&actor, input.upload).await?;
    let (issue, held) = ctx
        .upload_target(&actor, &upload.issue.to_string(), upload.comment)
        .await?;
    let length = u64::try_from(input.bytes.len()).unwrap_or(u64::MAX);
    let end = input.offset.checked_add(length);
    if length == 0
        || input.bytes.len() > PART_BYTES
        || end.is_none_or(|end| end > bytes_of(upload.size))
    {
        return Err(ErrorCode::InvalidRequest.into());
    }
    if input.offset == 0 {
        let limits = ctx.file_limits(issue.project).await?;
        let sniffed = ctx.files().sniff(&input.bytes);
        if limits.blocks(&upload.name, Some(&sniffed)) {
            ctx.drop_upload(upload.id).await;
            return Err(ErrorCode::FileTypeBlocked.into());
        }
    }
    match ctx
        .files()
        .write_part(upload.id, input.offset, input.bytes)
        .await
    {
        Ok(_) => {}
        Err(PartError::OutOfOrder { .. }) => return Err(ErrorCode::FileWrongPart.into()),
        Err(PartError::Io(error)) => return Err(AppError::Internal(error.to_string())),
    }
    ctx.store()
        .touch_upload(upload.id, ctx.now())
        .await
        .map_err(|error| internal::<S>(&error))?;
    let view = ctx.upload_view(&upload).await?;
    Ok(held.authorize(view))
}

/// Throws an unfinished upload away.
///
/// # Errors
/// `resource.not_found` for an upload that is not the person's or is gone.
pub async fn cancel_upload<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: CancelUploadQuery,
) -> Result<crate::auth::Done, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let upload = ctx.own_upload(&actor, input.upload).await?;
    ctx.drop_upload(upload.id).await;
    Ok(crate::auth::Done)
}

/// The row of a file that has just been put together: it gets a preview if it is a kind that has
/// one, and has none otherwise.
fn stored_file_of(inspected: Inspected, now: Timestamp) -> StoredFile {
    let preview = if PreviewKind::of_media_type(&inspected.sniffed.media_type).is_some() {
        PreviewState::Pending
    } else {
        PreviewState::Without
    };
    StoredFile {
        hash: inspected.hash,
        size: i64::try_from(inspected.size).unwrap_or(i64::MAX),
        media_type: inspected.sniffed.media_type,
        preview,
        created_at: now,
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// Reads the whole upload and judges it: all bytes must be there, within the limit of the
    /// project, of a type the organization accepts. An upload that fails the limit or the type is
    /// thrown away.
    async fn inspect_to_finish(
        &self,
        upload: &Upload,
        project: ProjectId,
    ) -> Result<Inspected, AppError> {
        let received = self
            .files()
            .received(upload.id)
            .await
            .map_err(|error| AppError::Internal(error.to_string()))?;
        if received != bytes_of(upload.size) {
            return Err(ErrorCode::FileIncomplete.into());
        }
        let limits = self.file_limits(project).await?;
        let inspected = self
            .files()
            .inspect_upload(upload.id)
            .await
            .map_err(|error| AppError::Internal(error.to_string()))?;
        if inspected.size != bytes_of(upload.size) {
            return Err(ErrorCode::FileIncomplete.into());
        }
        if inspected.size > limits.max_bytes {
            self.drop_upload(upload.id).await;
            return Err(ErrorCode::FileTooBig.into());
        }
        if limits.blocks(&upload.name, Some(&inspected.sniffed)) {
            self.drop_upload(upload.id).await;
            return Err(ErrorCode::FileTypeBlocked.into());
        }
        Ok(inspected)
    }
}

/// Finishes an upload: the whole file is read once, its real type is judged, and the file becomes
/// an attachment of the issue or the comment. A file that is stored already is not stored again.
///
/// # Errors
/// As for `send_part`, and `file.incomplete` when not all bytes are there, `file.too_big` and
/// `file.type_blocked` for what the whole file shows (the upload is then gone).
pub async fn finish_upload<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: FinishUpload,
) -> Result<Authorized<AttachmentView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let upload = ctx.own_upload(&actor, input.upload).await?;
    let (issue, held) = ctx
        .upload_target(&actor, &upload.issue.to_string(), upload.comment)
        .await?;
    let inspected = ctx.inspect_to_finish(&upload, issue.project).await?;
    // The bytes go into the store first: an attachment always has its file.
    ctx.files()
        .keep_upload(upload.id, inspected.hash)
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?;
    let now = ctx.now();
    let mut ids = ctx.ids();
    let attachment = Attachment {
        id: AttachmentId::generate(&mut ids)
            .map_err(|error| AppError::Internal(error.to_string()))?,
        issue: issue.id,
        comment: upload.comment,
        file: inspected.hash,
        name: upload.name.clone(),
        added_by: actor.person.id,
        added_at: now,
        deleted_at: None,
        version: FIRST_VERSION,
    };
    let job = JobId::generate(&mut ids).map_err(|error| AppError::Internal(error.to_string()))?;
    let file = stored_file_of(inspected, now);
    let mut changes = vec![
        set_to(member::ISSUE, json!(issue.id.to_string())),
        set_to(member::NAME, json!(attachment.name)),
        set_to(
            member::FILE,
            json!(bb_core::hex::encode(file.hash.as_bytes())),
        ),
        set_to(member::MEDIA_TYPE, json!(file.media_type.as_str())),
    ];
    if let Some(comment) = attachment.comment {
        changes.push(set_to(member::COMMENT, json!(comment.to_string())));
    }
    let event = ctx.event(
        by(actor.person.id),
        entity(RecordKind::Attachment, attachment.id),
        changes,
    )?;
    let (stored_attachment, stored_file, upload_id) = (attachment.clone(), file.clone(), upload.id);
    let (queue, payload) = (
        kind_name(kind::PREVIEW_FILE)?,
        super::jobs::preview_payload(file.hash),
    );
    let why = Why::default();
    let stopped = why.clone();
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                if !S::delete_upload(transaction, upload_id).await? {
                    return Err(stopped.stop::<S>(ErrorCode::NotFound));
                }
                let new = S::insert_stored_file(transaction, &stored_file).await?;
                S::insert_attachment(transaction, &stored_attachment).await?;
                if new && stored_file.preview == PreviewState::Pending {
                    S::enqueue_job(transaction, job, &queue, &payload, now, now).await?;
                }
                Ok(())
            })
        })
        .await
        .map_err(|error| why.answer::<S>(&error, ErrorCode::NotFound))?;
    let stored = ctx
        .store()
        .stored_file(file.hash)
        .await
        .map_err(|error| internal::<S>(&error))?
        .unwrap_or(file);
    let view = ctx
        .attachment_view(&actor, &held, &attachment, &stored)
        .await?;
    Ok(held.authorize(view))
}

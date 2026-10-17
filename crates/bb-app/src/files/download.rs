//! Downloading a file, or looking at its preview, through a signed link (ATT-07, ADR-0032 §3).
//!
//! A link is good for five minutes and for one person, and the use case judges access again every
//! time it is used: the person must still see the issue and the comment, and the attachment must
//! not be in the trash. A link that is passed on, that ran out, or that is kept after access ended
//! opens nothing, and every one of those answers `resource.not_found`, the same as a file that is
//! not there.

use bb_core::ErrorCode;
use bb_core::files::{
    DownloadLink, LinkTarget, PREVIEW_MEDIA_TYPE, PreviewStatus, UNKNOWN_MEDIA_TYPE,
};

use super::types::{DownloadQuery, FileDownload};
use crate::auth::{AppError, AuthStore, Ctx, internal};

impl<S: AuthStore> Ctx<'_, S> {
    /// The link a token holds, if this server signed it. A token that is no link is `None`.
    fn open_link(&self, token: &str) -> Option<DownloadLink> {
        self.services
            .links
            .open(token)
            .and_then(|claim| DownloadLink::from_bytes(&claim))
    }
}

/// Opens a file or its preview for the person who holds the link. A file has no fields to hide, so
/// the answer is the file itself: whether the person may have it was judged here, against the issue
/// and the comment, with the link only naming what is asked for.
///
/// # Errors
/// `resource.not_found` for a link that is not ours, ran out, is for another person, names an
/// attachment that is gone or in the trash, or names an issue or a comment the person may not see
/// any more; a preview that is not there is not found either.
pub async fn download<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: DownloadQuery,
) -> Result<FileDownload, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let link = ctx
        .open_link(&input.link)
        .filter(|link| link.works_at(ctx.now()) && link.person == actor.person.id);
    let Some(link) = link else {
        // The same work as for a link that is good, so the answers cannot be told apart.
        return Err(ctx.not_found_like_a_hidden_issue(&actor).await);
    };
    let (attachment, _, _) = ctx
        .visible_attachment(&actor, link.attachment, false)
        .await?;
    let file = ctx
        .store()
        .stored_file(attachment.file)
        .await
        .map_err(|error| internal::<S>(&error))?
        .ok_or_else(|| AppError::Internal("an attachment without its file".to_owned()))?;
    if link.target == LinkTarget::Preview && attachment.preview != PreviewStatus::Ready {
        return Err(ErrorCode::NotFound.into());
    }
    let Some(body) = ctx
        .files()
        .open(file.hash, link.target)
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?
    else {
        tracing::error!("a stored file is missing from the file store");
        return Err(AppError::Internal("a stored file is missing".to_owned()));
    };
    // A file is sent as plain data whatever it is: a browser that is told "HTML" or "SVG" could be
    // led to run it, and an attachment is saved by its name anyway. Only the preview, a PNG that
    // the server created, is sent as what it is.
    let media_type = match link.target {
        LinkTarget::File => UNKNOWN_MEDIA_TYPE,
        LinkTarget::Preview => PREVIEW_MEDIA_TYPE,
    }
    .to_owned();
    Ok(FileDownload {
        name: attachment.name,
        media_type,
        body,
    })
}

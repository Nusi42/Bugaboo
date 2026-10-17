//! The scenarios of the file use cases (ADR-0007, M1-F18): the preparation, the call, and how many
//! events the call added. `events.rs` names each use case once and runs them.

use bb_core::files::PART_BYTES;
use bb_core::id::{AttachmentId, IssueId};

use super::scenarios::{Added, a_project, measured, text, token};
use super::scenarios_issues::an_issue;
use super::world::World;
use crate::files::{
    DeleteAttachment, FinishUpload, RestoreAttachment, SendPartQuery, StartUploadQuery,
    delete_attachment, finish_upload, restore_attachment, send_part, start_upload,
};

/// The bytes of the file the scenarios upload.
pub(crate) const A_FILE: &[u8] = b"The printer jams on page two.";

/// Starts an upload of `bytes` to the issue and sends all of them as one part; the ID of the upload.
pub(crate) async fn an_upload(
    world: &World,
    issue: IssueId,
    name: &str,
    bytes: &[u8],
) -> Result<bb_core::id::UploadId, String> {
    let started = start_upload(
        &world.ctx(),
        StartUploadQuery {
            session_token: token(world),
            issue: issue.to_string(),
            comment: None,
            name: name.to_owned(),
            size: bytes.len() as u64,
        },
    )
    .await
    .map_err(text)?
    .into_value();
    for (at, part) in bytes.chunks(PART_BYTES).enumerate() {
        send_part(
            &world.ctx(),
            SendPartQuery {
                session_token: token(world),
                upload: started.id,
                offset: (at * PART_BYTES) as u64,
                bytes: part.to_vec(),
            },
        )
        .await
        .map_err(text)?;
    }
    Ok(started.id)
}

/// A file on the issue.
pub(crate) async fn an_attachment(
    world: &World,
    issue: IssueId,
    name: &str,
    bytes: &[u8],
) -> Result<AttachmentId, String> {
    let upload = an_upload(world, issue, name, bytes).await?;
    Ok(finish_upload(
        &world.ctx(),
        FinishUpload {
            session_token: token(world),
            upload,
        },
    )
    .await
    .map_err(text)?
    .into_value()
    .id)
}

pub(crate) async fn finish_upload_scenario(world: &World) -> Added {
    let project = a_project(world, "ACME").await?;
    let issue = an_issue(world, project).await?;
    let upload = an_upload(world, issue, "jam.txt", A_FILE).await?;
    measured(
        &world.store,
        finish_upload(
            &world.ctx(),
            FinishUpload {
                session_token: token(world),
                upload,
            },
        ),
    )
    .await
}

pub(crate) async fn delete_attachment_scenario(world: &World) -> Added {
    let project = a_project(world, "ACME").await?;
    let issue = an_issue(world, project).await?;
    let attachment = an_attachment(world, issue, "jam.txt", A_FILE).await?;
    measured(
        &world.store,
        delete_attachment(
            &world.ctx(),
            DeleteAttachment {
                session_token: token(world),
                attachment,
            },
        ),
    )
    .await
}

pub(crate) async fn restore_attachment_scenario(world: &World) -> Added {
    let project = a_project(world, "ACME").await?;
    let issue = an_issue(world, project).await?;
    let attachment = an_attachment(world, issue, "jam.txt", A_FILE).await?;
    delete_attachment(
        &world.ctx(),
        DeleteAttachment {
            session_token: token(world),
            attachment,
        },
    )
    .await
    .map_err(text)?;
    measured(
        &world.store,
        restore_attachment(
            &world.ctx(),
            RestoreAttachment {
                session_token: token(world),
                attachment,
            },
        ),
    )
    .await
}

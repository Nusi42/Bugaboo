//! The jobs of files (ADR-0032 §1 and §4, ADR-0017): the preview of a new file, and the daily
//! clean-up of what nothing needs any more.

use bb_core::files::{PreviewKind, PreviewState, TRASH_KEPT, UNFINISHED_KEPT};
use bb_core::hex;
use bb_core::security_log::Digest;
use serde_json::{Value, json};

use crate::auth::{AppError, AuthStore, Ctx, internal};

/// The member of the payload of a preview job that names the file.
const FILE_MEMBER: &str = "file";

/// How many rows one round of the clean-up takes.
const CLEAN_UP_BATCH: u64 = 100;

/// How many rounds one clean-up makes at most, so one run is short; what is left waits for the
/// next day.
const CLEAN_UP_ROUNDS_MAX: usize = 20;

/// The payload of the job that creates the preview of the file with this hash.
pub(super) fn preview_payload(hash: Digest) -> Value {
    json!({ FILE_MEMBER: hex::encode(hash.as_bytes()) })
}

/// The hash a preview job names.
fn hash_of(payload: &Value) -> Result<Digest, AppError> {
    payload
        .get(FILE_MEMBER)
        .and_then(Value::as_str)
        .and_then(hex::decode::<{ Digest::LENGTH }>)
        .map(Digest::new)
        .ok_or(AppError::Refused(bb_core::ErrorCode::InvalidRequest))
}

/// What a clean-up removed, for the log.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Cleaned {
    /// Unfinished uploads that were thrown away.
    pub(crate) uploads: usize,
    /// Attachments that were in the trash for good.
    pub(crate) attachments: u64,
    /// Files that nothing pointed to.
    pub(crate) files: usize,
}

impl<S: AuthStore> Ctx<'_, S> {
    /// One step of the job that creates the preview of a stored file in a process of its own.
    /// A file that cannot be previewed ends up without one, and that is no failure of the job.
    /// Doing it twice does the same.
    pub(crate) async fn run_preview_step(&self, payload: &Value) -> Result<(), AppError> {
        let hash = hash_of(payload)?;
        let Some(file) = self
            .store()
            .stored_file(hash)
            .await
            .map_err(|error| internal::<S>(&error))?
        else {
            return Ok(());
        };
        if file.preview != PreviewState::Pending {
            return Ok(());
        }
        let _hold = self.files().hold(hash).await;
        let state = match PreviewKind::of_media_type(&file.media_type) {
            Some(kind) => self
                .files()
                .create_preview(hash, kind)
                .await
                .map_err(|error| AppError::Internal(error.to_string()))?,
            None => PreviewState::Without,
        };
        self.store()
            .set_preview_state(hash, state)
            .await
            .map_err(|error| internal::<S>(&error))?;
        Ok(())
    }

    /// The end of a preview job that failed for good: the file has no preview.
    pub(crate) async fn preview_gave_up(&self, payload: &Value) -> Result<(), AppError> {
        let hash = hash_of(payload)?;
        self.store()
            .set_preview_state(hash, PreviewState::Without)
            .await
            .map_err(|error| internal::<S>(&error))?;
        Ok(())
    }

    /// The daily clean-up: unfinished uploads a day after their last part, attachments that stayed
    /// in the trash for thirty days, and the files that no attachment points to. The row of a file
    /// goes first, in one statement that looks for an attachment again, so a file that somebody
    /// added at the same moment is kept; the bytes go after it.
    pub(crate) async fn clean_up_files(&self) -> Result<Cleaned, AppError> {
        let mut cleaned = Cleaned::default();
        let now = self.now();
        let unfinished_before = now
            .checked_sub(UNFINISHED_KEPT)
            .map_err(|error| AppError::Internal(error.to_string()))?;
        let trashed_before = now
            .checked_sub(TRASH_KEPT)
            .map_err(|error| AppError::Internal(error.to_string()))?;
        for _ in 0..CLEAN_UP_ROUNDS_MAX {
            let stale = self
                .store()
                .uploads_untouched_since(unfinished_before, CLEAN_UP_BATCH)
                .await
                .map_err(|error| internal::<S>(&error))?;
            for upload in &stale {
                self.drop_upload(upload.id).await;
            }
            cleaned.uploads += stale.len();
            if u64::try_from(stale.len()).unwrap_or(u64::MAX) < CLEAN_UP_BATCH {
                break;
            }
        }
        for _ in 0..CLEAN_UP_ROUNDS_MAX {
            let purged = self
                .store()
                .purge_trashed_attachments(trashed_before, CLEAN_UP_BATCH)
                .await
                .map_err(|error| internal::<S>(&error))?;
            cleaned.attachments += purged;
            if purged < CLEAN_UP_BATCH {
                break;
            }
        }
        for _ in 0..CLEAN_UP_ROUNDS_MAX {
            let loose = self
                .store()
                .unreferenced_files(CLEAN_UP_BATCH)
                .await
                .map_err(|error| internal::<S>(&error))?;
            for file in &loose {
                // The row and the bytes go while the file is held, so a file that is being added
                // at this moment keeps its bytes.
                let _hold = self.files().hold(file.hash).await;
                let gone = self
                    .store()
                    .delete_unreferenced_file(file.hash)
                    .await
                    .map_err(|error| internal::<S>(&error))?;
                if gone {
                    cleaned.files += 1;
                    if let Err(error) = self.files().delete(file.hash).await {
                        tracing::warn!(cause = %error, "the bytes of a deleted file could not be removed");
                    }
                }
            }
            if u64::try_from(loose.len()).unwrap_or(u64::MAX) < CLEAN_UP_BATCH {
                break;
            }
        }
        Ok(cleaned)
    }
}

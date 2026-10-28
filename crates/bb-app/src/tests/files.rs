//! The file use cases against the in-memory store (ATT-01 … ATT-04, ATT-07, ADR-0032): a file is
//! stored once, an interrupted upload goes on, the limits and the blocked types refuse before
//! anything is stored, and a link works for five minutes.

use bb_core::ErrorCode;
use bb_core::files::{
    ATTACHMENTS_PER_ISSUE_MAX, Attachment, BIGGEST_FILE_BYTES, MEGABYTE, OPEN_UPLOADS_BYTES_MAX,
    OPEN_UPLOADS_MAX, PreviewStatus, TRASH_KEPT,
};
use bb_core::id::{AttachmentId, IssueId, UploadId};
use bb_core::security_log::Digest;
use bb_core::settings::{FILES_BLOCKED_TYPES, FILES_MAX_MEGABYTES, SettingValue};

use super::scenarios::{a_project, text, token};
use super::scenarios_files::{A_FILE, an_attachment, an_upload};
use super::scenarios_issues::an_issue;
use super::world::{Outcome, World};
use crate::auth::AppError;
use crate::files::{
    AttachmentStore as _, CancelUploadQuery, DeleteAttachment, DownloadQuery, FileDownload,
    FileStore as _, FinishUpload, IssueFiles, IssueFilesQuery, SendPartQuery, StartUploadQuery,
    UploadStateQuery, cancel_upload, delete_attachment, download, finish_upload, issue_files,
    send_part, start_upload, upload_state,
};
use crate::jobs::JobKind;
use crate::settings::{ChangeSetting, SettingsScope, change_setting};

const DAY: i64 = bb_core::time::SECONDS_PER_DAY;

async fn set(world: &World, scope: SettingsScope, key: &str, value: SettingValue) -> Outcome {
    change_setting(
        &world.ctx(),
        ChangeSetting {
            version: None,
            session_token: token(world),
            scope,
            key: key.to_owned(),
            value,
        },
    )
    .await
    .map_err(text)?;
    Ok(())
}

fn refused<T: std::fmt::Debug>(answer: Result<T, AppError>, code: ErrorCode) {
    match answer {
        Err(AppError::Refused(found)) => assert_eq!(found, code),
        other => panic!("not refused with {code}: {other:?}"),
    }
}

async fn remove(world: &World, attachment: AttachmentId) -> Result<crate::auth::Done, AppError> {
    delete_attachment(
        &world.ctx(),
        DeleteAttachment {
            session_token: token(world),
            attachment,
        },
    )
    .await
}

async fn list(world: &World, issue: IssueId) -> Result<IssueFiles, AppError> {
    issue_files(
        &world.ctx(),
        IssueFilesQuery {
            session_token: token(world),
            issue: issue.to_string(),
        },
    )
    .await
    .map(bb_policy::Authorized::into_value)
}

async fn open(world: &World, link: String) -> Result<FileDownload, AppError> {
    download(
        &world.ctx(),
        DownloadQuery {
            session_token: token(world),
            link,
        },
    )
    .await
}

async fn start(world: &World, issue: &str, name: &str, size: u64) -> Result<(), AppError> {
    start_upload(
        &world.ctx(),
        StartUploadQuery {
            session_token: token(world),
            issue: issue.to_owned(),
            comment: None,
            name: name.to_owned(),
            size,
        },
    )
    .await
    .map(|_| ())
}

#[tokio::test]
async fn att_02_the_clean_up_deletes_a_file_when_nothing_points_to_it() -> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "ACME").await?;
    let (first_issue, second_issue) = (
        an_issue(&world, project).await?,
        an_issue(&world, project).await?,
    );
    let first = an_attachment(&world, first_issue, "jam.txt", A_FILE).await?;
    let second = an_attachment(&world, second_issue, "copy of jam.txt", A_FILE).await?;
    assert_eq!(world.files.file_count(), 1, "the same content is one file");
    assert_eq!(world.store.lock().state.stored_files.len(), 1);
    assert_eq!(world.store.lock().state.attachments.len(), 2);
    assert_eq!(
        world.files.upload_count(),
        0,
        "a finished upload is not kept"
    );

    remove(&world, first).await?;
    remove(&world, second).await?;
    // The second went into the trash twenty days after the first (a session does not last that
    // long, so the day is written into the row).
    let later = world.ctx().now() + jiff::SignedDuration::from_hours(20 * 24);
    for attachment in &mut world.store.lock().state.attachments {
        if attachment.id == second {
            attachment.deleted_at = Some(later);
        }
    }
    // The trash keeps them for *Undo*.
    let cleaned = world.ctx().clean_up_files().await?;
    assert_eq!((cleaned.attachments, cleaned.files), (0, 0));

    // After the trash's days the attachment is gone, and the file stays: another one points to it.
    world.clock.advance(TRASH_KEPT.as_secs() + 1);
    let cleaned = world.ctx().clean_up_files().await?;
    assert_eq!((cleaned.attachments, cleaned.files), (1, 0));
    assert_eq!(world.files.file_count(), 1);

    // When the last one is gone, so is the file.
    world.clock.advance(20 * DAY);
    let cleaned = world.ctx().clean_up_files().await?;
    assert_eq!((cleaned.attachments, cleaned.files), (1, 1));
    assert_eq!(world.files.file_count(), 0);
    assert!(world.store.lock().state.stored_files.is_empty());
    Ok(())
}

#[tokio::test]
async fn att_01_an_interrupted_upload_goes_on_from_the_last_part() -> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "ACME").await?;
    let issue = an_issue(&world, project).await?;
    let (head, tail) = A_FILE.split_at(10);
    let started = start_upload(
        &world.ctx(),
        StartUploadQuery {
            session_token: token(&world),
            issue: issue.to_string(),
            comment: None,
            name: "jam.txt".to_owned(),
            size: A_FILE.len() as u64,
        },
    )
    .await
    .map_err(text)?
    .into_value();
    let part = |offset: u64, bytes: &[u8]| SendPartQuery {
        session_token: token(&world),
        upload: started.id,
        offset,
        bytes: bytes.to_vec(),
    };
    assert_eq!(started.offset, 0);
    let after_first = send_part(&world.ctx(), part(0, head)).await?.into_value();
    assert_eq!(after_first.offset, head.len() as u64);

    // The connection was lost: the client asks where the server stands and goes on from there.
    let state = upload_state(
        &world.ctx(),
        UploadStateQuery {
            session_token: token(&world),
            upload: started.id,
        },
    )
    .await?
    .into_value();
    assert_eq!(state.offset, head.len() as u64);
    // The first part again is refused, and the file is not finished yet.
    refused(
        send_part(&world.ctx(), part(0, head)).await,
        ErrorCode::FileWrongPart,
    );
    let finish = || FinishUpload {
        session_token: token(&world),
        upload: started.id,
    };
    refused(
        finish_upload(&world.ctx(), finish()).await,
        ErrorCode::FileIncomplete,
    );
    send_part(&world.ctx(), part(state.offset, tail)).await?;
    let attachment = finish_upload(&world.ctx(), finish()).await?.into_value();
    assert_eq!(attachment.size, A_FILE.len() as u64);
    assert_eq!(attachment.name, "jam.txt");
    Ok(())
}

#[tokio::test]
async fn att_03_a_file_over_the_limit_is_refused_before_it_is_stored() -> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "ACME").await?;
    let issue = an_issue(&world, project).await?.to_string();
    let default = FILES_MAX_MEGABYTES.default_value().map_err(text)?;
    assert_eq!(default, 25);
    refused(
        start(&world, &issue, "big.bin", 26 * MEGABYTE).await,
        ErrorCode::FileTooBig,
    );
    start(&world, &issue, "fits.bin", 25 * MEGABYTE).await?;

    // The organization may allow more, up to 100 MB, and a project may allow less.
    set(
        &world,
        SettingsScope::Organization,
        "files.max-megabytes",
        SettingValue::Integer(100),
    )
    .await?;
    start(&world, &issue, "big.bin", 60 * MEGABYTE).await?;
    set(
        &world,
        SettingsScope::Project(project),
        "files.max-megabytes",
        SettingValue::Integer(2),
    )
    .await?;
    refused(
        start(&world, &issue, "big.bin", 3 * MEGABYTE).await,
        ErrorCode::FileTooBig,
    );
    // A project never gets more than the organization.
    set(
        &world,
        SettingsScope::Organization,
        "files.max-megabytes",
        SettingValue::Integer(1),
    )
    .await?;
    set(
        &world,
        SettingsScope::Project(project),
        "files.max-megabytes",
        SettingValue::Integer(50),
    )
    .await?;
    refused(
        start(&world, &issue, "big.bin", 2 * MEGABYTE).await,
        ErrorCode::FileTooBig,
    );
    // Only the two that fit started an upload; nothing was stored for the refused ones.
    assert_eq!(world.files.upload_count(), 2);
    assert_eq!(world.store.lock().state.uploads.len(), 2);
    Ok(())
}

#[tokio::test]
async fn att_03_a_blocked_type_is_refused() -> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "ACME").await?;
    let issue = an_issue(&world, project).await?;
    set(
        &world,
        SettingsScope::Organization,
        FILES_BLOCKED_TYPES.definition().key(),
        SettingValue::Text("exe, bat".to_owned()),
    )
    .await?;
    // By its name, before anything is sent.
    refused(
        start(&world, &issue.to_string(), "Setup.EXE", 10).await,
        ErrorCode::FileTypeBlocked,
    );
    // By its content, as soon as the first part shows it, whatever the name says.
    let started = start_upload(
        &world.ctx(),
        StartUploadQuery {
            session_token: token(&world),
            issue: issue.to_string(),
            comment: None,
            name: "holiday.png".to_owned(),
            size: 20,
        },
    )
    .await?
    .into_value();
    let refusal = send_part(
        &world.ctx(),
        SendPartQuery {
            session_token: token(&world),
            upload: started.id,
            offset: 0,
            bytes: b"MZ this is a program".to_vec(),
        },
    )
    .await;
    refused(refusal, ErrorCode::FileTypeBlocked);
    assert_eq!(world.files.upload_count(), 0, "the upload is gone");
    assert!(world.store.lock().state.attachments.is_empty());
    Ok(())
}

#[tokio::test]
async fn att_01_an_unfinished_upload_is_deleted_a_day_after_its_last_part() -> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "ACME").await?;
    let issue = an_issue(&world, project).await?;
    an_upload(&world, issue, "jam.txt", A_FILE).await?;
    assert_eq!(world.files.upload_count(), 1);
    world.clock.advance(DAY / 2);
    assert_eq!(world.ctx().clean_up_files().await?.uploads, 0);
    world.clock.advance(DAY / 2 + 1);
    assert_eq!(world.ctx().clean_up_files().await?.uploads, 1);
    assert_eq!(world.files.upload_count(), 0);
    assert!(world.store.lock().state.uploads.is_empty());
    Ok(())
}

#[tokio::test]
async fn att_07_a_link_works_for_five_minutes_and_a_trashed_file_opens_nothing() -> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "ACME").await?;
    let issue = an_issue(&world, project).await?;
    let attachment = an_attachment(&world, issue, "jam.txt", A_FILE).await?;
    let link = list(&world, issue).await?.items[0].download_link.clone();
    let opened = open(&world, link.clone()).await?;
    assert_eq!(opened.name, "jam.txt");
    assert_eq!(opened.body.size, A_FILE.len() as u64);

    // A token that is no link, or a link with a letter changed, opens nothing.
    refused(
        open(&world, "nonsense".to_owned()).await,
        ErrorCode::NotFound,
    );
    let mut changed = link.clone().into_bytes();
    changed[10] = if changed[10] == b'A' { b'B' } else { b'A' };
    refused(
        open(&world, String::from_utf8(changed)?).await,
        ErrorCode::NotFound,
    );

    // Five minutes later the link has run out; a new list gives a new one.
    world.clock.advance(5 * 60 - 1);
    open(&world, link.clone()).await?;
    world.clock.advance(1);
    refused(open(&world, link.clone()).await, ErrorCode::NotFound);
    let fresh = list(&world, issue).await?.items[0].download_link.clone();
    open(&world, fresh.clone()).await?;

    // A file in the trash opens nothing, with a link that was good a moment ago.
    remove(&world, attachment).await?;
    refused(open(&world, fresh).await, ErrorCode::NotFound);
    assert!(list(&world, issue).await?.items.is_empty());
    Ok(())
}

#[tokio::test]
async fn att_03_open_uploads_are_limited_in_number_and_in_bytes() -> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "ACME").await?;
    let issue = an_issue(&world, project).await?.to_string();
    // The number: no more than a person may have open at a time.
    for at in 0..OPEN_UPLOADS_MAX {
        start(&world, &issue, &format!("{at}.txt"), 10).await?;
    }
    refused(
        start(&world, &issue, "one-too-many.txt", 10).await,
        ErrorCode::LimitReached,
    );
    // Ending an upload makes room again.
    let first = world.store.lock().state.uploads[0].id;
    cancel_upload(
        &world.ctx(),
        CancelUploadQuery {
            session_token: token(&world),
            upload: first,
        },
    )
    .await?;
    start(&world, &issue, "again.txt", 10).await?;
    let open: Vec<_> = world.store.lock().state.uploads.drain(..).collect();
    for upload in open {
        world.files.discard_upload(upload.id).await?;
    }

    // The bytes: what the open uploads say they will have is added up, so a few big ones cannot
    // fill the disk before one of them is finished.
    set(
        &world,
        SettingsScope::Organization,
        "files.max-megabytes",
        SettingValue::Integer(100),
    )
    .await?;
    let biggest = BIGGEST_FILE_BYTES;
    let fits = usize::try_from(OPEN_UPLOADS_BYTES_MAX / biggest).unwrap_or(0);
    for at in 0..fits {
        start(&world, &issue, &format!("{at}.bin"), biggest).await?;
    }
    refused(
        start(&world, &issue, "too-much.bin", 1).await,
        ErrorCode::LimitReached,
    );
    Ok(())
}

/// Puts `count` files on the issue as the store has them, `trashed` of them in the trash.
fn fill_with_files(world: &World, issue: IssueId, count: usize, trashed: usize) -> Outcome {
    let ctx = world.ctx();
    let (person, now) = (world.admin.person.id, ctx.now());
    for at in 0..count {
        world.store.lock().state.attachments.push(Attachment {
            id: AttachmentId::generate(&mut ctx.ids())?,
            issue,
            comment: None,
            file: Digest::new([7; Digest::LENGTH]),
            name: format!("{at}.txt"),
            added_by: person,
            added_at: now,
            deleted_at: (at < trashed).then_some(now),
            preview: PreviewStatus::Without,
            version: 1,
        });
    }
    Ok(())
}

#[tokio::test]
async fn att_03_files_in_the_trash_count_toward_the_limit_of_an_issue() -> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "ACME").await?;
    let issue = an_issue(&world, project).await?;
    // One upload is on its way while the issue fills up.
    let waiting = an_upload(&world, issue, "late.txt", A_FILE).await?;
    fill_with_files(&world, issue, ATTACHMENTS_PER_ISSUE_MAX, 50)?;
    // Deleting files does not make room: they come back with *Undo* until the trash is emptied.
    refused(
        start(&world, &issue.to_string(), "more.txt", 10).await,
        ErrorCode::LimitReached,
    );
    // The upload that was started before is judged again when it is finished.
    refused(
        finish_upload(
            &world.ctx(),
            FinishUpload {
                session_token: token(&world),
                upload: waiting,
            },
        )
        .await,
        ErrorCode::LimitReached,
    );
    Ok(())
}

#[tokio::test]
async fn att_02_the_clean_up_does_not_take_the_bytes_of_a_file_that_is_being_added() -> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "ACME").await?;
    let issue = an_issue(&world, project).await?;
    // A file that nothing points to any more, and that the clean-up has not removed yet.
    let first = an_attachment(&world, issue, "jam.txt", A_FILE).await?;
    remove(&world, first).await?;
    // It went into the trash long ago (a session does not last that long, so the day is written
    // into the row), and the trash was emptied.
    let long_ago = world.ctx().now() - TRASH_KEPT - jiff::SignedDuration::from_secs(1);
    for attachment in &mut world.store.lock().state.attachments {
        attachment.deleted_at = Some(long_ago);
    }
    let purged = world
        .store
        .purge_trashed_attachments(world.ctx().now() - TRASH_KEPT, 10)
        .await
        .map_err(text)?;
    assert_eq!(purged, 1);
    assert_eq!(world.store.lock().state.stored_files.len(), 1);
    assert_eq!(world.files.file_count(), 1);

    // The same content is added again. While that is going on, after the bytes are in the store and
    // before the attachment is, the clean-up runs.
    let upload = an_upload(&world, issue, "again.txt", A_FILE).await?;
    let gate = world.files.stop_after_keeping();
    let (for_finishing, for_cleaning) = (world.ctx(), world.ctx());
    let finishing = finish_upload(
        &for_finishing,
        FinishUpload {
            session_token: token(&world),
            upload,
        },
    );
    let cleaning = async {
        gate.entered.notified().await;
        let cleaning = for_cleaning.clean_up_files();
        let letting_go = async {
            for _ in 0..50 {
                tokio::task::yield_now().await;
            }
            gate.release.notify_one();
        };
        tokio::join!(cleaning, letting_go).0
    };
    let (finished, cleaned) = tokio::join!(finishing, cleaning);
    let attachment = finished?.into_value();
    assert_eq!(cleaned?.files, 0, "the file is being added: it is kept");
    assert_eq!(world.files.file_count(), 1, "the bytes are still there");
    let link = list(&world, issue).await?.items[0].download_link.clone();
    assert_eq!(open(&world, link).await?.name, attachment.name);
    Ok(())
}

#[tokio::test]
async fn att_01_a_part_is_refused_when_the_disk_has_no_room_and_the_upload_goes_on_later() -> Outcome
{
    let world = World::new().await?;
    let project = a_project(&world, "ACME").await?;
    let issue = an_issue(&world, project).await?.to_string();
    start(&world, &issue, "jam.txt", A_FILE.len() as u64).await?;
    let upload = world.store.lock().state.uploads[0].id;
    world.files.fill_up();
    let part = |offset: u64| SendPartQuery {
        session_token: token(&world),
        upload,
        offset,
        bytes: A_FILE.to_vec(),
    };
    refused(
        send_part(&world.ctx(), part(0)).await,
        ErrorCode::FileNoRoom,
    );
    // Nothing of the part was stored, and the upload is still there to go on from the start.
    let state = upload_state(
        &world.ctx(),
        UploadStateQuery {
            session_token: token(&world),
            upload,
        },
    )
    .await?
    .into_value();
    assert_eq!(state.offset, 0);
    Ok(())
}

impl World {
    /// Runs the preview jobs that are waiting, as the job runner would, and takes them off the queue.
    async fn run_preview_jobs(&self) -> Outcome {
        let waiting: Vec<_> = {
            let mut shared = self.store.lock();
            let (preview, rest): (Vec<_>, Vec<_>) = shared
                .state
                .jobs
                .drain(..)
                .partition(|job| job.kind.as_str() == JobKind::PreviewFile.code());
            shared.state.jobs = rest;
            preview
        };
        for job in waiting {
            self.ctx().run_preview_step(&job.payload).await?;
        }
        Ok(())
    }
}

/// The first bytes of a PNG, which the in-memory store takes for a picture.
const A_PICTURE: &[u8] = b"\x89PNG\r\n\x1a\nthe rest of a picture";

async fn finish(
    world: &World,
    upload: UploadId,
) -> Result<bb_policy::Authorized<crate::files::AttachmentView>, AppError> {
    finish_upload(
        &world.ctx(),
        FinishUpload {
            session_token: token(world),
            upload,
        },
    )
    .await
}

#[tokio::test]
async fn att_04_finishing_a_file_that_is_stored_already_answers_like_a_new_one() -> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "ACME").await?;
    let issue = an_issue(&world, project).await?;
    // The first of its content: its preview is on its way.
    let first = finish(
        &world,
        an_upload(&world, issue, "first.png", A_PICTURE).await?,
    )
    .await?
    .into_value();
    assert_eq!(first.preview, PreviewStatus::Pending);
    assert!(first.preview_link.is_none());
    // The job makes the preview, and then the attachment says so.
    world.run_preview_jobs().await?;
    let first_listed = list(&world, issue).await?.items;
    assert_eq!(first_listed[0].preview, PreviewStatus::Ready);
    assert!(first_listed[0].preview_link.is_some());

    // The same content again, by somebody who cannot know it is stored: the answer says the same
    // as for the first one did, with no preview and no link taken from the other attachment.
    let second = finish(
        &world,
        an_upload(&world, issue, "second.png", A_PICTURE).await?,
    )
    .await?
    .into_value();
    assert_eq!(second.preview, first.preview);
    assert_eq!(second.preview_link, first.preview_link);
    assert_eq!(
        (second.media_type, second.size),
        (first.media_type, first.size)
    );
    let listed = list(&world, issue).await?.items;
    let own = |name: &str| listed.iter().find(|item| item.name == name).unwrap();
    assert_eq!(own("second.png").preview, PreviewStatus::Pending);
    assert!(own("second.png").preview_link.is_none());
    // Nor can its preview be opened before its own job ran.
    assert_eq!(world.files.previews_made(), 1);

    // Its own job takes the preview that exists, as its own state; nothing is made again.
    world.run_preview_jobs().await?;
    let listed = list(&world, issue).await?.items;
    assert_eq!(
        listed
            .iter()
            .find(|item| item.name == "second.png")
            .unwrap()
            .preview,
        PreviewStatus::Ready
    );
    assert_eq!(world.files.previews_made(), 1, "the preview was reused");
    Ok(())
}

#[tokio::test]
async fn att_03_two_finishes_at_the_last_place_of_an_issue_do_not_both_pass() -> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "ACME").await?;
    let issue = an_issue(&world, project).await?;
    // Two uploads are complete and wait to be finished while the issue has room for one more file.
    let first = an_upload(&world, issue, "first.txt", A_FILE).await?;
    let second = an_upload(&world, issue, "second.txt", b"Another file.").await?;
    fill_with_files(&world, issue, ATTACHMENTS_PER_ISSUE_MAX - 1, 0)?;
    // Both are finished at the same moment: the limit is judged again inside the change, so one
    // of them gets the place and the other is refused.
    let (one, other) = (world.ctx(), world.ctx());
    let (one, other) = tokio::join!(
        finish_upload(
            &one,
            FinishUpload {
                session_token: token(&world),
                upload: first,
            },
        ),
        finish_upload(
            &other,
            FinishUpload {
                session_token: token(&world),
                upload: second,
            },
        ),
    );
    let refused = [one, other]
        .into_iter()
        .filter(|answer| matches!(answer, Err(AppError::Refused(ErrorCode::LimitReached))))
        .count();
    assert_eq!(refused, 1);
    let held = world
        .store
        .attachment_count_of_issue(issue)
        .await
        .map_err(text)?;
    assert_eq!(held, ATTACHMENTS_PER_ISSUE_MAX as u64);
    // The one that was refused is ended: its row is gone, so it no longer counts as an open upload,
    // and the bytes of its file are gone too, as nothing points to them.
    assert!(world.store.lock().state.uploads.is_empty());
    assert_eq!(world.files.upload_count(), 0);
    assert_eq!(world.files.file_count(), 1);
    Ok(())
}

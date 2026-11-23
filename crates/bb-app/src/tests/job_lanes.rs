//! The two lanes of the queue (ADR-0017, amendment of 2026-11-22) and the previews that
//! make up the long one (ADR-0032 §4): slow previews keep neither an e-mail nor an upload of
//! another file waiting, and a person who adds files without end gets no previews beyond a limit.

use std::time::Duration;

use bb_core::files::{PREVIEWS_PER_PERSON_MAX, PreviewStatus};
use bb_core::notification::MAIL_BATCH_SECONDS;
use bb_core::settings::{FILES_PDF_PREVIEWS, SettingValue};
use bb_core::technical::TechnicalName;

use super::fake_files::PDF_START;
use super::mail::mail_is_on;
use super::notifications::{Change, Crew, Status};
use super::scenarios::{a_project, text, token};
use super::scenarios_files::{A_PICTURE, an_attachment};
use super::scenarios_issues::an_issue;
use super::world::{Outcome, World};
use crate::auth::AppError;
use crate::files::{AttachmentStore as _, FileStore as _};
use crate::jobs::{JobKind, JobStore as _, LEASE, Lane};
use crate::settings::{ChangeSetting, SettingsScope, change_setting};

/// How long a short job may take while previews stall: far less than one preview may take.
const SHORT_JOB_BUDGET: Duration = Duration::from_secs(5);

/// How many slow previews wait before the e-mail.
const SLOW_PREVIEWS: usize = 6;

/// A picture of its own for each number, so each is a file of its own.
fn picture(number: usize) -> Vec<u8> {
    [A_PICTURE, number.to_string().as_bytes()].concat()
}

/// Runs the jobs of `lane` until none is due, as the runner does.
async fn run_lane_while_due(world: &World, lane: Lane) -> Result<(), AppError> {
    while world.ctx().run_lane(lane).await? > 0 {}
    Ok(())
}

#[tokio::test]
async fn adr_0017_an_e_mail_queued_behind_slow_previews_goes_within_its_budget() -> Outcome {
    let mut crew = Crew::new().await?;
    mail_is_on(&crew).await?;
    for number in 0..SLOW_PREVIEWS {
        an_attachment(
            &crew.world,
            crew.issue,
            &format!("{number}.png"),
            &picture(number),
        )
        .await?;
    }
    let _gate = crew.world.files.stall_previews();
    let bob = crew.bob.session_token.clone();
    crew.change(&bob, Change::Status(Status::Done)).await?;
    crew.world.clock.advance(MAIL_BATCH_SECONDS + 1);
    // The long lane takes the previews, which stall; the short lane sends the e-mail meanwhile.
    let long = run_lane_while_due(&crew.world, Lane::Long);
    tokio::pin!(long);
    let short = tokio::time::timeout(
        SHORT_JOB_BUDGET,
        run_lane_while_due(&crew.world, Lane::Short),
    );
    tokio::select! {
        biased;
        sent = short => sent??,
        _ = &mut long => return Err("the previews did not stall".into()),
    }
    assert_eq!(crew.world.mailer.sent().len(), 1);
    assert_eq!(
        crew.world.files.previews_made(),
        0,
        "the previews still wait"
    );
    Ok(())
}

#[tokio::test]
async fn att_04_a_slow_preview_does_not_hold_the_files_of_others() -> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "ACME").await?;
    let issue = an_issue(&world, project).await?;
    let attachment = an_attachment(&world, issue, "slow.png", A_PICTURE).await?;
    let hash = world
        .store
        .attachment(attachment)
        .await
        .map_err(text)?
        .ok_or("the attachment is there")?
        .file;
    let gate = world.files.stall_previews();
    let long = run_lane_while_due(&world, Lane::Long);
    tokio::pin!(long);
    // While the preview is being made, an upload or the clean-up can hold the file.
    let held = async {
        gate.entered.notified().await;
        tokio::time::timeout(SHORT_JOB_BUDGET, world.files.hold(hash)).await
    };
    tokio::select! {
        biased;
        taken = held => {
            taken.map_err(|_| "the preview held the file while it was made")?;
        }
        _ = &mut long => return Err("the preview did not stall".into()),
    }
    gate.release.notify_one();
    long.await?;
    let attachment = world
        .store
        .attachment(attachment)
        .await
        .map_err(text)?
        .ok_or("the attachment is there")?;
    assert_eq!(attachment.preview, PreviewStatus::Ready);
    Ok(())
}

#[tokio::test]
async fn adr_0017_a_long_job_taken_by_another_worker_while_it_waited_is_left_to_it() -> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "ACME").await?;
    let issue = an_issue(&world, project).await?;
    for number in 0..2 {
        an_attachment(&world, issue, &format!("{number}.png"), &picture(number)).await?;
    }
    let gate = world.files.stall_previews();
    let ctx = world.ctx();
    let long = ctx.run_lane(Lane::Long);
    tokio::pin!(long);
    tokio::select! {
        biased;
        () = gate.entered.notified() => {}
        _ = &mut long => return Err("the preview did not stall".into()),
    }
    // The first preview takes longer than the lease, and another worker takes the jobs this one
    // holds: the second waits behind the first.
    world.clock.advance(LEASE.as_secs() + 1);
    let other = TechnicalName::new("worker.other")?;
    let taken = world
        .store
        .lease_jobs(&other, ctx.now(), LEASE, 5, &Lane::Long.kinds()?)
        .await
        .map_err(text)?;
    assert!(taken.len() >= 2, "the other worker took both previews");
    gate.release.notify_one();
    // A worker that started the second preview would stall at the gate again.
    tokio::time::timeout(SHORT_JOB_BUDGET, long)
        .await
        .map_err(|_| "the worker started a job that another worker holds")??;
    assert_eq!(world.files.previews_made(), 1);
    Ok(())
}

#[tokio::test]
async fn att_04_a_person_who_adds_files_without_end_gets_no_preview_beyond_the_limit() -> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "ACME").await?;
    let issue = an_issue(&world, project).await?;
    let limit = usize::try_from(PREVIEWS_PER_PERSON_MAX)?;
    let mut added = Vec::new();
    for number in 0..=limit {
        added.push(an_attachment(&world, issue, &format!("{number}.png"), &picture(number)).await?);
    }
    let previews_waiting = world
        .store
        .lock()
        .state
        .jobs
        .iter()
        .filter(|job| job.kind.as_str() == JobKind::PreviewFile.code())
        .count();
    assert_eq!(previews_waiting, limit);
    let last = world
        .store
        .attachment(added[limit])
        .await
        .map_err(text)?
        .ok_or("the last attachment is there")?;
    assert_eq!(last.preview, PreviewStatus::Without);
    // The file itself is kept and can be downloaded as any other.
    assert!(
        world
            .files
            .open(last.file, bb_core::files::LinkTarget::File)
            .await?
            .is_some()
    );
    Ok(())
}

#[tokio::test]
async fn att_04_an_organization_may_keep_to_previews_of_pictures() -> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "ACME").await?;
    let issue = an_issue(&world, project).await?;
    let pdf = [PDF_START, b"1.7 a page"].concat();
    let before = an_attachment(&world, issue, "manual.pdf", &pdf).await?;
    run_lane_while_due(&world, Lane::Long).await?;
    let preview_of = async |id| -> Result<PreviewStatus, String> {
        Ok(world
            .store
            .attachment(id)
            .await
            .map_err(text)?
            .ok_or("the attachment is there")?
            .preview)
    };
    assert_eq!(preview_of(before).await?, PreviewStatus::Ready);
    change_setting(
        &world.ctx(),
        ChangeSetting {
            version: None,
            session_token: token(&world),
            scope: SettingsScope::Organization,
            key: FILES_PDF_PREVIEWS.definition().key().to_owned(),
            value: SettingValue::Bool(false),
        },
    )
    .await?;
    let pdf_after =
        an_attachment(&world, issue, "other.pdf", &[pdf.as_slice(), b"2"].concat()).await?;
    let picture = an_attachment(&world, issue, "shot.png", &picture(1)).await?;
    run_lane_while_due(&world, Lane::Long).await?;
    assert_eq!(preview_of(pdf_after).await?, PreviewStatus::Without);
    assert_eq!(preview_of(picture).await?, PreviewStatus::Ready);
    Ok(())
}

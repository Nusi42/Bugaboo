//! The two lanes of the queue (ADR-0017, amendment of 2026-11-22) and the previews that
//! make up the long one (ADR-0032 §4): slow previews keep neither an e-mail nor an upload of
//! another file waiting, and a person who adds files without end gets no previews beyond a limit.

use std::time::Duration;

use bb_core::files::{PREVIEWS_PER_PERSON_MAX, PreviewStatus};
use bb_core::notification::MAIL_BATCH_SECONDS;

use super::mail::mail_is_on;
use super::notifications::{Change, Crew, Status};
use super::scenarios::{a_project, text};
use super::scenarios_files::{A_PICTURE, an_attachment};
use super::scenarios_issues::an_issue;
use super::world::{Outcome, World};
use crate::auth::AppError;
use crate::files::{AttachmentStore as _, FileStore as _};
use crate::jobs::{JobKind, Lane};

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

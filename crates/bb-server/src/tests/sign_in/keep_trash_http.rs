//! The trash after its keep period, on both databases (PRIV-05, ADR-0045, M9-Q1, M3-F25, M4-F131):
//! an issue goes for good with its comments, files, relations and events; an issue with time
//! entries stays in the trash, because time entries are business records that only an administrator
//! deletes.

use axum::http::StatusCode;
use bb_app::auth::RequestInfo;
use bb_app::comments::CommentStore as _;
use bb_app::context::Clock as _;
use bb_app::files::AttachmentStore as _;
use bb_app::history::HistoryStore as _;
use bb_app::issues::IssueStore as _;
use bb_core::event::{EntityKind, RecordKind};
use bb_core::files::TRASH_KEPT;
use bb_core::id::{AttachmentId, CommentId, EntityId, IssueId};
use bb_core::time::SECONDS_PER_DAY;
use bb_testing::{Failure, TestResult, each_database};
use serde_json::json;

use super::files_http::{on_disk, run_jobs, upload};
use super::http::{ADA_EMAIL, Browser, rig_and_app};
use super::issues_site::{ISSUES, Site, id_in, id_of, key_in, ok, version_of};
use super::keep_rules_http::DELETE_PAST;
use super::relations_http::{kinds, put_under};
use super::rig::Rig;
use super::time_http::MY_ENTRIES;

/// The kinds of record whose events name an issue of the trash or a record under it.
const KINDS: [RecordKind; 4] = [
    RecordKind::Issue,
    RecordKind::Comment,
    RecordKind::Attachment,
    RecordKind::Relation,
];

/// A new issue: its readable ID and its internal ID.
async fn new_issue(
    site: &Site<'_>,
    browser: &mut Browser,
    title: &str,
) -> Result<(String, IssueId), Failure> {
    let made = site.create(browser, title).await?;
    let answer = ok(&made)?;
    Ok((key_in(answer)?, id_in(answer)?.parse()?))
}

/// Puts the issue into the trash.
async fn into_the_trash(browser: &mut Browser, issue: &str) -> Result<(), Failure> {
    let gone = browser
        .send("DELETE", &format!("{ISSUES}/{issue}"), None)
        .await?;
    assert_eq!(gone.status, StatusCode::NO_CONTENT, "{}", gone.body);
    Ok(())
}

/// The events of the kinds of [`KINDS`] about one of `records`.
async fn events_about(rig: &Rig, records: &[EntityId]) -> Result<usize, Failure> {
    let mut found = 0;
    for kind in KINDS {
        found += rig
            .database
            .events_of_kind(&EntityKind::from(kind), 10_000)
            .await?
            .iter()
            .filter(|event| records.contains(&event.entity.id))
            .count();
    }
    Ok(found)
}

/// How many rows of history the issue has stored, also those the page leaves out because the other
/// issue of a relation is gone.
async fn history_rows(rig: &Rig, issue: IssueId) -> Result<usize, Failure> {
    // The page reads the events into rows first; the store's rows are counted after that.
    rig.services
        .ctx(RequestInfo::default())
        .catch_up_history()
        .await?;
    Ok(rig
        .storage
        .history_page(issue, None, None, 0, 1000)
        .await?
        .len())
}

/// The daily jobs, as a new day starts them.
async fn a_day_later(rig: &Rig, days: i64) -> Result<(), Failure> {
    rig.clock.advance(days * SECONDS_PER_DAY);
    run_jobs(rig).await
}

#[tokio::test]
async fn priv_05_the_trash_is_deleted_after_its_period_with_its_files_and_events() -> TestResult {
    each_database("priv_05_trash", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (doomed, doomed_id) = new_issue(&site, &mut ada, "Printer jams").await?;
        let (kept, kept_id) = new_issue(&site, &mut ada, "Paper tray").await?;
        let written = ada
            .send(
                "POST",
                &format!("{ISSUES}/{doomed}/comments"),
                Some(json!({"text": "The tray is empty"})),
            )
            .await?;
        let comment: CommentId = id_of(&written)?.parse()?;
        let file = upload(&mut ada, &doomed, "jam.txt", b"the jam").await?;
        let attachment: AttachmentId = id_in(&file)?.parse()?;
        let (blocks, _) = kinds(&mut ada).await?;
        let related = ada
            .send(
                "POST",
                &format!("{ISSUES}/{doomed}/relations"),
                Some(json!({"other": kept, "relation_kind": blocks, "direction": "forward"})),
            )
            .await?;
        ok(&related)?;
        let version = version_of(&mut ada, &kept).await?;
        ok(&put_under(&mut ada, &kept, version, Some(&doomed)).await?)?;
        into_the_trash(&mut ada, &doomed).await?;
        let rows_before = history_rows(&rig, kept_id).await?;
        let records = [doomed_id.into(), comment.into(), attachment.into()];
        assert!(events_about(&rig, &records).await? > 0);

        // A day before the end of its period the issue is still in the trash.
        let period = TRASH_KEPT.as_secs() / SECONDS_PER_DAY;
        a_day_later(&rig, period - 1).await?;
        assert!(rig.storage.issue(doomed_id).await?.is_some());
        // Two days later it is gone for good, with what belonged to it.
        a_day_later(&rig, 2).await?;
        assert!(rig.storage.issue(doomed_id).await?.is_none());
        assert!(rig.storage.comment(comment).await?.is_none());
        assert!(rig.storage.attachment(attachment).await?.is_none());
        assert!(on_disk(&rig, "content").is_empty(), "the file went");
        assert_eq!(events_about(&rig, &records).await?, 0);
        // The other issue stays, without the relation and without its parent.
        let mut ada = Browser::signed_in_as(&app, ADA_EMAIL).await?;
        let other = ada.send("GET", &format!("{ISSUES}/{kept}"), None).await?;
        assert!(ok(&other)?["parent"].is_null(), "{}", other.body);
        let relations = ada
            .send("GET", &format!("{ISSUES}/{kept}/related-issues"), None)
            .await?;
        assert!(
            !relations.body.to_string().contains(&doomed),
            "{}",
            relations.body
        );
        // The history of the other issue keeps its rows, also those of the relation that went.
        assert_eq!(history_rows(&rig, kept_id).await?, rows_before);
        // Safe to run again.
        assert_eq!(
            rig.storage
                .purge_trashed_issues(rig.clock.now(), 100)
                .await?,
            0
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn priv_05_an_issue_in_the_trash_stays_while_its_time_entries_are_kept() -> TestResult {
    each_database("priv_05_trash_time", async |db| {
        let (rig, app) = rig_and_app(db).await?;
        let (site, mut ada) = Site::start(&rig, &app).await?;
        let (booked, booked_id) = new_issue(&site, &mut ada, "Customer call").await?;
        let entry = ada
            .send(
                "POST",
                MY_ENTRIES,
                Some(json!({"issue": booked, "date": "2027-01-14", "minutes": 30})),
            )
            .await?;
        ok(&entry)?;
        into_the_trash(&mut ada, &booked).await?;
        // Long past the period of the trash, the issue stays there for its time entry.
        a_day_later(&rig, 2 * TRASH_KEPT.as_secs() / SECONDS_PER_DAY).await?;
        let stored = rig
            .storage
            .issue(booked_id)
            .await?
            .ok_or("the issue went")?;
        assert!(stored.deleted_at.is_some(), "it is still in the trash");

        // When the entry is past its keep period, it is still there: only an administrator
        // deletes it (the one entry below). The next purge of the trash then takes the issue.
        a_day_later(&rig, 11 * 366).await?;
        assert!(rig.storage.issue(booked_id).await?.is_some());
        let mut ada = Browser::signed_in_as(&app, ADA_EMAIL).await?;
        assert_eq!(
            ok(&ada.send("POST", DELETE_PAST, None).await?)?["deleted"],
            1
        );
        a_day_later(&rig, 1).await?;
        assert!(rig.storage.issue(booked_id).await?.is_none());
        Ok(())
    })
    .await
}

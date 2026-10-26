//! The notifications on both databases: rows kept once for a person and key, the place of the
//! reader that moves on after them, the e-mail that waits, the keep period, and the encrypted mail
//! password (NTF-01, NTF-02, PRIV-05, SEC-16).

use bb_app::auth::AccountStore;
use bb_app::jobs::{JobStore, QueuedJob};
use bb_app::notifications::NotificationStore;
use bb_app::ports::ChangeStore;
use bb_core::event::FieldChange;
use bb_core::id::{EventId, IssueId, JobId, NotificationId, SequentialIds, UserId};
use bb_core::job::JobState;
use bb_core::mail::PASSWORD_SECRET;
use bb_core::notification::{MailState, Notification, NotificationFilter, NotificationKind, Once};
use bb_core::technical::TechnicalName;
use bb_testing::{TestResult, each_database};
use jiff::SignedDuration;
use sea_query::{Expr, ExprTrait as _, Query};
use serde_json::json;

use super::*;
use crate::schema::Secret;
use crate::test_support::{at, event, fixture, ids, person};

/// A person in the database.
async fn stored_person(
    storage: &AuthStorage,
    source: &mut SequentialIds,
    email: &str,
) -> Result<UserId, StoreError> {
    let who = person(source, email);
    let id = who.id;
    let change = event(source, 1);
    ChangeStore::record_change(storage, &change, move |tx| {
        Box::pin(async move { AuthStorage::insert_person(tx, &who, "hash", at(1)).await })
    })
    .await?;
    Ok(id)
}

fn row(
    source: &mut SequentialIds,
    recipient: UserId,
    issue: Option<IssueId>,
    seconds: i64,
) -> Result<Notification, bb_core::id::IdError> {
    let event = EventId::generate(source)?;
    Ok(Notification {
        id: NotificationId::generate(source)?,
        recipient,
        kind: NotificationKind::FollowedActivity,
        issue,
        event,
        once: Once::Event(event),
        record: None,
        by: None,
        at: at(seconds),
        changes: vec![FieldChange {
            field: "status".to_owned(),
            before: None,
            after: Some(json!("done")),
        }],
        in_app: true,
        mail: MailState::Never,
        read_at: None,
    })
}

#[tokio::test]
async fn ntf_01_a_notification_is_kept_once_for_a_person_and_key_and_the_reader_resumes()
-> TestResult {
    each_database("ntf_01_resumes", async |db| {
        let fixture = fixture(db).await?;
        let storage = fixture.storage.clone();
        let mut source = ids();
        let ana = stored_person(&storage, &mut source, "ana@example.test").await?;
        // An event for the reader to read.
        let changed = event(&mut source, 10);
        ChangeStore::record_change(&storage, &changed, |_| Box::pin(async { Ok(()) })).await?;
        let events = storage.notification_events(10).await?;
        let last = events
            .last()
            .map(|(position, _)| *position)
            .ok_or("no event")?;
        assert!(events.iter().any(|(_, found)| found.id == changed.id));

        let made = Notification {
            event: changed.id,
            once: Once::Event(changed.id),
            ..row(&mut source, ana, None, 10)?
        };
        storage
            .store_notifications(std::slice::from_ref(&made), &[], last, at(11))
            .await?;
        // The reader stopped before its place was recorded and reads the event again: it makes the
        // same row, with an ID of its own, and the store keeps the first.
        let again = Notification {
            id: NotificationId::generate(&mut source)?,
            ..made.clone()
        };
        storage
            .store_notifications(&[again], &[], last, at(12))
            .await?;
        let listed = storage
            .notifications_of(ana, NotificationFilter::All, None, 10)
            .await?;
        assert_eq!(listed, [made]);
        // A new storage over the same database (a restart) finds nothing left to read.
        let restarted = fixture.storage.clone();
        assert!(restarted.notification_events(10).await?.is_empty());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn ntf_01_the_store_lists_newest_first_by_filter_and_marks_as_read() -> TestResult {
    each_database("ntf_01_list", async |db| {
        let fixture = fixture(db).await?;
        let storage = fixture.storage.clone();
        let mut source = ids();
        let ana = stored_person(&storage, &mut source, "ana@example.test").await?;
        let ben = stored_person(&storage, &mut source, "ben@example.test").await?;
        let first = row(&mut source, ana, None, 1)?;
        let mut second = row(&mut source, ana, None, 2)?;
        second.kind = NotificationKind::Mentioned;
        let hidden_from_the_app = Notification {
            in_app: false,
            ..row(&mut source, ana, None, 3)?
        };
        let others = row(&mut source, ben, None, 4)?;
        storage
            .store_notifications(
                &[
                    first.clone(),
                    second.clone(),
                    hidden_from_the_app,
                    others.clone(),
                ],
                &[],
                0,
                at(5),
            )
            .await?;
        let all = storage
            .notifications_of(ana, NotificationFilter::All, None, 10)
            .await?;
        assert_eq!(
            all,
            [second.clone(), first.clone()],
            "newest first, only the app's"
        );
        let mentions = storage
            .notifications_of(ana, NotificationFilter::Mentions, None, 10)
            .await?;
        assert_eq!(mentions, [second.clone()]);
        let before = storage
            .notifications_of(ana, NotificationFilter::All, Some(second.id), 10)
            .await?;
        assert_eq!(before, std::slice::from_ref(&first));
        // Marking is for the person who owns the row.
        let marked = {
            let (recipient, id) = (ben, first.id);
            let change = event(&mut source, 6);
            ChangeStore::record_change(&storage, &change, move |tx| {
                Box::pin(async move {
                    AuthStorage::mark_notifications_read(tx, recipient, &[id], at(6)).await
                })
            })
            .await?
        };
        assert_eq!(marked, 0, "not Ben's notification");
        // *Mark all as read* goes up to the newest the person has seen: a later one stays unread.
        let change = event(&mut source, 7);
        let older = first.id;
        let marked = ChangeStore::record_change(&storage, &change, move |tx| {
            Box::pin(async move {
                AuthStorage::mark_notifications_read_up_to(tx, ana, older, at(7)).await
            })
        })
        .await?;
        assert_eq!(marked, 1);
        assert_eq!(
            storage
                .notifications_of(ana, NotificationFilter::Unread, None, 10)
                .await?,
            [second.clone()]
        );
        let change = event(&mut source, 8);
        let newer = second.id;
        let marked = ChangeStore::record_change(&storage, &change, move |tx| {
            Box::pin(async move {
                AuthStorage::mark_notifications_read_up_to(tx, ana, newer, at(8)).await
            })
        })
        .await?;
        assert_eq!(marked, 1);
        assert!(
            storage
                .notifications_of(ana, NotificationFilter::Unread, None, 10)
                .await?
                .is_empty()
        );
        assert_eq!(
            storage
                .notifications_of(ben, NotificationFilter::Unread, None, 10)
                .await?,
            [others]
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn adr_0007_the_rows_the_jobs_and_the_place_of_the_reader_are_one_step() -> TestResult {
    each_database("adr_0007_one_step", async |db| {
        let fixture = fixture(db).await?;
        let storage = fixture.storage.clone();
        let mut source = ids();
        let ana = stored_person(&storage, &mut source, "ana@example.test").await?;
        let job = QueuedJob {
            id: JobId::generate(&mut source)?,
            kind: TechnicalName::new("mail.send")?,
            payload: json!({"what": "changes"}),
            run_at: at(301),
        };
        let first = row(&mut source, ana, None, 1)?;
        storage
            .store_notifications(&[first], std::slice::from_ref(&job), 5, at(4))
            .await?;
        assert_eq!(
            storage
                .database()
                .outbox_position(&consumer_name(CONSUMER)?)
                .await?,
            5
        );
        // A step that fails half way (the job is in the queue already) leaves nothing of it: no
        // row, and the reader has not moved, so it reads the same events again.
        let second = row(&mut source, ana, None, 2)?;
        let failed = storage
            .store_notifications(
                std::slice::from_ref(&second),
                std::slice::from_ref(&job),
                9,
                at(6),
            )
            .await;
        assert!(failed.is_err(), "the job is there twice");
        assert_eq!(
            storage
                .database()
                .outbox_position(&consumer_name(CONSUMER)?)
                .await?,
            5
        );
        let found = storage
            .notifications_of(ana, NotificationFilter::All, None, 10)
            .await?;
        assert!(found.iter().all(|each| each.id != second.id));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn ntf_02_the_e_mail_that_waits_is_found_by_person_issue_and_time_and_can_be_given_up()
-> TestResult {
    each_database("ntf_02_waiting", async |db| {
        let fixture = fixture(db).await?;
        let storage = fixture.storage.clone();
        let mut source = ids();
        let ana = stored_person(&storage, &mut source, "ana@example.test").await?;
        let issue = IssueId::generate(&mut source)?;
        let other = IssueId::generate(&mut source)?;
        let waiting = |source: &mut SequentialIds, issue: Option<IssueId>, seconds: i64| {
            Ok::<_, bb_core::id::IdError>(Notification {
                mail: MailState::Waiting,
                ..row(source, ana, issue, seconds)?
            })
        };
        let (a, b, c, d) = (
            waiting(&mut source, Some(issue), 1)?,
            waiting(&mut source, Some(issue), 20)?,
            waiting(&mut source, Some(other), 2)?,
            waiting(&mut source, None, 3)?,
        );
        let jobs = [QueuedJob {
            id: JobId::generate(&mut source)?,
            kind: TechnicalName::new("mail.send")?,
            payload: json!({"what": "changes"}),
            run_at: at(301),
        }];
        storage
            .store_notifications(&[a.clone(), b.clone(), c, d], &jobs, 0, at(4))
            .await?;
        // The job is in the queue in the same step as the rows.
        assert_eq!(storage.database().queued_job_count().await?, 1);
        let found = storage.waiting_mail(ana, Some(issue), at(10), 10).await?;
        assert_eq!(
            found,
            std::slice::from_ref(&a),
            "the one at 20 seconds is not due yet"
        );
        let found = storage.waiting_mail(ana, Some(issue), at(30), 10).await?;
        assert_eq!(found, [a.clone(), b.clone()]);
        assert_eq!(storage.waiting_mail(ana, None, at(30), 10).await?.len(), 1);
        storage.set_mail_state(&[a.id], MailState::Sent).await?;
        assert_eq!(
            storage.waiting_mail(ana, Some(issue), at(30), 10).await?,
            std::slice::from_ref(&b)
        );
        assert_eq!(
            storage
                .finish_mail(ana, Some(issue), at(30), MailState::Never)
                .await?,
            1
        );
        assert!(
            storage
                .waiting_mail(ana, Some(issue), at(30), 10)
                .await?
                .is_empty()
        );
        assert_eq!(
            storage
                .waiting_mail(ana, Some(other), at(30), 10)
                .await?
                .len(),
            1
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn priv_05_notifications_before_the_cutoff_are_deleted_and_the_rest_stay() -> TestResult {
    each_database("priv_05_notifications", async |db| {
        let fixture = fixture(db).await?;
        let storage = fixture.storage.clone();
        let mut source = ids();
        let ana = stored_person(&storage, &mut source, "ana@example.test").await?;
        let old = row(&mut source, ana, None, 100)?;
        let new = row(&mut source, ana, None, 200)?;
        storage
            .store_notifications(&[old, new.clone()], &[], 0, at(201))
            .await?;
        assert_eq!(storage.purge_notifications(at(150)).await?, 1);
        assert_eq!(
            storage.purge_notifications(at(150)).await?,
            0,
            "safe to run again"
        );
        assert_eq!(
            storage
                .notifications_of(ana, NotificationFilter::All, None, 10)
                .await?,
            [new]
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn priv_05_the_payload_of_a_dead_job_is_emptied_after_its_keep_period_and_the_job_stays()
-> TestResult {
    each_database("priv_05_dead_payload", async |db| {
        let fixture = fixture(db).await?;
        let storage = fixture.storage.clone();
        let mut source = ids();
        let worker = TechnicalName::new("worker.one")?;
        let kind = TechnicalName::new("mail.send")?;
        let id = JobId::generate(&mut source)?;
        storage
            .queue_job(id, &kind, &json!({"recipient": "ana"}), at(100), at(100))
            .await?;
        let taken = storage
            .lease_jobs(&worker, at(101), SignedDuration::from_secs(60), 5)
            .await?;
        assert_eq!(taken.len(), 1);
        let failure = TechnicalName::new("job.failed")?;
        storage
            .fail_job(
                id,
                &worker,
                &failure,
                bb_core::job::FailureKind::Permanent,
                at(102),
            )
            .await?;
        assert_eq!(
            storage.clear_dead_job_payloads(at(50)).await?,
            0,
            "not old enough"
        );
        assert_eq!(storage.clear_dead_job_payloads(at(1000)).await?, 1);
        assert_eq!(
            storage.clear_dead_job_payloads(at(1000)).await?,
            0,
            "safe to run again"
        );
        let dead = storage.database().dead_jobs(10).await?;
        assert_eq!(dead.len(), 1);
        assert_eq!(dead[0].state, JobState::Dead);
        assert!(dead[0].payload.is_null());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_16_the_mail_password_is_stored_encrypted() -> TestResult {
    each_database("sec_16_mail_password", async |db| {
        let fixture = fixture(db).await?;
        let storage = fixture.storage.clone();
        assert!(storage.mail_password().await?.is_none());
        let password = "a made-up mail password";
        storage
            .put_mail_password(password.as_bytes(), at(1))
            .await?;
        let read = storage.mail_password().await?.ok_or("no password")?;
        assert_eq!(read.as_slice(), password.as_bytes());
        // What the database holds is not the password.
        let name = TechnicalName::new(PASSWORD_SECRET)?;
        assert_eq!(
            storage.database().secret_names().await?,
            std::slice::from_ref(&name)
        );
        let mut select = Query::select();
        select
            .column(Secret::Ciphertext)
            .from(Secret::Table)
            .and_where(Expr::col(Secret::Name).eq(name.as_str()));
        let row = storage
            .database()
            .fetch_optional(&select)
            .await?
            .ok_or("no row")?;
        let ciphertext: Vec<u8> = row.get(Secret::Ciphertext)?;
        let plain = password.as_bytes();
        assert!(
            !ciphertext
                .windows(plain.len())
                .any(|window| window == plain)
        );
        // A new password replaces it, and removing it leaves nothing.
        storage.put_mail_password(b"another", at(2)).await?;
        assert_eq!(
            storage
                .mail_password()
                .await?
                .ok_or("no password")?
                .as_slice(),
            b"another"
        );
        storage.delete_mail_password().await?;
        assert!(storage.mail_password().await?.is_none());
        Ok(())
    })
    .await
}

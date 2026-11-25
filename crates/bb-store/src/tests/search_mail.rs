//! E-mailed searches on both databases (QRY-07): one row of a saved search for a person, the due
//! ones found by the time of their next e-mail (none that waits for its person), the queuing of
//! an e-mail and the move to the next day together or not at all, and the rows that go with the
//! private searches of a removed person.

use bb_app::ports::{BoxFuture, ChangeStore};
use bb_app::saved_searches::SavedSearchStore as _;
use bb_core::id::{IdSource, JobId, SavedSearchId, SearchMailId, UserId};
use bb_core::saved_search::{
    BoardOptions, CalendarOptions, SavedSearch, TimelineOptions, ViewKind,
};
use bb_core::search_mail::{MailDays, MailHour, MailPause, SearchMail};
use bb_core::technical::TechnicalName;
use bb_core::version::FIRST_VERSION;
use bb_testing::{TestResult, each_database};
use serde_json::json;

use super::*;
use crate::test_support::{at, event, fixture, ids};

async fn changed<T: Send>(
    storage: &AuthStorage,
    source: &mut impl IdSource,
    write: impl for<'t> FnOnce(&'t mut Transaction) -> BoxFuture<'t, Result<T, StoreError>> + Send,
) -> Result<T, StoreError> {
    ChangeStore::record_change(storage, &event(source, 5), write).await
}

async fn a_search(
    storage: &AuthStorage,
    source: &mut impl IdSource,
    maker: UserId,
) -> Result<SavedSearchId, StoreError> {
    let search = SavedSearch {
        id: SavedSearchId::generate(source).unwrap(),
        name: "Open bugs".to_owned(),
        text: "status is open".to_owned(),
        maker,
        view: ViewKind::List,
        group_by: None,
        timeline: TimelineOptions::default(),
        board: BoardOptions::default(),
        calendar: CalendarOptions::default(),
        created_at: at(1),
        updated_at: at(1),
        deleted_at: None,
        version: FIRST_VERSION,
    };
    let id = search.id;
    changed(storage, source, move |tx| {
        Box::pin(async move { AuthStorage::insert_saved_search(tx, &search, &[]).await })
    })
    .await?;
    Ok(id)
}

fn mail(
    source: &mut impl IdSource,
    holder: UserId,
    search: SavedSearchId,
    next: i64,
) -> SearchMail {
    SearchMail {
        id: SearchMailId::generate(source).unwrap(),
        holder,
        search,
        days: MailDays::new([true, false, false, true, false, false, false]).unwrap(),
        hour: MailHour::new(8).unwrap(),
        only_when_changed: true,
        paused: None,
        next_at: at(next),
        last_sent_at: None,
        created_at: at(1),
        updated_at: at(1),
    }
}

async fn put(
    storage: &AuthStorage,
    source: &mut impl IdSource,
    mail: &SearchMail,
) -> Result<(), StoreError> {
    let stored = mail.clone();
    changed(storage, source, move |tx| {
        Box::pin(async move { AuthStorage::put_search_mail(tx, &stored).await })
    })
    .await
}

fn a_job(source: &mut impl IdSource, run_at: i64) -> QueuedJob {
    QueuedJob {
        id: JobId::generate(source).unwrap(),
        kind: TechnicalName::new("search-mail.send").unwrap(),
        payload: json!({"mail": "x"}),
        run_at: at(run_at),
    }
}

#[tokio::test]
async fn qry_07_an_e_mailed_search_is_kept_and_found_when_it_is_due() -> TestResult {
    each_database("qry_07_store_due", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        let holder = UserId::generate(&mut source);
        let search = a_search(storage, &mut source, holder).await?;
        let other = a_search(storage, &mut source, holder).await?;
        let mut first = mail(&mut source, holder, search, 100);
        put(storage, &mut source, &first).await?;
        let second = mail(&mut source, holder, other, 5000);
        put(storage, &mut source, &second).await?;
        assert_eq!(storage.search_mail(first.id).await?, Some(first.clone()));
        assert_eq!(
            storage.search_mail_of(holder, search).await?,
            Some(first.clone())
        );
        assert_eq!(storage.count_search_mails_of(holder).await?, 2);
        assert_eq!(storage.search_mails_of(holder).await?.len(), 2);
        // Writing it again changes the row, never a second one.
        first.hour = MailHour::new(9).unwrap();
        put(storage, &mut source, &first).await?;
        assert_eq!(storage.count_search_mails_of(holder).await?, 2);
        assert_eq!(storage.search_mail(first.id).await?, Some(first.clone()));

        let due = storage.search_mails_due(at(200), 10).await?;
        assert_eq!(due, [first.clone()]);
        // Queued once: the next e-mail moves on with the job.
        let job = a_job(&mut source, 150);
        assert!(
            storage
                .queue_search_mail(first.id, at(100), at(7000), &job, at(120))
                .await?
        );
        assert!(storage.search_mails_due(at(200), 10).await?.is_empty());
        // A second round with the time it read before queues nothing.
        let again = a_job(&mut source, 150);
        assert!(
            !storage
                .queue_search_mail(first.id, at(100), at(7000), &again, at(120))
                .await?
        );
        // One waiting for its person is never due.
        storage
            .pause_search_mail(second.id, MailPause::NoLongerOpens)
            .await?;
        assert_eq!(storage.search_mails_due(at(9000), 10).await?.len(), 1);
        let waiting = storage.search_mail(second.id).await?.unwrap();
        assert_eq!(waiting.paused, Some(MailPause::NoLongerOpens));
        storage.search_mail_sent(first.id, at(160)).await?;
        assert_eq!(
            storage.search_mail(first.id).await?.unwrap().last_sent_at,
            Some(at(160))
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn priv_04_the_e_mailed_searches_of_a_private_search_go_with_it() -> TestResult {
    each_database("qry_07_store_removal", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        let holder = UserId::generate(&mut source);
        let search = a_search(storage, &mut source, holder).await?;
        let made = mail(&mut source, holder, search, 100);
        put(storage, &mut source, &made).await?;
        let gone = changed(storage, &mut source, move |tx| {
            Box::pin(async move { AuthStorage::delete_private_searches_of(tx, holder).await })
        })
        .await?;
        assert_eq!(gone, [search]);
        assert_eq!(storage.search_mail(made.id).await?, None);
        // Stopping one that is gone says so.
        let deleted = changed(storage, &mut source, move |tx| {
            Box::pin(async move { AuthStorage::delete_search_mail(tx, made.id).await })
        })
        .await?;
        assert!(!deleted);
        Ok(())
    })
    .await
}

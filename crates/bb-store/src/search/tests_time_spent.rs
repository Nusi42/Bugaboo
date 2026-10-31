//! *Time spent* and the worked-out *Time left* in the search and the totals, on both databases
//! (TIME-03, PRIV-13, ADR-0035 decision 4): the compiled *Time left* gives what `bb-core` gives for
//! every case of the vectors, *Time spent* is searched in hours and minutes and grouped in ranges,
//! and the totals count the time of the issues the reader sees only.

use bb_app::ports::ChangeStore;
use bb_app::search::{FieldTarget, GroupKey, SearchStore as _, Searchable};
use bb_app::time::TimeStore as _;
use bb_app::time::spent::TimeSpentStore as _;
use bb_core::id::{IssueId, TimeEntryId};
use bb_core::time::Minutes;
use bb_core::time::entry::TimeEntry;
use bb_core::time::range::LengthRange;
use bb_core::version::FIRST_VERSION;
use bb_searchlang::{Field, FieldKind};
use bb_testing::{TestResult, each_database};
use sea_query::{Expr, ExprTrait as _, Iden, Query};

use super::tests::{Data, request_with, seed};
use super::tests_more::titles_of;
use crate::AuthStorage;
use crate::db::StoredId as _;
use crate::schema::{IssueRow, TimeEntryRow};
use crate::test_support::{at, event};

/// The three lengths of an issue, for a person who may read them all.
fn lengths() -> Vec<Searchable> {
    [
        ("estimated time", FieldTarget::EstimatedTime),
        ("time left", FieldTarget::TimeLeft),
        ("time spent", FieldTarget::TimeSpent),
    ]
    .map(|(word, target)| Searchable {
        field: Field::new(word, FieldKind::Duration),
        target,
    })
    .to_vec()
}

/// Sets the estimate and the typed time left of an issue, as stored.
async fn set_lengths(
    storage: &AuthStorage,
    issue: IssueId,
    estimated: Option<i64>,
    typed: Option<i64>,
) -> Result<(), crate::StoreError> {
    let mut update = Query::update();
    update
        .table(IssueRow::Table)
        .value(IssueRow::EstimatedTime, estimated)
        .value(IssueRow::TimeLeft, typed)
        .and_where(Expr::col(IssueRow::Id).eq(issue.stored()));
    storage.database().execute(&update).await.map(drop)
}

/// Books `minutes` on an issue, as one time entry of the person of the tests.
async fn book(data: &mut Data, storage: &AuthStorage, issue: IssueId, minutes: u32) -> TestResult {
    let entry = TimeEntry {
        id: TimeEntryId::generate(&mut data.source)?,
        person: data.me,
        issue,
        date: jiff::civil::date(2026, 1, 2),
        minutes: Minutes::new(minutes),
        note: String::new(),
        billable: false,
        started_at: None,
        created_at: at(1),
        version: FIRST_VERSION,
    };
    ChangeStore::record_change(storage, &event(&mut data.source, 7), move |tx| {
        Box::pin(async move { AuthStorage::insert_time_entry(tx, &entry).await })
    })
    .await?;
    Ok(())
}

/// Takes every time entry away.
async fn no_entries(storage: &AuthStorage) -> Result<(), crate::StoreError> {
    let mut delete = Query::delete();
    delete.from_table(TimeEntryRow::Table);
    storage.database().execute(&delete).await.map(drop)
}

/// The name of the column of the compiled *Time left*.
#[derive(Clone, Copy, Iden)]
enum Shown {
    Left,
}

/// The compiled *Time left* of one issue, as the database works it out.
async fn compiled_left(
    storage: &AuthStorage,
    issue: IssueId,
) -> Result<Option<i64>, crate::StoreError> {
    let mut select = Query::select();
    select
        .expr_as(super::spent::time_left(), Shown::Left)
        .from(IssueRow::Table)
        .and_where(Expr::col((IssueRow::Table, IssueRow::Id)).eq(issue.stored()));
    let row = storage.database().fetch_optional(&select).await?;
    row.map_or(Ok(None), |row| row.get::<Option<i64>>(Shown::Left))
}

#[tokio::test]
async fn time_03_the_compiled_time_left_matches_the_vectors() -> TestResult {
    let file: serde_json::Value =
        serde_json::from_str(include_str!("../../../../tools/testdata/time-left.json"))?;
    let cases = file["cases"].as_array().cloned().unwrap_or_default();
    assert!(cases.len() > 5, "the vectors have cases");
    each_database("time_03_time_left_vectors", async |db| {
        let fixture = crate::test_support::fixture(db).await?;
        let storage = &fixture.storage;
        let mut data = seed(storage).await;
        let issue = data.issues[0].id;
        for case in &cases {
            no_entries(storage).await?;
            set_lengths(
                storage,
                issue,
                case["estimated"].as_i64(),
                case["typed"].as_i64(),
            )
            .await?;
            let spent = u32::try_from(case["spent"].as_u64().unwrap_or_default())?;
            if spent > 0 {
                book(&mut data, storage, issue, spent).await?;
            }
            assert_eq!(
                compiled_left(storage, issue).await?,
                case["left"].as_i64(),
                "{case}"
            );
        }
        Ok(())
    })
    .await
}

#[tokio::test]
async fn time_03_time_spent_and_time_left_are_searched_sorted_and_grouped() -> TestResult {
    each_database("time_03_spent_search", async |db| {
        let fixture = crate::test_support::fixture(db).await?;
        let storage = &fixture.storage;
        let mut data = seed(storage).await;
        let (printer, login, hidden) = (data.issues[0].id, data.issues[1].id, data.hidden.id);
        for (issue, minutes) in [(printer, 60), (printer, 30), (login, 30), (hidden, 500)] {
            book(&mut data, storage, issue, minutes).await?;
        }
        set_lengths(storage, login, Some(120), None).await?;
        let found = |text: &str| {
            let request = request_with(&data, text, None, 100, lengths(), Vec::new());
            async move { titles_of(storage, &request).await.unwrap_or_default() }
        };
        assert_eq!(found("time spent > 1 hour").await, ["Printer jams"]);
        assert_eq!(
            found("time spent >= 30 minutes sort by time spent, highest first").await,
            ["Printer jams", "Login broken"]
        );
        assert_eq!(found("time spent is 0 minutes").await.len(), 4);
        // Login broken: 2 hours estimated, 30 minutes spent, so 1 hour 30 minutes left.
        assert_eq!(
            found("time left is 1 hour 30 minutes").await,
            ["Login broken"]
        );
        assert_eq!(found("time left is empty").await.len(), 5);
        let groups = storage
            .search_groups(
                &request_with(&data, "", None, 1, lengths(), Vec::new()),
                "time spent",
            )
            .await
            .map_err(|error| format!("{error:?}"))?;
        let ranges = LengthRange::all();
        let counted: Vec<(GroupKey, u64)> = groups
            .into_iter()
            .map(|group| (group.key, group.issues))
            .collect();
        // Every issue has a time spent, 0 for one without entries; the hidden one is in no group.
        assert_eq!(
            counted,
            [
                (GroupKey::Length(ranges[0]), 5),
                (GroupKey::Length(ranges[1]), 1),
            ]
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn time_03_the_totals_add_the_time_left_and_the_time_spent_of_every_level() -> TestResult {
    each_database("time_03_totals_spent", async |db| {
        let fixture = crate::test_support::fixture(db).await?;
        let storage = &fixture.storage;
        let mut data = seed(storage).await;
        // Under Login broken: Docs typo (4 hours, nothing spent) and, under it, Crash on save (10
        // hours, 100 minutes spent). Under Printer jams only the hidden issue, with Slow page.
        let (docs, crash) = (data.issues[2].id, data.issues[4].id);
        set_lengths(storage, docs, Some(240), None).await?;
        set_lengths(storage, crash, Some(600), None).await?;
        book(&mut data, storage, crash, 100).await?;
        let (hidden, slow) = (data.hidden.id, data.issues[3].id);
        book(&mut data, storage, hidden, 700).await?;
        book(&mut data, storage, slow, 50).await?;
        let below = request_with(&data, "under ACME-2", None, 1, lengths(), Vec::new());
        let lengths_below = storage
            .search_lengths(&below)
            .await
            .map_err(|e| format!("{e:?}"))?;
        assert_eq!(lengths_below.estimated, Some(Minutes::new(840)));
        assert_eq!(lengths_below.time_left, Some(Minutes::new(240 + 500)));
        let spent = storage
            .spent_on_search(&below)
            .await
            .map_err(|e| format!("{e:?}"))?;
        assert_eq!(spent, Minutes::new(100));
        // The hidden issue and what sits under it count for nobody who cannot see them.
        let hidden = request_with(&data, "under ACME-1", None, 1, lengths(), Vec::new());
        assert!(
            !storage
                .search_lengths(&hidden)
                .await
                .map_err(|e| format!("{e:?}"))?
                .found_any
        );
        assert_eq!(
            storage
                .spent_on_search(&hidden)
                .await
                .map_err(|e| format!("{e:?}"))?,
            Minutes::new(0)
        );
        let project = request_with(&data, "", None, 1, lengths(), Vec::new());
        assert_eq!(
            storage
                .spent_on_search(&project)
                .await
                .map_err(|e| format!("{e:?}"))?,
            Minutes::new(150)
        );
        Ok(())
    })
    .await
}

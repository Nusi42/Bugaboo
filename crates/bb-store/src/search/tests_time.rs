//! *Estimated time* in the search and in the totals, on both databases (TIME-03, PRIV-13): searched in
//! hours and minutes, sorted, grouped in ranges, and summed over the issues the reader sees only,
//! every level below an issue counted.

use bb_app::search::{FieldTarget, GroupKey, LengthTotals, SearchStore as _, Searchable};
use bb_core::time::Minutes;
use bb_core::time::range::LengthRange;
use bb_searchlang::{Field, FieldKind};
use bb_testing::{TestResult, each_database};
use sea_query::{Expr, ExprTrait as _, Query};

use super::tests::{Data, request_with, seed};
use super::tests_more::titles_of;
use crate::AuthStorage;
use crate::db::StoredId as _;
use crate::schema::IssueRow;

/// The estimates of the issues of the tests, in minutes, in the order of `Data::issues`: Printer
/// jams, Login broken, Docs typo, Slow page, Crash on save and Idea (none); the hidden issue has the
/// longest of all.
const ESTIMATES: [Option<i64>; 6] = [Some(30), Some(90), Some(240), Some(7), Some(600), None];
const HIDDEN_ESTIMATE: i64 = 1_000;

/// The field *Estimated time* for the person of the tests.
fn estimated() -> Searchable {
    Searchable {
        field: Field::new("estimated time", FieldKind::Duration),
        target: FieldTarget::EstimatedTime,
    }
}

/// The issues of the tests with their estimates.
async fn estimated_data(storage: &AuthStorage) -> Result<Data, crate::StoreError> {
    let data = seed(storage).await;
    let all = data
        .issues
        .iter()
        .zip(ESTIMATES)
        .chain([(&data.hidden, Some(HIDDEN_ESTIMATE))]);
    for (issue, minutes) in all {
        let mut update = Query::update();
        update
            .table(IssueRow::Table)
            .value(IssueRow::EstimatedTime, minutes)
            .and_where(Expr::col(IssueRow::Id).eq(issue.id.stored()));
        storage.database().execute(&update).await?;
    }
    Ok(data)
}

/// The titles a search finds, for a person who may read *Estimated time*.
async fn found(storage: &AuthStorage, data: &Data, text: &str) -> Vec<String> {
    titles_of(
        storage,
        &request_with(data, text, None, 100, vec![estimated()], Vec::new()),
    )
    .await
    .unwrap_or_default()
}

/// The totals of the lengths of a search, for a person who may read *Estimated time* (`with`) or
/// not.
async fn totals(storage: &AuthStorage, data: &Data, text: &str, with: bool) -> LengthTotals {
    let more = if with { vec![estimated()] } else { Vec::new() };
    storage
        .search_lengths(&request_with(data, text, None, 1, more, Vec::new()))
        .await
        .unwrap_or_default()
}

#[tokio::test]
async fn time_03_estimated_time_is_searched_in_hours_and_minutes_sorted_and_grouped_in_ranges()
-> TestResult {
    each_database("time_03_estimated_search", async |db| {
        let fixture = crate::test_support::fixture(db).await?;
        let storage = &fixture.storage;
        let data = estimated_data(storage).await?;
        assert_eq!(
            found(storage, &data, "estimated time > 1 hour sort by title").await,
            ["Crash on save", "Docs typo", "Login broken"]
        );
        assert_eq!(
            found(storage, &data, "estimated time is 30 minutes").await,
            ["Printer jams"]
        );
        assert_eq!(
            found(
                storage,
                &data,
                "estimated time <= 1 hour 30 minutes sort by title"
            )
            .await,
            ["Login broken", "Printer jams", "Slow page"]
        );
        assert_eq!(
            found(storage, &data, "estimated time is empty").await,
            ["Idea"]
        );
        assert_eq!(
            found(storage, &data, "sort by estimated time, highest first").await,
            [
                "Crash on save",
                "Docs typo",
                "Login broken",
                "Printer jams",
                "Slow page",
                "Idea"
            ]
        );
        // The groups are ranges, the shortest first and the issues without an estimate last; the
        // hidden issue is in none of them.
        let groups = storage
            .search_groups(
                &request_with(&data, "", None, 1, vec![estimated()], Vec::new()),
                "estimated time",
            )
            .await
            .map_err(|error| format!("{error:?}"))?;
        let ranges = LengthRange::all();
        let counted: Vec<(GroupKey, u64)> = groups
            .into_iter()
            .map(|group| (group.key, group.issues))
            .collect();
        assert_eq!(
            counted,
            [
                (GroupKey::Length(ranges[0]), 2),
                (GroupKey::Length(ranges[1]), 1),
                (GroupKey::Length(ranges[2]), 1),
                (GroupKey::Length(ranges[3]), 1),
                (GroupKey::Empty, 1),
            ]
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn time_03_the_totals_of_an_issue_count_its_sub_issues_at_every_level() -> TestResult {
    each_database("time_03_totals_every_level", async |db| {
        let fixture = crate::test_support::fixture(db).await?;
        let storage = &fixture.storage;
        let data = estimated_data(storage).await?;
        // Docs typo sits under Login broken, and Crash on save under Docs typo.
        let below = totals(storage, &data, "under ACME-2", true).await;
        assert!(below.found_any);
        assert_eq!(below.estimated, Some(Minutes::new(240 + 600)));
        // Time left is not among the fields of this person: never summed.
        assert_eq!(below.time_left, None);
        // An issue with nothing below it.
        let nothing = totals(storage, &data, "under ACME-6", true).await;
        assert_eq!(nothing, LengthTotals::default());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn priv_13_a_total_never_counts_an_issue_the_reader_cannot_see() -> TestResult {
    each_database("priv_13_totals_sight", async |db| {
        let fixture = crate::test_support::fixture(db).await?;
        let storage = &fixture.storage;
        let data = estimated_data(storage).await?;
        // The hidden issue sits under Printer jams, with Slow page under it: the reader sees no
        // sub-issue of Printer jams, so the total has nothing.
        assert!(!totals(storage, &data, "under ACME-1", true).await.found_any);
        // The project counts the issues the reader sees, not the hidden one in SECRET.
        let project = totals(storage, &data, "", true).await;
        assert_eq!(
            project.estimated,
            Some(Minutes::new(30 + 90 + 240 + 7 + 600))
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn priv_13_a_total_of_a_field_the_reader_may_not_read_is_not_summed() -> TestResult {
    each_database("priv_13_totals_fields", async |db| {
        let fixture = crate::test_support::fixture(db).await?;
        let storage = &fixture.storage;
        let data = estimated_data(storage).await?;
        let without = totals(storage, &data, "", false).await;
        assert!(without.found_any);
        assert_eq!((without.estimated, without.time_left), (None, None));
        Ok(())
    })
    .await
}

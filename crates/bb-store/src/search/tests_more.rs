//! More of the compiler on both databases: what a hidden issue cannot show through, where the values
//! of an own field count, a day that is 23 hours long, a search that is dropped half way.

use bb_app::access::PolicyStore as _;
use bb_app::fields::{Definition, FieldStore as _};
use bb_app::issues::IssueStore as _;
use bb_app::ports::ChangeStore;
use bb_app::search::{FieldTarget, OwnReach, Searchable};
use bb_app::search::{GroupKey, SearchStore as _};
use bb_core::event::{EntityKind, RecordKind};
use bb_core::id::{EntityId, FieldId, ProjectId};
use bb_core::own_fields::{FieldFor, FieldType, Look, OwnField, Rules, Scope};
use bb_core::own_fields_value::Cell;
use bb_core::project::{Project, ShortName};
use bb_core::version::FIRST_VERSION;
use bb_searchlang::{Field, FieldKind};
use bb_testing::{TestResult, each_database};

use super::tests::{request, request_with, seed, titles};
use crate::AuthStorage;
use crate::test_support::{event, fixture, issue};

#[tokio::test]
async fn qry_05_an_issue_with_a_hidden_parent_has_no_parent_for_a_search() -> TestResult {
    each_database("search_hidden_parent", async |db| {
        let fixture = fixture(db).await?;
        let data = seed(&fixture.storage).await;
        let storage = &fixture.storage;
        // Slow page sits under an issue the person does not read: that is no parent for them.
        let none = titles(storage, &data, "parent is empty sort by title").await;
        assert!(none.contains(&"Slow page".to_owned()), "{none:?}");
        let some = titles(storage, &data, "parent is not empty sort by title").await;
        assert_eq!(some, ["Crash on save", "Docs typo"]);
        // Grouping by it names no hidden issue, and puts Slow page with the issues without one.
        let groups = storage
            .search_groups(&request(&data, "", None, 1), "parent")
            .await
            .map_err(|error| format!("{error:?}"))?;
        assert!(
            groups
                .iter()
                .all(|group| group.key != GroupKey::Issue(data.hidden.id))
        );
        let empty = groups.iter().find(|group| group.key == GroupKey::Empty);
        assert_eq!(empty.map(|group| group.issues), Some(4));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn qry_04_a_walk_never_goes_through_an_issue_the_person_does_not_read() -> TestResult {
    each_database("search_hidden_middle", async |db| {
        let fixture = fixture(db).await?;
        let data = seed(&fixture.storage).await;
        let storage = &fixture.storage;
        // Printer jams blocks the hidden issue, which blocks Slow page: Slow page is reached only
        // through an issue the person does not read.
        let connected = titles(storage, &data, "connected to ACME-1 sort by title").await;
        assert_eq!(connected, ["Idea", "Login broken"]);
        let below = titles(storage, &data, "under ACME-1 sort by title").await;
        assert!(below.is_empty(), "{below:?}");
        let blocked = titles(storage, &data, "blocked by ACME-1 sort by title").await;
        assert_eq!(blocked, ["Login broken"]);
        // The hidden issue cannot be named either: no answer tells it from one that is not there.
        let named = titles(storage, &data, "connected to SECRET-1").await;
        let missing = titles(storage, &data, "connected to SECRET-99").await;
        assert_eq!(named, missing);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_17_a_search_text_with_a_walk_in_front_never_becomes_sql() -> TestResult {
    each_database("search_injection_walk", async |db| {
        let fixture = fixture(db).await?;
        let data = seed(&fixture.storage).await;
        let nasty = "x'); DROP TABLE issue; -- \" \\ ; /* */";
        let escaped = nasty.replace('\\', "\\\\").replace('"', "\\\"");
        let text = format!("under ACME-1 or title is \"{escaped}\"");
        let ask = request(&data, &text, None, 10);
        let dialect = fixture.storage.database().dialect();
        let plan = super::statements::SortPlan::of(&ask, dialect).map_err(|e| e.to_string())?;
        let built = super::statements::page_statement(&ask, dialect, &plan, None)
            .map_err(|error| error.to_string())?;
        let super::statements::Built::With(query) = &built else {
            return Err("a walk is a statement with a head".into());
        };
        let (sql, values) = query.build(sea_query::SqliteQueryBuilder);
        assert!(sql.starts_with("WITH RECURSIVE"), "{sql}");
        assert!(!sql.contains("DROP") && !sql.contains(nasty), "{sql}");
        assert!(values.0.iter().any(
            |value| matches!(value, sea_query::Value::String(Some(text)) if text.as_str() == nasty)
        ));
        assert!(fixture.storage.search_page(&ask).await.is_ok());
        assert_eq!(titles(&fixture.storage, &data, "").await.len(), 6);
        Ok(())
    })
    .await
}

/// A count that has to go through `up_to` numbers.
fn counting(up_to: i64) -> sea_query::WithQuery {
    use sea_query::{
        Alias, CommonTableExpression, Expr, ExprTrait as _, Func, Query, UnionType, WithClause,
    };
    let numbers = Alias::new("numbers");
    let mut first = Query::select();
    first.expr(Expr::val(1_i64));
    let mut next = Query::select();
    next.expr(Expr::col(Alias::new("n")).add(1_i64))
        .from(numbers.clone())
        .and_where(Expr::col(Alias::new("n")).lt(up_to));
    first.union(UnionType::All, next);
    let mut table = CommonTableExpression::new();
    table
        .table_name(numbers.clone())
        .columns([Alias::new("n")])
        .query(first);
    let mut clause = WithClause::new();
    clause.recursive(true).cte(table);
    let mut count = Query::select();
    count
        .expr(Func::count(Expr::col(Alias::new("n"))))
        .from(numbers);
    count.with(clause)
}

#[tokio::test]
async fn sec_17_a_search_that_is_dropped_leaves_no_limit_on_the_connection() -> TestResult {
    use std::time::Duration;

    use crate::Database;

    // A pool of one connection: the connection the dropped read used is the one the next read gets.
    let folder = tempfile::tempdir()?;
    let database = Database::single_connection(&folder.path().join("one.db")).await?;
    let slow = counting(2_000_000_000);
    // The future is dropped while the read runs, as when a client goes away.
    let dropped = tokio::time::timeout(
        Duration::from_millis(100),
        database.fetch_all_within(&slow, Duration::from_millis(300)),
    )
    .await;
    assert!(dropped.is_err(), "the read should still be running");
    // The dropped read is stopped at its next step and its connection closed, so the next read on
    // the pool of one connection is not kept waiting by it and is not stopped by its limit.
    let started = std::time::Instant::now();
    let tiny = database.fetch_all(&counting(10)).await?;
    assert_eq!(tiny.len(), 1);
    assert!(
        started.elapsed() < Duration::from_millis(500),
        "the dropped read kept the connection for {:?}",
        started.elapsed()
    );
    for _ in 0..3 {
        let rows = database.fetch_all(&counting(3_000_000)).await?;
        assert_eq!(rows.len(), 1);
    }
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "a dropped read kept the connection: {:?}",
        started.elapsed()
    );
    Ok(())
}

/// A text field *Rack*, and a second project OTHER with an issue *Outside*; the issue *Printer jams*
/// of ACME and *Outside* both hold the value `a1`. Says the field and the project.
async fn rack_in_two_projects(
    storage: &AuthStorage,
    data: &mut super::tests::Data,
) -> Result<(FieldId, ProjectId), Box<dyn std::error::Error + Send + Sync>> {
    let field = FieldId::generate(&mut data.source)?;
    let other = ProjectId::generate(&mut data.source)?;
    let mut outside = issue(&mut data.source, other, data.issue_type, 1);
    outside.title = "Outside".to_owned();
    let definition = Definition {
        field: OwnField {
            id: field,
            name: "Rack".to_owned(),
            help: String::new(),
            kind: FieldType::Text,
            applies_to: FieldFor::Issues,
            rules: Rules::default(),
            default_value: String::new(),
            look: Look::starting(FieldType::Text),
            never_to_assistant: false,
            position: 0,
            retired_at: None,
            version: FIRST_VERSION,
        },
        scope: Scope::default(),
        choices: Vec::new(),
    };
    let row = Project {
        id: other,
        parent: None,
        short_name: ShortName::new("OTHER")?,
        name: "Other".to_owned(),
        version: FIRST_VERSION,
    };
    let placed = [data.issues[0].id, outside.id];
    let ev = event(&mut data.source, 7);
    ChangeStore::record_change(storage, &ev, move |tx| {
        Box::pin(async move {
            AuthStorage::insert_project(tx, &row).await?;
            AuthStorage::insert_issue(tx, &outside, &[]).await?;
            AuthStorage::insert_field(tx, &definition).await?;
            for issue in placed {
                let cells = [Cell {
                    text: Some("a1".to_owned()),
                    number: None,
                    date: None,
                    reference: None,
                }];
                AuthStorage::put_field_value(
                    tx,
                    EntityKind::from(RecordKind::Issue),
                    EntityId::from(issue),
                    field,
                    &cells,
                )
                .await?;
            }
            Ok(())
        })
    })
    .await?;
    Ok((field, other))
}

#[tokio::test]
async fn qry_05_a_value_of_an_own_field_counts_only_where_the_field_reaches() -> TestResult {
    each_database("search_own_reach", async |db| {
        let fixture = fixture(db).await?;
        let mut data = seed(&fixture.storage).await;
        let (field, other) = rack_in_two_projects(&fixture.storage, &mut data).await?;
        let rack = Searchable {
            field: Field::new("rack", FieldKind::Text),
            target: FieldTarget::Own {
                field,
                kind: FieldType::Text,
            },
        };
        let ask = |reach: Vec<ProjectId>| {
            let mut ask = request_with(
                &data,
                "rack is a1 sort by title",
                None,
                10,
                vec![rack.clone()],
                vec![OwnReach {
                    field,
                    projects: reach,
                    issue_types: None,
                }],
            );
            // The person reads both projects; the field reaches only some of them.
            ask.sight.all.push(other);
            ask
        };
        let storage = &fixture.storage;
        let found = |ask| async move {
            storage
                .search_page(&ask)
                .await
                .map(|hits| {
                    hits.into_iter()
                        .map(|hit| hit.issue.title)
                        .collect::<Vec<_>>()
                })
                .map_err(|error| format!("{error:?}"))
        };
        assert_eq!(found(ask(vec![data.project])).await?, ["Printer jams"]);
        assert_eq!(
            found(ask(vec![data.project, other])).await?,
            ["Outside", "Printer jams"]
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn qry_04_a_day_of_twenty_three_hours_is_a_day_in_the_zone_of_the_person() -> TestResult {
    use bb_app::ports::ChangeStore;
    use jiff::Timestamp;
    use jiff::tz::TimeZone;

    each_database("search_dst_day", async |db| {
        let fixture = fixture(db).await?;
        let mut data = seed(&fixture.storage).await;
        // In Berlin the clocks went forward on 2026-03-29, so that day has 23 hours and the day
        // before today starts at 23:00 UTC of the 28th and ends at 22:00 UTC of the 29th.
        let moments = [
            ("a", "2026-03-28T22:59:59Z"),
            ("b", "2026-03-28T23:00:00Z"),
            ("c", "2026-03-29T21:59:59Z"),
            ("d", "2026-03-29T22:00:00Z"),
        ];
        let mut made = Vec::new();
        for (number, (title, at_time)) in moments.iter().enumerate() {
            let mut each = issue(
                &mut data.source,
                data.project,
                data.issue_type,
                100 + i64::try_from(number)?,
            );
            each.title = (*title).to_owned();
            let time: Timestamp = at_time.parse()?;
            each.created_at = time;
            each.updated_at = time;
            made.push(each);
        }
        let ev = event(&mut data.source, 8);
        let rows = made.clone();
        ChangeStore::record_change(&fixture.storage, &ev, move |tx| {
            Box::pin(async move {
                for each in &rows {
                    AuthStorage::insert_issue(tx, each, &[]).await?;
                }
                Ok(())
            })
        })
        .await?;
        let mut ask = request(&data, "created is yesterday sort by title", None, 10);
        ask.now = "2026-03-30T10:00:00Z".parse()?;
        ask.zone = TimeZone::get("Europe/Berlin")?;
        let hits = fixture
            .storage
            .search_page(&ask)
            .await
            .map_err(|error| format!("{error:?}"))?;
        let titles: Vec<_> = hits.into_iter().map(|hit| hit.issue.title).collect();
        assert_eq!(titles, ["b", "c"]);
        // The same days seen from UTC are other days.
        ask.zone = TimeZone::UTC;
        let hits = fixture
            .storage
            .search_page(&ask)
            .await
            .map_err(|error| format!("{error:?}"))?;
        let titles: Vec<_> = hits.into_iter().map(|hit| hit.issue.title).collect();
        assert_eq!(titles, ["c", "d"]);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn qry_05_the_list_of_a_project_never_shows_an_issue_of_a_hidden_type() -> TestResult {
    each_database("search_list_hidden_type", async |db| {
        let fixture = fixture(db).await?;
        let data = seed(&fixture.storage).await;
        let storage = &fixture.storage;
        // The list of a project is the search with its sight, newest first.
        let mut list = request(&data, "", None, 100);
        let shown = storage
            .search_page(&list)
            .await
            .map_err(|error| format!("{error:?}"))?;
        assert_eq!(shown.len(), 6);
        assert!(
            shown
                .windows(2)
                .all(|pair| pair[0].issue.created_at >= pair[1].issue.created_at)
        );
        // Where the sight hides the type, not one issue of it is listed.
        list.sight.hidden = vec![(data.project, data.issue_type)];
        let none = storage
            .search_page(&list)
            .await
            .map_err(|error| format!("{error:?}"))?;
        assert!(none.is_empty(), "{}", none.len());
        // The dump of the sample data asks for a whole project in pages and reads each issue once.
        let mut seen = Vec::new();
        let mut after = None;
        loop {
            let request = bb_app::search::SearchRequest::whole_project(data.project, after, 4)
                .map_err(|mistakes| format!("{mistakes:?}"))?;
            let page = storage
                .search_page(&request)
                .await
                .map_err(|error| format!("{error:?}"))?;
            after = page.last().map(|hit| hit.issue.id);
            let full = page.len() == 4;
            seen.extend(page.into_iter().map(|hit| hit.issue.id));
            if !full {
                break;
            }
        }
        assert_eq!(seen.len(), 6);
        seen.sort();
        seen.dedup();
        assert_eq!(seen.len(), 6);
        Ok(())
    })
    .await
}

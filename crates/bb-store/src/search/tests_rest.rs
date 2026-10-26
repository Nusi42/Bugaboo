//! More of the compiler on both databases: `member of`, `within` over a subtree, own fields of
//! numbers and dates, a field with many values sorted the other way round, a cursor that stands
//! for nothing the person reads, and paging through issues whose parent is hidden.

use bb_app::access::PolicyStore as _;
use bb_app::auth::AccountStore as _;
use bb_app::fields::FieldStore as _;
use bb_app::issues::IssueStore as _;
use bb_app::ports::ChangeStore;
use bb_app::search::{FieldTarget, OwnReach, ProjectLink, SearchFailure, SearchStore as _};
use bb_core::access::{Group, GroupMember};
use bb_core::event::{EntityKind, RecordKind};
use bb_core::id::{EntityId, FieldId, GroupId, IssueId, ProjectId};
use bb_core::names::name_key;
use bb_core::own_fields::FieldType;
use bb_core::own_fields_value::Cell;
use bb_core::project::{Project, ShortName};
use bb_core::version::FIRST_VERSION;
use bb_searchlang::{Field, FieldKind};
use bb_testing::{TestDatabase, TestResult, each_database};

use super::tests::{Data, request, request_with, seed, titles};
use super::tests_more::a_field_in_two_projects;
use crate::AuthStorage;
use crate::test_support::{at, event, fixture, issue};

#[tokio::test]
async fn qry_04_member_of_finds_the_issues_of_the_people_of_a_group() -> TestResult {
    each_database("search_member_of", async |db| {
        let fixture = fixture(db).await?;
        let mut data = seed(&fixture.storage).await;
        let storage = &fixture.storage;
        // A person who is in the group reported one issue more.
        let group = GroupId::generate(&mut data.source)?;
        let checker = crate::test_support::person(&mut data.source, "checker@example.test");
        let mut reported = issue(&mut data.source, data.project, data.issue_type, 20);
        reported.title = "Reported".to_owned();
        reported.reported_by = checker.id;
        let row = Group {
            id: group,
            name: "Checkers".to_owned(),
            version: FIRST_VERSION,
        };
        let member = GroupMember {
            group,
            person: checker.id,
            added_at: at(1),
        };
        let ev = event(&mut data.source, 9);
        ChangeStore::record_change(storage, &ev, move |tx| {
            Box::pin(async move {
                AuthStorage::insert_person(tx, &checker, "hash", at(0)).await?;
                AuthStorage::insert_issue(tx, &reported, &[]).await?;
                AuthStorage::insert_group(tx, &row).await?;
                AuthStorage::insert_group_member(tx, &member).await
            })
        })
        .await?;
        let reported_by = bb_app::search::Searchable {
            field: Field::new("reportedBy", FieldKind::Person),
            target: FieldTarget::ReportedBy,
        };
        let mut ask = request_with(
            &data,
            "reportedBy member of \"Checkers\"",
            None,
            10,
            vec![reported_by],
            Vec::new(),
        );
        ask.names.member_groups = vec![(name_key("Checkers"), vec![group])];
        let found: Vec<_> = storage
            .search_page(&ask)
            .await
            .map_err(|error| format!("{error:?}"))?
            .into_iter()
            .map(|hit| hit.issue.title)
            .collect();
        assert_eq!(found, ["Reported"]);
        // A group the person may not ask about is no group: nothing is found, and nothing breaks.
        ask.names.member_groups.clear();
        assert!(titles_of(storage, &ask).await.is_empty());
        Ok(())
    })
    .await
}

async fn titles_of(storage: &AuthStorage, ask: &bb_app::search::SearchRequest) -> Vec<String> {
    storage
        .search_page(ask)
        .await
        .map(|hits| hits.into_iter().map(|hit| hit.issue.title).collect())
        .unwrap_or_default()
}

#[tokio::test]
async fn qry_04_within_finds_a_project_and_the_projects_below_it_that_the_person_sees() -> TestResult
{
    each_database("search_within", async |db| {
        let fixture = fixture(db).await?;
        let mut data = seed(&fixture.storage).await;
        let storage = &fixture.storage;
        let (below, beside) = (
            ProjectId::generate(&mut data.source)?,
            ProjectId::generate(&mut data.source)?,
        );
        let mut inside = issue(&mut data.source, below, data.issue_type, 1);
        inside.title = "Below".to_owned();
        let mut outside = issue(&mut data.source, beside, data.issue_type, 1);
        outside.title = "Beside".to_owned();
        let (acme_name, below_name, beside_name) = (
            ShortName::new("ACME")?,
            ShortName::new("BELOW")?,
            ShortName::new("BESIDE")?,
        );
        let projects = vec![
            Project {
                id: below,
                parent: Some(data.project),
                short_name: below_name.clone(),
                name: "Below".to_owned(),
                version: FIRST_VERSION,
            },
            Project {
                id: beside,
                parent: None,
                short_name: beside_name.clone(),
                name: "Beside".to_owned(),
                version: FIRST_VERSION,
            },
        ];
        let made = [inside, outside];
        let ev = event(&mut data.source, 10);
        ChangeStore::record_change(storage, &ev, move |tx| {
            Box::pin(async move {
                for project in &projects {
                    AuthStorage::insert_project(tx, project).await?;
                }
                for each in &made {
                    AuthStorage::insert_issue(tx, each, &[]).await?;
                }
                Ok(())
            })
        })
        .await?;
        let ask = |text: &str, sees: &[ProjectId]| {
            let mut ask = request(&data, text, None, 20);
            ask.sight.all = sees.to_vec();
            ask.projects = vec![
                ProjectLink {
                    id: data.project,
                    parent: None,
                    short_name: acme_name.clone(),
                },
                ProjectLink {
                    id: below,
                    parent: Some(data.project),
                    short_name: below_name.clone(),
                },
                ProjectLink {
                    id: beside,
                    parent: None,
                    short_name: beside_name.clone(),
                },
            ];
            ask
        };
        let all = [data.project, below, beside];
        let acme = titles_of(storage, &ask("project within ACME sort by title", &all)).await;
        assert_eq!(acme.len(), 7, "{acme:?}");
        assert!(acme.contains(&"Below".to_owned()) && !acme.contains(&"Beside".to_owned()));
        assert_eq!(
            titles_of(storage, &ask("project within BELOW", &all)).await,
            ["Below"]
        );
        // A project below that the person does not see is not looked into.
        let narrow = titles_of(
            storage,
            &ask("project within ACME", &[data.project, beside]),
        )
        .await;
        assert!(!narrow.contains(&"Below".to_owned()));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn qry_05_an_own_number_compares_and_sorts_where_its_field_reaches() -> TestResult {
    let cell = Cell {
        text: None,
        number: Some(5),
        date: None,
        reference: None,
    };
    each_database("search_own_number", async |db| {
        own_field_case(
            db,
            FieldType::WholeNumber,
            cell.clone(),
            "rack < 3",
            "rack >= 5",
        )
        .await
    })
    .await
}

#[tokio::test]
async fn qry_05_an_own_date_compares_and_sorts_where_its_field_reaches() -> TestResult {
    let cell = Cell {
        text: None,
        number: None,
        date: Some(1_800_000_000_000_000),
        reference: None,
    };
    each_database("search_own_date", async |db| {
        own_field_case(
            db,
            FieldType::Date,
            cell.clone(),
            "rack < 2020-01-01",
            "rack > 2020-01-01",
        )
        .await
    })
    .await
}

/// The field `rack` of this kind holds `cell` on two issues, one of them in another project that
/// the person reads. `low` finds none and `high` both.
async fn own_field_case(
    db: &TestDatabase,
    kind: FieldType,
    cell: Cell,
    low: &str,
    high: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let fixture = fixture(db).await?;
    let storage = &fixture.storage;
    let mut data = seed(storage).await;
    let (field, other) = a_field_in_two_projects_with(storage, &mut data, kind, cell).await?;
    let own = bb_app::search::Searchable {
        field: Field::new(
            "rack",
            if kind == FieldType::Date {
                FieldKind::Date
            } else {
                FieldKind::Number
            },
        ),
        target: FieldTarget::Own { field, kind },
    };
    let ask = |text: &str, reach: Vec<ProjectId>| {
        let mut ask = request_with(
            &data,
            text,
            None,
            10,
            vec![own.clone()],
            vec![OwnReach {
                field,
                projects: reach,
                issue_types: None,
            }],
        );
        ask.sight.all.push(other);
        ask
    };
    let both = vec![data.project, other];
    assert!(
        titles_of(storage, &ask(low, both.clone())).await.is_empty(),
        "{low}"
    );
    assert_eq!(
        titles_of(
            storage,
            &ask(&format!("{high} sort by title"), both.clone())
        )
        .await,
        ["Outside", "Printer jams"],
        "{high}"
    );
    // Where the field does not reach, its value is none.
    assert_eq!(
        titles_of(storage, &ask(high, vec![data.project])).await,
        ["Printer jams"]
    );
    let empty = titles_of(storage, &ask("rack is empty", vec![data.project])).await;
    assert!(empty.contains(&"Outside".to_owned()) && !empty.contains(&"Printer jams".to_owned()));
    // Sorting by it: the issues with a value first, the others after.
    let down = titles_of(storage, &ask("sort by rack, highest first", both)).await;
    assert_eq!(down.len(), 7, "{down:?}");
    assert!(down[..2].contains(&"Printer jams".to_owned()));
    Ok(())
}

/// Like `a_field_in_two_projects`, for another word and a cell of its own.
async fn a_field_in_two_projects_with(
    storage: &AuthStorage,
    data: &mut Data,
    kind: FieldType,
    cell: Cell,
) -> Result<(FieldId, ProjectId), Box<dyn std::error::Error + Send + Sync>> {
    let (field, other) = a_field_in_two_projects(storage, data, kind, "unused").await?;
    // The values are written again as the cell asks; the name is the word of the field.
    let issues: Vec<IssueId> = storage_issues(storage, data, other).await?;
    let cells = [cell];
    let ev = event(&mut data.source, 11);
    ChangeStore::record_change(storage, &ev, move |tx| {
        Box::pin(async move {
            for issue in issues {
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

async fn storage_issues(
    storage: &AuthStorage,
    data: &Data,
    other: ProjectId,
) -> Result<Vec<IssueId>, Box<dyn std::error::Error + Send + Sync>> {
    let mut found = vec![data.issues[0].id];
    let mut ask = request(data, "title is \"Outside\"", None, 5);
    ask.sight.all.push(other);
    found.extend(
        storage
            .search_page(&ask)
            .await
            .map_err(|error| format!("{error:?}"))?
            .into_iter()
            .map(|hit| hit.issue.id),
    );
    Ok(found)
}

#[tokio::test]
async fn qry_05_a_field_with_many_values_sorts_by_its_smallest_and_by_its_largest() -> TestResult {
    each_database("search_many_values_z_to_a", async |db| {
        let fixture = fixture(db).await?;
        let data = seed(&fixture.storage).await;
        let storage = &fixture.storage;
        // Printer jams has the tags Bug and UI, Login broken has UI, the others none.
        let up = titles_of(storage, &request(&data, "sort by tags, title", None, 10)).await;
        assert_eq!(up[..2], ["Printer jams", "Login broken"], "{up:?}");
        let down = titles_of(
            storage,
            &request(&data, "sort by tags, Z to A, title", None, 10),
        )
        .await;
        // Both end in UI at most, so both come first; the issues without a tag stay last.
        let first: std::collections::BTreeSet<_> = down[..2].iter().cloned().collect();
        assert_eq!(
            first,
            ["Login broken", "Printer jams"]
                .map(String::from)
                .into_iter()
                .collect()
        );
        assert_eq!(down.len(), 6);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn qry_05_a_cursor_that_is_not_an_issue_the_person_reads_is_one_bad_cursor() -> TestResult {
    each_database("search_bad_cursor", async |db| {
        let fixture = fixture(db).await?;
        let mut data = seed(&fixture.storage).await;
        let storage = &fixture.storage;
        let gone = IssueId::generate(&mut data.source)?;
        let trashed = data.issues[0].id;
        let ev = event(&mut data.source, 12);
        ChangeStore::record_change(storage, &ev, move |tx| {
            Box::pin(async move {
                AuthStorage::set_issue_deleted(tx, trashed, Some(at(5)), at(5))
                    .await
                    .map(|_| ())
            })
        })
        .await?;
        // An issue that does not exist, one in the trash and one the person does not read are
        // answered alike.
        for after in [gone, trashed, data.hidden.id] {
            let failure = storage
                .search_page(&request(&data, "", Some(after), 3))
                .await
                .err();
            assert_eq!(failure, Some(SearchFailure::BadCursor), "{after}");
        }
        Ok(())
    })
    .await
}

#[tokio::test]
async fn perf_02_paging_by_the_parent_never_names_a_hidden_parent_and_walks_every_issue_once()
-> TestResult {
    each_database("search_page_by_parent", async |db| {
        let fixture = fixture(db).await?;
        let data = seed(&fixture.storage).await;
        let storage = &fixture.storage;
        let whole = titles(storage, &data, "sort by parent, title").await;
        assert_eq!(whole.len(), 6);
        let mut walked = Vec::new();
        let mut after = None;
        loop {
            let hits = storage
                .search_page(&request(&data, "sort by parent, title", after, 2))
                .await
                .map_err(|error| format!("{error:?}"))?;
            after = hits.last().map(|hit| hit.issue.id);
            let full = hits.len() == 2;
            walked.extend(hits.into_iter().map(|hit| hit.issue.title));
            if !full {
                break;
            }
        }
        assert_eq!(walked, whole);
        Ok(())
    })
    .await
}

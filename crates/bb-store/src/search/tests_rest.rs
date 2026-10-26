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

/// A comment of `text` on an issue, the way the use case stores it.
fn said(
    source: &mut impl bb_core::id::IdSource,
    issue: IssueId,
    text: &str,
) -> Result<bb_core::comment::Comment, Box<dyn std::error::Error + Send + Sync>> {
    Ok(bb_core::comment::Comment {
        id: bb_core::id::CommentId::generate(source)?,
        issue,
        author: bb_core::id::UserId::generate(source),
        text: text.to_owned(),
        created_at: at(2),
        edited_at: None,
        deleted_at: None,
        version: FIRST_VERSION,
    })
}

/// Writes the comments and indexes every issue, by hand like the outbox reader would.
async fn comment_and_index(
    storage: &AuthStorage,
    data: &mut Data,
    comments: &[(IssueId, &str)],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use bb_app::comments::CommentStore as _;
    let mut made = Vec::new();
    for (issue, text) in comments {
        made.push(said(&mut data.source, *issue, text)?);
    }
    let mut all: Vec<IssueId> = data.issues.iter().map(|each| each.id).collect();
    all.push(data.hidden.id);
    let ev = event(&mut data.source, 13);
    ChangeStore::record_change(storage, &ev, move |tx| {
        Box::pin(async move {
            for each in &made {
                AuthStorage::insert_comment(tx, each).await?;
            }
            crate::text_index::reindex_issues(tx, &all).await
        })
    })
    .await?;
    Ok(())
}

#[tokio::test]
async fn qry_05_contains_on_its_own_finds_all_words_across_the_texts_of_issues_the_person_reads()
-> TestResult {
    each_database("search_bare_contains", async |db| {
        let fixture = fixture(db).await?;
        let mut data = seed(&fixture.storage).await;
        let storage = &fixture.storage;
        let login = data.issues[1].id;
        let hidden = data.hidden.id;
        comment_and_index(
            storage,
            &mut data,
            &[
                (login, "Zebra stripes appear"),
                (hidden, "zebra and crash in the secret project"),
            ],
        )
        .await?;
        let found = |text: &'static str| {
            let ask = request(&data, text, None, 20);
            async move { titles_of(storage, &ask).await }
        };
        // A word only in a comment: found on the issue the person reads, not on the hidden one.
        assert_eq!(found("contains zebra").await, ["Login broken"]);
        // The words may stand in different texts of the issue: the title and a comment.
        assert_eq!(found("contains \"login zebra\"").await, ["Login broken"]);
        assert!(found("contains \"login printer\"").await.is_empty());
        // The title of an issue is found too, and the hidden comment adds nothing to its word.
        assert_eq!(found("contains \"crash\"").await, ["Crash on save"]);
        // It combines with any other filter, and `not` leaves the issue out.
        assert_eq!(
            found("contains zebra and priority is critical").await,
            ["Login broken"]
        );
        let without = found("not contains zebra").await;
        assert_eq!(without.len(), 5);
        assert!(!without.contains(&"Login broken".to_owned()));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn perf_02_the_issues_whose_title_has_the_words_come_first_then_the_newest() -> TestResult {
    each_database("search_title_hit_order", async |db| {
        let fixture = fixture(db).await?;
        let mut data = seed(&fixture.storage).await;
        let storage = &fixture.storage;
        // Slow page has the word in its title (created 4); Idea (6) and Docs typo (3) in a comment.
        let (docs, idea) = (data.issues[2].id, data.issues[5].id);
        comment_and_index(
            storage,
            &mut data,
            &[(idea, "the page jams"), (docs, "a page with a typo")],
        )
        .await?;
        let whole = titles_of(storage, &request(&data, "contains page", None, 20)).await;
        assert_eq!(whole, ["Slow page", "Idea", "Docs typo"]);
        // Paging one by one never skips or repeats one: the cursor holds only the ID.
        let mut walked = Vec::new();
        let mut after = None;
        loop {
            let hits = storage
                .search_page(&request(&data, "contains page", after, 1))
                .await
                .map_err(|error| format!("{error:?}"))?;
            after = hits.last().map(|hit| hit.issue.id);
            let more = !hits.is_empty();
            walked.extend(hits.into_iter().map(|hit| hit.issue.title));
            if !more {
                break;
            }
        }
        assert_eq!(walked, whole);
        // An order that was asked for stays the one asked for.
        let sorted = titles_of(
            storage,
            &request(&data, "contains page sort by title", None, 20),
        )
        .await;
        assert_eq!(sorted, ["Docs typo", "Idea", "Slow page"]);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_06_a_person_named_for_one_kind_of_field_is_not_found_by_another() -> TestResult {
    use bb_core::fields::{ASSIGNED_TO_PEOPLE, REPORTED_BY_PEOPLE};
    each_database("search_people_by_kind", async |db| {
        let fixture = fixture(db).await?;
        let data = seed(&fixture.storage).await;
        let storage = &fixture.storage;
        let reporter = bb_app::search::Searchable {
            field: Field::new("reportedBy", FieldKind::Person),
            target: FieldTarget::ReportedBy,
        };
        let ask = |kind| {
            let mut ask = request_with(
                &data,
                "reportedBy is \"Me Too\" sort by title",
                None,
                20,
                vec![reporter.clone()],
                Vec::new(),
            );
            ask.names.people = vec![(kind, name_key("Me Too"), vec![data.me])];
            ask
        };
        // Named as a reporter, the person is found as one; named as an assignee, they are not.
        assert_eq!(titles_of(storage, &ask(REPORTED_BY_PEOPLE)).await.len(), 5);
        assert!(
            titles_of(storage, &ask(ASSIGNED_TO_PEOPLE))
                .await
                .is_empty()
        );
        Ok(())
    })
    .await
}

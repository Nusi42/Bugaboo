//! The search compiler on both databases (QRY-04, QRY-05, SEC-17): one table of searches that must
//! find the same issues in the same order on SQLite and PostgreSQL, paging that neither skips nor
//! repeats an issue, and values that never become SQL.

use bb_app::access::PolicyStore as _;
use bb_app::issues::IssueStore as _;
use bb_app::ports::ChangeStore;
use bb_app::relations::RelationStore as _;
use bb_app::search::{
    FieldTarget, Names, ProjectLink, SearchPosition, SearchRequest, SearchStore as _, Searchable,
};
use bb_app::tags::TagStore as _;
use bb_core::id::{IssueTypeId, ProjectId, RelationId, RelationKindId, TagId, UserId};
use bb_core::issue::{Issue, IssueIcon, IssueType, Priority, StatusGroup};
use bb_core::own_fields::ChoiceColor;
use bb_core::project::{Project, ShortName};
use bb_core::relation::{IssueParent, Relation, RelationKind};
use bb_core::tags::{IssueSight, IssueTag, Tag};
use bb_core::version::FIRST_VERSION;
use bb_core::workflow::starter;
use bb_searchlang::{Field, FieldKind, IssueField, ReadableFields, YES, check, parse_with};
use bb_testing::{TestResult, each_database};
use jiff::tz::TimeZone;

use crate::AuthStorage;
use crate::test_support::{at, event, fixture, ids, issue};

/// What the tests search in: six issues of one project, with the people, tags and relations that
/// the helpers follow.
struct Data {
    project: ProjectId,
    me: UserId,
    blocks: RelationKindId,
}

/// Six issues of one project, by the person who asks except the last.
fn six_issues(
    source: &mut impl bb_core::id::IdSource,
    project: ProjectId,
    issue_type: IssueTypeId,
    me: UserId,
    stranger: UserId,
) -> Vec<Issue> {
    // (title, priority, created, closed, reported by)
    let rows = [
        ("Printer jams", Priority::High, 1, false, me),
        ("Login broken", Priority::Critical, 2, false, me),
        ("Docs typo", Priority::Low, 3, false, me),
        ("Slow page", Priority::High, 4, true, me),
        ("Crash on save", Priority::High, 5, false, me),
        ("Idea", Priority::Normal, 6, false, stranger),
    ];
    rows.iter()
        .enumerate()
        .map(|(number, (title, priority, created, closed, reporter))| {
            let mut made = issue(
                source,
                project,
                issue_type,
                i64::try_from(number).unwrap() + 1,
            );
            made.title = (*title).to_owned();
            made.priority = *priority;
            made.created_at = at(*created);
            made.updated_at = at(*created);
            made.reported_by = *reporter;
            if *closed {
                made.closed_at = Some(at(20));
            }
            made
        })
        .collect()
}

async fn seed(storage: &AuthStorage) -> Data {
    let mut source = ids();
    let project = ProjectId::generate(&mut source).unwrap();
    let issue_type = IssueTypeId::generate(&mut source).unwrap();
    let me = UserId::generate(&mut source);
    let stranger = UserId::generate(&mut source);
    let issues = six_issues(&mut source, project, issue_type, me, stranger);
    let tag = Tag {
        id: TagId::generate(&mut source).unwrap(),
        name: "UI".to_owned(),
        color: ChoiceColor::Cat1,
        created_at: at(0),
        version: FIRST_VERSION,
    };
    let second_tag = Tag {
        id: TagId::generate(&mut source).unwrap(),
        name: "Bug".to_owned(),
        ..tag.clone()
    };
    let blocks = RelationKind {
        id: RelationKindId::generate(&mut source).unwrap(),
        forward: "blocks".to_owned(),
        backward: "is blocked by".to_owned(),
        position: 0,
        retired_at: None,
        version: FIRST_VERSION,
    };
    let (row, kind, rows) = (
        Project {
            id: project,
            parent: None,
            short_name: ShortName::new("ACME").unwrap(),
            name: "Acme".to_owned(),
            version: FIRST_VERSION,
        },
        IssueType {
            project: None,
            id: issue_type,
            name: "Bug".to_owned(),
            icon: IssueIcon::Bug,
            position: 0,
            retired_at: None,
            version: FIRST_VERSION,
        },
        issues.clone(),
    );
    let (tagged, kind_row, block_from_to) = (
        // Printer jams has two tags, Login broken one, the rest none.
        [(0, tag.id), (1, tag.id), (0, second_tag.id)].map(|(at_issue, tag)| IssueTag {
            issue: issues[at_issue].id,
            tag,
            added_by: me,
            added_at: at(0),
        }),
        blocks.clone(),
        [(0, 1), (1, 5)].map(|(from, to)| Relation {
            id: RelationId::generate(&mut source).unwrap(),
            kind: blocks.id,
            from: issues[from].id,
            to: issues[to].id,
            added_by: me,
            added_at: at(0),
        }),
    );
    // Docs typo sits under Login broken, and Crash on save under Docs typo.
    let parents = [(2, 1), (4, 2)].map(|(child, parent)| IssueParent {
        child: issues[child].id,
        parent: issues[parent].id,
        set_by: me,
        set_at: at(0),
    });
    ChangeStore::record_change(storage, &event(&mut source, 5), move |tx| {
        Box::pin(async move {
            AuthStorage::insert_project(tx, &row).await?;
            AuthStorage::insert_issue_type(tx, &kind).await?;
            for each in &rows {
                AuthStorage::insert_issue(tx, each, &[]).await?;
            }
            AuthStorage::insert_tag(tx, &tag).await?;
            AuthStorage::insert_tag(tx, &second_tag).await?;
            for link in &tagged {
                AuthStorage::insert_issue_tag(tx, link).await?;
            }
            AuthStorage::insert_relation_kind(tx, &kind_row).await?;
            for relation in &block_from_to {
                AuthStorage::insert_relation(tx, relation).await?;
            }
            for parent in &parents {
                AuthStorage::set_issue_parent(tx, parent).await?;
            }
            Ok(())
        })
    })
    .await
    .unwrap();
    Data {
        project,
        me,
        blocks: blocks.id,
    }
}

/// The fields the tests may search by, with what each reads.
fn searchables() -> Vec<Searchable> {
    let status_names: Vec<String> = vec![starter::OPEN.to_owned()];
    let priorities: Vec<String> = Priority::ALL
        .iter()
        .map(|each| each.search_word())
        .collect();
    let with = |field: Field, choices: Vec<String>, target| Searchable {
        field: Field { choices, ..field },
        target,
    };
    vec![
        with(
            Field::new("title", FieldKind::Text),
            vec![],
            FieldTarget::Title,
        ),
        with(
            Field::unordered("status"),
            status_names,
            FieldTarget::Status,
        ),
        with(
            Field::new("priority", FieldKind::Choice),
            priorities,
            FieldTarget::Priority,
        ),
        with(Field::unordered("tags"), vec![], FieldTarget::Tags),
        with(
            Field::new("assignedTo", FieldKind::Person),
            vec![],
            FieldTarget::AssignedTo,
        ),
        with(
            IssueField::Project.field(vec![]),
            vec![],
            FieldTarget::Project,
        ),
        with(
            IssueField::Parent.field(vec![]),
            vec![],
            FieldTarget::Parent,
        ),
        with(
            IssueField::Created.field(vec![]),
            vec![],
            FieldTarget::Created,
        ),
        with(
            IssueField::Closed.field(vec![]),
            vec![],
            FieldTarget::Closed,
        ),
    ]
}

/// A request for a search text, for the person who reported most of the issues.
fn request(data: &Data, text: &str, after: Option<SearchPosition>, limit: u64) -> SearchRequest {
    let fields = searchables();
    let readable = ReadableFields {
        fields: fields.iter().map(|each| each.field.clone()).collect(),
    };
    let parsed = parse_with(text, &readable).unwrap();
    let checked = check(parsed, &readable).unwrap();
    SearchRequest {
        search: checked,
        fields,
        sight: IssueSight {
            all: vec![data.project],
            own: vec![],
            person: data.me,
            hidden: vec![],
            with_trashed: false,
        },
        me: data.me,
        now: at(100),
        zone: TimeZone::UTC,
        projects: vec![ProjectLink {
            id: data.project,
            parent: None,
            short_name: "ACME".to_owned(),
        }],
        names: Names {
            statuses: vec![("open".to_owned(), vec![starter::OPEN.to_owned()])],
            ..Names::default()
        },
        status_groups: StatusGroup::ALL
            .iter()
            .map(|group| (*group, Vec::new()))
            .collect(),
        blocking_kinds: vec![data.blocks],
        after,
        limit,
    }
}

async fn titles(storage: &AuthStorage, data: &Data, text: &str) -> Vec<String> {
    let hits = storage
        .search_page(&request(data, text, None, 100))
        .await
        .unwrap();
    hits.into_iter().map(|hit| hit.issue.title).collect()
}

/// The searches every database must answer alike: the text, and the titles in order.
const TABLE: [(&str, &[&str]); 12] = [
    (
        "",
        &[
            "Idea",
            "Crash on save",
            "Slow page",
            "Docs typo",
            "Login broken",
            "Printer jams",
        ],
    ),
    (
        "priority is high",
        &["Crash on save", "Slow page", "Printer jams"],
    ),
    (
        "priority is high sort by created oldest first",
        &["Printer jams", "Slow page", "Crash on save"],
    ),
    (
        "priority >= high sort by priority, highest first, created, oldest first",
        &["Login broken", "Printer jams", "Slow page", "Crash on save"],
    ),
    ("priority < normal", &["Docs typo"]),
    ("not priority is high and title is \"Idea\"", &["Idea"]),
    ("tags is ui", &["Login broken", "Printer jams"]),
    (
        "tags is empty",
        &["Idea", "Crash on save", "Slow page", "Docs typo"],
    ),
    (
        "closed is empty and created < 2026-01-02",
        &[
            "Idea",
            "Crash on save",
            "Docs typo",
            "Login broken",
            "Printer jams",
        ],
    ),
    (
        "created in last 1 day sort by closed",
        &[
            "Slow page",
            "Idea",
            "Crash on save",
            "Docs typo",
            "Login broken",
            "Printer jams",
        ],
    ),
    (
        "status is Open and project is ACME sort by title",
        &[
            "Crash on save",
            "Docs typo",
            "Idea",
            "Login broken",
            "Printer jams",
            "Slow page",
        ],
    ),
    (
        "priority is high or priority is critical sort by title, Z to A",
        &["Slow page", "Printer jams", "Login broken", "Crash on save"],
    ),
];

#[tokio::test]
async fn arch_db_parity_a_table_of_searches_gives_the_same_issues_in_the_same_order() -> TestResult
{
    each_database("search_parity", async |db| {
        let fixture = fixture(db).await?;
        let data = seed(&fixture.storage).await;
        for (text, expected) in TABLE {
            assert_eq!(
                titles(&fixture.storage, &data, text).await,
                expected,
                "{text}"
            );
        }
        Ok(())
    })
    .await
}

#[tokio::test]
async fn qry_04_the_helpers_follow_sub_issues_blocks_and_chains_of_relations() -> TestResult {
    each_database("search_helpers", async |db| {
        let fixture = fixture(db).await?;
        let data = seed(&fixture.storage).await;
        let storage = &fixture.storage;
        let id = |at: usize| format!("ACME-{}", at + 1);
        // Docs typo and Crash on save sit under Login broken, at any depth.
        let under = titles(storage, &data, &format!("under {} sort by title", id(1))).await;
        assert_eq!(under, ["Crash on save", "Docs typo"]);
        let parent = titles(storage, &data, &format!("parent is {}", id(1))).await;
        assert_eq!(parent, ["Docs typo"]);
        // Printer jams blocks Login broken, which blocks Idea.
        let blocks = titles(storage, &data, &format!("blocks {}", id(1))).await;
        assert_eq!(blocks, ["Printer jams"]);
        let blocked = titles(storage, &data, &format!("blocked by {}", id(1))).await;
        assert_eq!(blocked, ["Idea"]);
        let near = titles(
            storage,
            &data,
            &format!("connected to {} within 1 step sort by title", id(0)),
        )
        .await;
        assert_eq!(near, ["Login broken"]);
        let far = titles(
            storage,
            &data,
            &format!("connected to {} sort by title", id(0)),
        )
        .await;
        assert_eq!(far, ["Idea", "Login broken"]);
        // Two walks in one statement.
        let both = titles(
            storage,
            &data,
            &format!(
                "under {} or connected to {} within 1 step sort by title",
                id(1),
                id(0)
            ),
        )
        .await;
        assert_eq!(both, ["Crash on save", "Docs typo", "Login broken"]);
        // A reference to an issue nobody reads finds nothing, and so does an unknown one.
        assert!(titles(storage, &data, "under ACME-99").await.is_empty());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn perf_02_paging_walks_every_issue_once_in_every_order() -> TestResult {
    each_database("search_paging", async |db| {
        let fixture = fixture(db).await?;
        let data = seed(&fixture.storage).await;
        let storage = &fixture.storage;
        for text in [
            "",
            "sort by priority",
            "sort by closed",
            "sort by title, Z to A",
            "sort by priority, highest first, closed, oldest first",
        ] {
            let all = titles(storage, &data, text).await;
            let mut walked = Vec::new();
            let mut after = None;
            loop {
                let page = storage
                    .search_page(&request(&data, text, after.clone(), 2))
                    .await
                    .map_err(|error| format!("{error:?}"))?;
                if page.is_empty() {
                    break;
                }
                after = page.last().map(|hit| hit.position.clone());
                walked.extend(page.into_iter().map(|hit| hit.issue.title));
            }
            assert_eq!(walked, all, "{text}");
            assert_eq!(all.len(), 6);
        }
        Ok(())
    })
    .await
}

#[tokio::test]
async fn qry_05_the_count_and_the_groups_come_from_the_same_search() -> TestResult {
    each_database("search_counts", async |db| {
        let fixture = fixture(db).await?;
        let data = seed(&fixture.storage).await;
        let storage = &fixture.storage;
        let high = request(&data, "priority is high", None, 1);
        assert_eq!(storage.search_count(&high).await.unwrap(), 3);
        let groups = storage
            .search_groups(&request(&data, "", None, 1), "priority")
            .await
            .unwrap();
        let counts: Vec<(Option<&str>, u64)> = groups
            .iter()
            .map(|group| (group.value.as_deref(), group.issues))
            .collect();
        assert_eq!(counts.iter().map(|(_, n)| n).sum::<u64>(), 6);
        // An issue with two tags stands in the group of each; one with none in the group without a
        // value, which comes last. The count line is still the number of issues.
        let by_tag = storage
            .search_groups(&request(&data, "", None, 1), "tags")
            .await
            .unwrap();
        let by_tag: Vec<(Option<&str>, u64)> = by_tag
            .iter()
            .map(|group| (group.value.as_deref(), group.issues))
            .collect();
        assert_eq!(by_tag, [(Some("bug"), 1), (Some("ui"), 2), (None, 4)]);
        assert_eq!(
            storage
                .search_count(&request(&data, "", None, 1))
                .await
                .unwrap(),
            6
        );
        // The group of an unreadable field is as unknown as the field.
        assert!(storage.search_groups(&high, "nonsense").await.is_err());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn qry_05_the_sight_decides_which_issues_a_search_can_find() -> TestResult {
    each_database("search_sight", async |db| {
        let fixture = fixture(db).await?;
        let data = seed(&fixture.storage).await;
        let mut ask = request(&data, "", None, 100);
        // Where the person sees only what they reported, the issue of a stranger is not found,
        // also not by naming it.
        ask.sight.all.clear();
        ask.sight.own = vec![data.project];
        ask.sight.person = UserId::generate(&mut ids());
        assert!(fixture.storage.search_page(&ask).await.unwrap().is_empty());
        ask.sight.person = data.me;
        assert_eq!(fixture.storage.search_count(&ask).await.unwrap(), 5);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_17_a_search_text_never_becomes_sql() -> TestResult {
    each_database("search_injection", async |db| {
        let fixture = fixture(db).await?;
        let data = seed(&fixture.storage).await;
        let nasty = "x'); DROP TABLE issue; -- \" \\ ; /* */";
        let escaped = nasty.replace('\\', "\\\\").replace('"', "\\\"");
        let text = format!("title is \"{escaped}\" or tags is \"{escaped}\"");
        let ask = request(&data, &text, None, 10);
        let statement =
            super::statements::page_statement(&ask, fixture.storage.database().dialect())
                .map_err(|error| error.to_string())?;
        let super::statements::Built::Plain(select) = &statement else {
            return Err("a search without helpers has no walks".into());
        };
        let (sql, values) = select.build(sea_query::SqliteQueryBuilder);
        assert!(!sql.contains("DROP") && !sql.contains(nasty), "{sql}");
        assert!(values.0.iter().any(
            |value| matches!(value, sea_query::Value::String(Some(text)) if text.as_str() == nasty)
        ));
        assert!(fixture.storage.search_page(&ask).await.unwrap().is_empty());
        assert_eq!(titles(&fixture.storage, &data, "").await.len(), 6);
        assert_eq!(YES, "yes");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn qry_05_a_word_that_is_not_a_readable_field_cannot_be_compiled() -> TestResult {
    each_database("search_unknown", async |db| {
        let fixture = fixture(db).await?;
        let data = seed(&fixture.storage).await;
        let mut ask = request(&data, "title is x", None, 10);
        // The caller forgot to check: the word is not among the fields the person may read.
        ask.fields.retain(|each| each.field.word != "title");
        assert!(fixture.storage.search_page(&ask).await.is_err());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_17_a_search_over_its_time_answers_too_slow() -> TestResult {
    use sea_query::{Alias, Expr, ExprTrait as _, Func, Query, UnionType, WithClause};
    each_database("search_too_slow", async |db| {
        let fixture = fixture(db).await?;
        let database = fixture.storage.database();
        // A count that has to go through two billion numbers: far over the time it is given.
        let numbers = Alias::new("numbers");
        let mut first = Query::select();
        first.expr(Expr::val(1_i64));
        let mut next = Query::select();
        next.expr(Expr::col(Alias::new("n")).add(1_i64))
            .from(numbers.clone())
            .and_where(Expr::col(Alias::new("n")).lt(2_000_000_000_i64));
        first.union(UnionType::All, next);
        let mut table = sea_query::CommonTableExpression::new();
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
        let slow = count.with(clause);
        let started = std::time::Instant::now();
        let answer = database
            .fetch_all_within(&slow, std::time::Duration::from_millis(200))
            .await;
        assert!(
            matches!(answer, Err(crate::StoreError::TimedOut)),
            "{:?}",
            answer.err()
        );
        assert!(started.elapsed() < std::time::Duration::from_secs(10));
        // The connection is fine afterwards, with no limit left on it.
        let data = seed(&fixture.storage).await;
        assert_eq!(titles(&fixture.storage, &data, "").await.len(), 6);
        Ok(())
    })
    .await
}

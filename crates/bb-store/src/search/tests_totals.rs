//! The totals of a search on both databases (QRY-08, PRIV-13, ARCH-DB-PARITY): an issue with several
//! values stands in each of their groups and once in the total of all, a value counts only in the
//! places it is readable in, and people of one set are one group. The shapes the older tests read
//! (`GroupCount`, `LengthTotals`) are made from the same totals here.

use bb_app::issues::IssueStore as _;
use bb_app::ports::ChangeStore;
use bb_app::search::{
    FieldPlace, FieldTarget, GroupBy, GroupKey, GroupPage, ReadableWhere, SearchRequest,
    SearchStore as _, Searchable, Sum, SumAsked, TotalsAsked,
};
use bb_core::id::{IssueTypeId, UserId};
use bb_core::issue::AssignedPerson;
use bb_testing::{TestResult, each_database};
use sea_query::{Expr, ExprTrait as _, Query};

use super::tests::{request_with, seed};
use super::tests_time_spent::lengths;
use crate::AuthStorage;
use crate::db::StoredId as _;
use crate::schema::IssueRow;
use crate::test_support::{at, event, fixture, issue};

/// How many issues a group holds, as the older tests read the totals.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct GroupCount {
    pub(super) key: GroupKey,
    pub(super) issues: u64,
}

/// The totals of the three lengths of a search, as the older tests read them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct LengthTotals {
    pub(super) found_any: bool,
    pub(super) estimated: Option<bb_core::time::Minutes>,
    pub(super) time_left: Option<bb_core::time::Minutes>,
    pub(super) time_spent: Option<bb_core::time::Minutes>,
}

impl AuthStorage {
    /// The count of each group of the field `by`, from the totals without sums.
    pub(super) async fn search_groups(
        &self,
        request: &SearchRequest,
        by: &str,
    ) -> Result<Vec<GroupCount>, bb_app::search::SearchFailure> {
        let asked = TotalsAsked {
            by: Some(GroupBy {
                word: by.to_owned(),
                people: Vec::new(),
                then: None,
                page: None,
            }),
            sums: Vec::new(),
            all: false,
        };
        Ok(self
            .search_totals(request, &asked)
            .await?
            .groups
            .into_iter()
            .map(|group| GroupCount {
                key: group.key,
                issues: group.total.issues,
            })
            .collect())
    }

    /// The three lengths of the issues a search finds, each summed where it is a readable field.
    pub(super) async fn search_lengths(
        &self,
        request: &SearchRequest,
    ) -> Result<LengthTotals, bb_app::search::SearchFailure> {
        let asked = TotalsAsked {
            by: None,
            sums: [
                FieldTarget::EstimatedTime,
                FieldTarget::TimeLeft,
                FieldTarget::TimeSpent,
            ]
            .map(|target| SumAsked {
                target,
                readable: ReadableWhere::Everywhere,
            })
            .to_vec(),
            all: true,
        };
        let all = self.search_totals(request, &asked).await?.all;
        let minutes = |at: usize| {
            all.sums
                .get(at)
                .and_then(|sum| sum.sum)
                .and_then(|sum| bb_core::time::Minutes::try_from(sum).ok())
        };
        Ok(LengthTotals {
            found_any: all.issues > 0,
            estimated: minutes(0),
            time_left: minutes(1),
            time_spent: minutes(2),
        })
    }
}

/// The sum of *Estimated time* where `readable` says.
fn estimated(readable: ReadableWhere) -> SumAsked {
    SumAsked {
        target: FieldTarget::EstimatedTime,
        readable,
    }
}

/// The issues of the tests with Printer jams (tags UI and Bug) estimated at 60 minutes and Login
/// broken (UI) at 30, and a request of all of them that may read the lengths and the reporter.
async fn estimated_issues(
    storage: &AuthStorage,
) -> Result<(super::tests::Data, SearchRequest), bb_testing::Failure> {
    let data = seed(storage).await;
    for (at, minutes) in [(0, 60_i64), (1, 30)] {
        let mut update = Query::update();
        update
            .table(IssueRow::Table)
            .value(IssueRow::EstimatedTime, minutes)
            .and_where(Expr::col(IssueRow::Id).eq(data.issues[at].id.stored()));
        storage.database().execute(&update).await?;
    }
    let mut more = lengths();
    more.push(Searchable {
        field: bb_searchlang::Field::new("reportedBy", bb_searchlang::FieldKind::Person),
        target: FieldTarget::ReportedBy,
    });
    let request: SearchRequest = request_with(&data, "", None, 1, more, Vec::new());
    Ok((data, request))
}

#[tokio::test]
async fn qry_08_a_sum_counts_only_in_the_places_it_is_readable_in() -> TestResult {
    each_database("qry_08_compiled_places", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let (mut data, request) = estimated_issues(storage).await?;
        let sum = |sum, valued| Sum { sum, valued };
        // Only on the issue types the reader may read the field on, in the project.
        let other_type = IssueTypeId::generate(&mut data.source)?;
        for (types, expected) in [
            (Some(vec![other_type]), sum(None, 0)),
            (Some(vec![data.issue_type]), sum(Some(90), 2)),
            (None, sum(Some(90), 2)),
        ] {
            let asked = TotalsAsked {
                by: None,
                sums: vec![estimated(ReadableWhere::In(vec![FieldPlace {
                    project: data.project,
                    types,
                }]))],
                all: true,
            };
            let all = storage
                .search_totals(&request, &asked)
                .await
                .map_err(|error| format!("{error:?}"))?
                .all;
            assert_eq!(all.issues, 6);
            assert_eq!(all.sums, vec![expected]);
        }
        let nowhere = TotalsAsked {
            by: None,
            sums: vec![estimated(ReadableWhere::In(Vec::new()))],
            all: true,
        };
        let all = storage
            .search_totals(&request, &nowhere)
            .await
            .map_err(|error| format!("{error:?}"))?
            .all;
        assert_eq!(all.sums, vec![sum(None, 0)]);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn qry_08_the_compiler_adds_up_groups_and_sets_of_people() -> TestResult {
    each_database("qry_08_compiled_totals", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let (data, request) = estimated_issues(storage).await?;
        let by = |word: &str, people| {
            Some(GroupBy {
                word: word.to_owned(),
                people,
                then: None,
                page: None,
            })
        };
        let everywhere = TotalsAsked {
            by: by("tags", Vec::new()),
            sums: vec![estimated(ReadableWhere::Everywhere)],
            all: true,
        };
        let totals = storage
            .search_totals(&request, &everywhere)
            .await
            .map_err(|error| format!("{error:?}"))?;
        let sum = |sum, valued| Sum { sum, valued };
        let tag = |name: &str| {
            data.tags
                .iter()
                .find(|(each, _)| each == name)
                .map(|(_, id)| GroupKey::Tag(*id))
        };
        let of = |key: Option<GroupKey>| {
            totals
                .groups
                .iter()
                .find(|group| Some(&group.key) == key.as_ref())
                .map(|group| (group.total.issues, group.total.sums.clone()))
        };
        assert_eq!(of(tag("ui")), Some((2, vec![sum(Some(90), 2)])));
        assert_eq!(of(tag("bug")), Some((1, vec![sum(Some(60), 1)])));
        assert_eq!(of(Some(GroupKey::Empty)), Some((4, vec![sum(None, 0)])));
        // All of them: the issue with two tags counts once.
        assert_eq!(totals.all.issues, 6);
        assert_eq!(totals.all.sums, vec![sum(Some(90), 2)]);
        // The two people who reported the issues as one set: one group of all six.
        let reporters: Vec<_> = data
            .issues
            .iter()
            .map(|issue| issue.reported_by)
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        assert_eq!(reporters.len(), 2);
        let merged = TotalsAsked {
            by: by("reportedBy", vec![reporters.clone()]),
            sums: vec![estimated(ReadableWhere::Everywhere)],
            all: true,
        };
        let totals = storage
            .search_totals(&request, &merged)
            .await
            .map_err(|error| format!("{error:?}"))?;
        assert_eq!(totals.groups.len(), 1, "{:?}", totals.groups);
        assert_eq!(totals.groups[0].key, GroupKey::People(0));
        assert_eq!(totals.groups[0].total.issues, 6);
        assert_eq!(totals.all.issues, 6);
        // A reporter in no set is in no group, and their issues still count in all of them.
        let one = TotalsAsked {
            by: by("reportedBy", vec![vec![reporters[0]]]),
            sums: Vec::new(),
            all: true,
        };
        let totals = storage
            .search_totals(&request, &one)
            .await
            .map_err(|error| format!("{error:?}"))?;
        assert_eq!(totals.groups.len(), 1, "{:?}", totals.groups);
        assert!(totals.groups[0].total.issues < 6);
        assert_eq!(totals.all.issues, 6);
        // Without the total of all, nothing is counted there.
        let without = TotalsAsked { all: false, ..one };
        let totals = storage
            .search_totals(&request, &without)
            .await
            .map_err(|error| format!("{error:?}"))?;
        assert_eq!(totals.all.issues, 0);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn qry_08_two_people_of_one_set_on_one_issue_count_once_in_its_group() -> TestResult {
    each_database("qry_08_compiled_people_once", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let (mut data, request) = estimated_issues(storage).await?;
        let (one, other) = (
            UserId::generate(&mut data.source),
            UserId::generate(&mut data.source),
        );
        // A new issue assigned to both of them, and another to the first only.
        let mut issues = Vec::new();
        for (number, people) in [(7, vec![one, other]), (8, vec![one])] {
            let made = issue(&mut data.source, data.project, data.issue_type, number);
            let assigned: Vec<AssignedPerson> = people
                .iter()
                .enumerate()
                .map(|(position, person)| AssignedPerson {
                    issue: made.id,
                    person: *person,
                    added_at: at(0),
                    position: i64::try_from(position).unwrap_or(0),
                })
                .collect();
            issues.push((made, assigned));
        }
        ChangeStore::record_change(storage, &event(&mut data.source, 9), move |tx| {
            Box::pin(async move {
                for (made, assigned) in &issues {
                    AuthStorage::insert_issue(tx, made, assigned).await?;
                }
                Ok(())
            })
        })
        .await?;
        let asked = TotalsAsked {
            by: Some(GroupBy {
                word: "assignedTo".to_owned(),
                people: vec![vec![one, other]],
                then: None,
                page: None,
            }),
            sums: Vec::new(),
            all: true,
        };
        let totals = storage
            .search_totals(&request, &asked)
            .await
            .map_err(|error| format!("{error:?}"))?;
        let set = totals
            .groups
            .iter()
            .find(|group| group.key == GroupKey::People(0))
            .ok_or("no group of the set")?;
        // The issue of both counts once in the group of their set.
        assert_eq!(set.total.issues, 2, "{:?}", totals.groups);
        assert_eq!(totals.all.issues, 8);
        Ok(())
    })
    .await
}

/// The groups of the field `by`, read in pages of `limit` after the place of the last group.
async fn walked(
    storage: &AuthStorage,
    request: &SearchRequest,
    by: &str,
    limit: u64,
) -> Result<Vec<GroupKey>, String> {
    let mut seen = Vec::new();
    let mut after = None;
    loop {
        let asked = TotalsAsked {
            by: Some(GroupBy {
                word: by.to_owned(),
                people: Vec::new(),
                then: None,
                page: Some(GroupPage {
                    after: after.clone(),
                    limit,
                }),
            }),
            sums: Vec::new(),
            all: true,
        };
        let totals = storage
            .search_totals(request, &asked)
            .await
            .map_err(|error| format!("{error:?}"))?;
        // Every page has the total of all the issues.
        assert_eq!(totals.all.issues, 9, "{by}");
        assert!(totals.groups.len() as u64 <= limit, "{by}");
        let Some(last) = totals.groups.last() else {
            return Ok(seen);
        };
        after = Some(last.key.place());
        seen.extend(totals.groups.into_iter().map(|group| group.key));
        if seen.len() > 100 {
            return Err(format!("{by}: the pages do not end"));
        }
    }
}

#[tokio::test]
async fn qry_08_the_store_reads_a_page_of_groups_after_the_place_of_the_last() -> TestResult {
    each_database("qry_08_compiled_pages", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let (mut data, request) = estimated_issues(storage).await?;
        // Titles in small and capital letters and with an accent, which the bytes order alike on
        // both databases.
        let mut more = Vec::new();
        for (number, title) in [(7, "a"), (8, "B"), (9, "\u{c4}")] {
            let mut made = issue(&mut data.source, data.project, data.issue_type, number);
            title.clone_into(&mut made.title);
            more.push(made);
        }
        ChangeStore::record_change(storage, &event(&mut data.source, 9), move |tx| {
            Box::pin(async move {
                for made in &more {
                    AuthStorage::insert_issue(tx, made, &[]).await?;
                }
                Ok(())
            })
        })
        .await?;
        // Texts, IDs with the group without a value last, moments and lengths of time.
        for by in ["title", "tags", "created", "estimated time"] {
            let every = storage
                .search_groups(&request, by)
                .await
                .map_err(|error| format!("{error:?}"))?;
            let mut expected: Vec<GroupKey> = every.into_iter().map(|group| group.key).collect();
            expected.sort_by_cached_key(GroupKey::place);
            for limit in [1, 2, 100] {
                let seen = walked(storage, &request, by, limit).await?;
                assert_eq!(seen, expected, "{by} in pages of {limit}");
            }
            let mut once = expected.clone();
            once.dedup();
            assert_eq!(once, expected, "{by}");
        }
        Ok(())
    })
    .await
}

//! The cells of a board on both databases (PLAN-06, PERF-N1, ARCH-DB-PARITY): the totals of a
//! search grouped by one field and then by a second, and the first issues of every cell in one
//! statement, numbered in the order of the search.

use bb_app::search::{CellAt, CellWindow, Cells, GroupBy, GroupKey, SearchStore as _, TotalsAsked};
use bb_core::issue::StatusGroup;
use bb_testing::{TestResult, each_database};

use super::tests::{request, seed};
use crate::test_support::fixture;

/// The rows by priority, each split again by the status group.
fn by_priority_then_group() -> GroupBy {
    GroupBy::of("priority").split_by("statusGroup")
}

/// The cells of the status groups as columns, in their order, and rows by priority.
fn cells() -> Cells {
    Cells {
        columns: "statusGroup".to_owned(),
        column_of: StatusGroup::ALL
            .iter()
            .enumerate()
            .map(|(at, group)| (GroupKey::StatusGroup(*group), at))
            .collect(),
        rows: Some(by_priority_then_group()),
    }
}

#[tokio::test]
async fn plan_06_the_cells_of_a_board_are_counted_and_read_in_one_statement_each() -> TestResult {
    each_database("plan_06_store_cells", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let data = seed(storage).await;
        let all = request(&data, "", None, 1);
        let totals = storage
            .search_totals(
                &all,
                &TotalsAsked {
                    by: Some(by_priority_then_group()),
                    sums: Vec::new(),
                    all: true,
                },
            )
            .await
            .map_err(|failure| format!("{failure:?}"))?;
        // Every group has its second key, and the issues add up to the six the search finds.
        assert!(totals.groups.iter().all(|group| group.then.is_some()));
        let added: u64 = totals.groups.iter().map(|group| group.total.issues).sum();
        assert_eq!(added, 6);
        assert_eq!(totals.all.issues, 6);
        // A page of groups split by a second field would end inside a group: it is refused.
        let paged = TotalsAsked {
            by: Some(
                by_priority_then_group().paged(Some(bb_app::search::GroupPage {
                    after: None,
                    limit: 1,
                })),
            ),
            sums: Vec::new(),
            all: true,
        };
        assert!(storage.search_totals(&all, &paged).await.is_err());
        // One card of each cell, then the rest of the cells after it.
        let rows: Vec<GroupKey> = totals
            .groups
            .iter()
            .map(|group| group.key.clone())
            .collect();
        let window = |skip| CellWindow {
            skip,
            take: 1,
            only: None,
            rows: rows.clone(),
        };
        let first = storage
            .search_cell_pages(&all, &cells(), &window(0))
            .await
            .map_err(|failure| format!("{failure:?}"))?;
        assert_eq!(first.len(), totals.groups.len());
        let rest = storage
            .search_cell_pages(&all, &cells(), &window(1))
            .await
            .map_err(|failure| format!("{failure:?}"))?;
        assert_eq!(first.len() + rest.len(), 6, "every issue once at most");
        let mut seen: Vec<_> = first.iter().chain(&rest).map(|hit| hit.issue.id).collect();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), first.len() + rest.len());
        // *Show more* of one cell reads that cell only.
        let one = first.first().ok_or("no cell")?.at.clone();
        let more = storage
            .search_cell_pages(
                &all,
                &cells(),
                &CellWindow {
                    only: Some(CellAt {
                        column: one.column,
                        row: one.row.clone(),
                    }),
                    take: 50,
                    ..window(0)
                },
            )
            .await
            .map_err(|failure| format!("{failure:?}"))?;
        assert!(!more.is_empty() && more.iter().all(|hit| hit.at == one));
        // A row the board does not show is not read.
        let none = storage
            .search_cell_pages(
                &all,
                &cells(),
                &CellWindow {
                    rows: Vec::new(),
                    ..window(0)
                },
            )
            .await
            .map_err(|failure| format!("{failure:?}"))?;
        assert!(none.is_empty());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn plan_06_every_group_key_is_written_and_read_back_by_the_one_pair() -> TestResult {
    use bb_core::id::{
        ChoiceId, GroupId, IssueId, IssueTypeId, LabelId, ProjectId, SequentialIds, TagId, UserId,
    };
    use bb_core::issue::Priority;
    use bb_core::time::Minutes;
    use bb_core::time::range::LengthRange;
    use sea_query::{Expr, ExprTrait as _, Query};

    use super::order::{GroupValue, IdOf};
    use super::statements::{GROUP_COLUMN, key_value, read_key_at};

    each_database("plan_06_group_keys", async |db| {
        let fixture = fixture(db).await?;
        let mut ids = SequentialIds::default();
        let length = LengthRange::starting_at(Minutes::try_from(60_i64)?).ok_or("no range")?;
        let keys = [
            (GroupKey::Empty, GroupValue::Whole),
            (GroupKey::Text("in-progress".to_owned()), GroupValue::Text),
            (GroupKey::Whole(-7), GroupValue::Whole),
            (GroupKey::People(2), GroupValue::People),
            (GroupKey::Length(length), GroupValue::Length),
            (GroupKey::Priority(Priority::High), GroupValue::Priority),
            (
                GroupKey::StatusGroup(StatusGroup::Done),
                GroupValue::StatusGroup,
            ),
            (
                GroupKey::Person(UserId::generate(&mut ids)),
                GroupValue::Id(IdOf::Person),
            ),
            (
                GroupKey::Group(GroupId::generate(&mut ids)?),
                GroupValue::Id(IdOf::Group),
            ),
            (
                GroupKey::Issue(IssueId::generate(&mut ids)?),
                GroupValue::Id(IdOf::Issue),
            ),
            (
                GroupKey::Project(ProjectId::generate(&mut ids)?),
                GroupValue::Id(IdOf::Project),
            ),
            (
                GroupKey::Tag(TagId::generate(&mut ids)?),
                GroupValue::Id(IdOf::Tag),
            ),
            (
                GroupKey::Label(LabelId::generate(&mut ids)?),
                GroupValue::Id(IdOf::Label),
            ),
            (
                GroupKey::IssueType(IssueTypeId::generate(&mut ids)?),
                GroupValue::Id(IdOf::IssueType),
            ),
            (
                GroupKey::Choice(ChoiceId::generate(&mut ids)?),
                GroupValue::Id(IdOf::Choice),
            ),
        ];
        for (key, what) in keys {
            let mut value = key_value(&key).map_or_else(|| Expr::cust("NULL"), Expr::val);
            // A bare value has no type of its own: PostgreSQL is told it, as a column gives it, so a
            // statement of one text is always one type.
            if fixture.storage.database().dialect() == crate::db::Dialect::Postgres {
                let kind = match what {
                    GroupValue::Id(_) => "bytea",
                    GroupValue::Text => "text",
                    _ => "bigint",
                };
                value = value.cast_as(sea_query::Alias::new(kind));
            }
            let mut select = Query::select();
            select.expr_as(value, GROUP_COLUMN);
            let rows = fixture.storage.database().fetch_all(&select).await?;
            let row = rows.first().ok_or("no row")?;
            assert_eq!(read_key_at(row, what, GROUP_COLUMN)?, Some(key));
        }
        Ok(())
    })
    .await
}

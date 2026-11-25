//! One step of the relation map on both databases (ISS-16): the issues one line away, through the
//! kinds chosen and *part of*, never through an issue the person does not read, not known yet, by
//! ID and stopped inside the statement.

use bb_app::search::{MapStep, SearchStore as _};
use bb_core::id::IssueId;
use bb_testing::{TestResult, each_database};

use super::tests::{Data, request, seed};
use crate::test_support::fixture;

fn step(data: &Data, near: &[usize], known: &[usize], part_of: bool, most: u64) -> MapStep {
    let id = |at: &usize| data.issues[*at].id;
    MapStep {
        near: near.iter().map(id).collect(),
        known: known.iter().map(id).collect(),
        kinds: vec![data.blocks],
        part_of,
        most,
        began: std::time::Instant::now(),
    }
}

fn sorted(data: &Data, at: &[usize]) -> Vec<IssueId> {
    let mut ids: Vec<IssueId> = at.iter().map(|each| data.issues[*each].id).collect();
    ids.sort();
    ids
}

#[tokio::test]
async fn iss_16_a_step_of_the_map_never_passes_through_a_hidden_issue() -> TestResult {
    each_database("map_step_sight", async |db| {
        let fixture = fixture(db).await?;
        let data = seed(&fixture.storage).await;
        let storage = &fixture.storage;
        let ask = request(&data, "", None, 0);
        let walk = async |map: MapStep| {
            storage
                .map_step(&ask, &map)
                .await
                .map_err(|failure| format!("{failure:?}"))
        };
        // From the first issue: it blocks the second and the hidden one, and the hidden one sits
        // under it. Only the second is a step away.
        assert_eq!(
            walk(step(&data, &[0], &[0], true, 10)).await?,
            sorted(&data, &[1])
        );
        // From the second: the first is known, the sixth is blocked by it, the third sits under it.
        assert_eq!(
            walk(step(&data, &[1], &[0, 1], true, 10)).await?,
            sorted(&data, &[2, 5])
        );
        // Without *part of* the sub-issue is no step.
        assert_eq!(
            walk(step(&data, &[1], &[0, 1], false, 10)).await?,
            sorted(&data, &[5])
        );
        // The fourth is joined only through the hidden one, so no walk reaches it.
        assert!(
            walk(step(&data, &[2, 5], &[0, 1, 2, 5], true, 10))
                .await?
                .contains(&data.issues[4].id)
        );
        assert!(
            !walk(step(&data, &[2, 5], &[0, 1, 2, 5], true, 10))
                .await?
                .contains(&data.issues[3].id)
        );
        // The statement stops at the number asked for, by ID.
        let first = walk(step(&data, &[1], &[0, 1], true, 1)).await?;
        assert_eq!(first, sorted(&data, &[2, 5])[..1]);
        // The whole walk has the time of one search: a step past it is too slow.
        let mut late = step(&data, &[1], &[0, 1], true, 10);
        late.began = std::time::Instant::now()
            .checked_sub(super::SEARCH_TIME)
            .ok_or("the clock started too short a time ago")?;
        let failure = storage.map_step(&ask, &late).await.err();
        assert_eq!(failure, Some(bb_app::search::SearchFailure::TooSlow));
        // No kind and no *part of* is no step.
        let mut nothing = step(&data, &[0], &[0], false, 10);
        nothing.kinds.clear();
        assert!(walk(nothing).await?.is_empty());
        Ok(())
    })
    .await
}

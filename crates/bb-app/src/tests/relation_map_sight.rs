//! The relation map walks only through what the reader sees (ISS-16, PRIV-13, SEC-MATRIX): random
//! graphs of relations and sub-issues over three projects, a reader who sees a random set of them,
//! and the answer compared with the map of the same graph with the hidden issues taken out. No
//! gap, no count and no line may tell of a hidden issue; `more` is worked out on the walk the reader
//! sees.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use bb_core::ErrorCode;
use bb_core::id::{IssueId, ProjectId, RelationId, RelationKindId, SequentialIds};
use bb_core::relation::{IssueParent, Relation};
use bb_policy::StartingRole;
use bb_searchlang::MAX_STEPS;
use jiff::Timestamp;
use proptest::prelude::*;

use super::scenarios::{a_project, text};
use super::scenarios_issues::an_issue;
use super::workflows_rules::{assign, member, starting_role};
use super::world::World;
use crate::auth::AppError;
use crate::relations::{
    IssueMapQuery, KindsShown, MAP_ISSUES_MOST, RelationKindsQuery, RelationMap, issue_map,
    relation_kinds,
};

/// The projects of a graph.
const PROJECTS: [&str; 3] = ["AAA", "BBB", "CCC"];

/// A line of a random graph: two issues by their place, and what joins them: the first kind of
/// relation (*blocks*), the last (*related to*), or the first under the second (*part of*).
#[derive(Clone, Copy, Debug)]
struct Seed {
    one: usize,
    other: usize,
    what: usize,
}

/// A random graph and what the reader asks.
#[derive(Clone, Debug)]
struct Case {
    /// The project of each issue, by its place in [`PROJECTS`].
    issues: Vec<usize>,
    lines: Vec<Seed>,
    /// Which projects the reader sees.
    sees: [bool; 3],
    start: usize,
    steps: Option<u32>,
    blocks: bool,
    related: bool,
    part_of: bool,
}

fn case() -> impl Strategy<Value = Case> {
    (2_usize..12)
        .prop_flat_map(|count| {
            (
                proptest::collection::vec(0_usize..PROJECTS.len(), count),
                proptest::collection::vec(
                    (0..count, 0..count, 0_usize..3).prop_map(|(one, other, what)| Seed {
                        one,
                        other,
                        what,
                    }),
                    0..(count * 2),
                ),
                any::<[bool; 3]>(),
                0..count,
                proptest::option::of(1..=MAX_STEPS),
                (any::<bool>(), any::<bool>(), any::<bool>()),
            )
        })
        .prop_map(
            |(issues, lines, sees, start, steps, (blocks, related, part_of))| Case {
                issues,
                lines,
                sees,
                start,
                steps,
                blocks,
                related,
                part_of,
            },
        )
}

/// The graph as it was stored: the issues, the relations and which issue sits under which.
struct Stored {
    issues: Vec<(IssueId, usize)>,
    relations: Vec<Relation>,
    parents: Vec<IssueParent>,
    kinds: (RelationKindId, RelationKindId),
}

/// The kinds of relation of the first start: the first (*blocks*) and the last (*related to*).
async fn first_and_last_kind(world: &World) -> Result<(RelationKindId, RelationKindId), String> {
    let page = relation_kinds(
        &world.ctx(),
        RelationKindsQuery {
            session_token: world.admin.session_token.clone(),
            cursor: None,
            limit: 50,
        },
    )
    .await
    .map_err(text)?
    .into_value();
    match (page.items.first(), page.items.last()) {
        (Some(first), Some(last)) => Ok((first.id, last.id)),
        _ => Err("no kind of relation on a started server".to_owned()),
    }
}

/// Makes the issues of the case and stores its lines as they are, loops among them: a sub-issue
/// sits under one parent at most and never under itself, the rest is kept as drawn.
async fn store_graph(world: &World, projects: &[ProjectId], case: &Case) -> Result<Stored, String> {
    let mut issues = Vec::new();
    for project in &case.issues {
        issues.push((an_issue(world, projects[*project]).await?, *project));
    }
    let kinds = first_and_last_kind(world).await?;
    let mut ids = SequentialIds::default();
    let mut relations: Vec<Relation> = Vec::new();
    let mut parents: Vec<IssueParent> = Vec::new();
    let by = world.admin.person.id;
    for seed in &case.lines {
        let (one, other) = (issues[seed.one].0, issues[seed.other].0);
        if one == other {
            continue;
        }
        if seed.what == 2 {
            // The parent is the earlier issue, so sub-issues never make a circle.
            let (child, parent) = if seed.one > seed.other {
                (one, other)
            } else {
                (other, one)
            };
            if parents.iter().all(|row| row.child != child) {
                parents.push(IssueParent {
                    child,
                    parent,
                    set_by: by,
                    set_at: Timestamp::UNIX_EPOCH,
                });
            }
            continue;
        }
        let kind = if seed.what == 0 { kinds.0 } else { kinds.1 };
        if relations
            .iter()
            .any(|relation| relation.kind == kind && relation.from == one && relation.to == other)
        {
            continue;
        }
        relations.push(Relation {
            id: RelationId::generate(&mut ids).map_err(|error| error.to_string())?,
            kind,
            from: one,
            to: other,
            added_by: by,
            added_at: Timestamp::UNIX_EPOCH,
        });
    }
    {
        let mut shared = world.store.lock();
        shared.state.relations.extend(relations.iter().cloned());
        shared.state.parents.extend(parents.iter().cloned());
    }
    Ok(Stored {
        issues,
        relations,
        parents,
        kinds,
    })
}

/// The map of the graph with the hidden issues taken out, worked out without the use case: every
/// visible issue within the steps, by its fewest steps (a breadth-first walk), cut to the first
/// [`MAP_ISSUES_MOST`] by steps and ID, with the lines among them.
fn model(
    stored: &Stored,
    case: &Case,
) -> (BTreeMap<IssueId, u32>, BTreeSet<(IssueId, IssueId)>, bool) {
    let sees = |issue: IssueId| {
        stored
            .issues
            .iter()
            .any(|(id, project)| *id == issue && case.sees[*project])
    };
    let kinds: Vec<RelationKindId> = [
        (case.blocks, stored.kinds.0),
        (case.related, stored.kinds.1),
    ]
    .into_iter()
    .filter_map(|(chosen, kind)| chosen.then_some(kind))
    .collect();
    let mut lines: Vec<(IssueId, IssueId)> = stored
        .relations
        .iter()
        .filter(|relation| kinds.contains(&relation.kind))
        .map(|relation| (relation.from, relation.to))
        .collect();
    if case.part_of {
        lines.extend(stored.parents.iter().map(|row| (row.child, row.parent)));
    }
    let start = stored.issues[case.start].0;
    let steps = case.steps.unwrap_or(MAX_STEPS);
    let mut away: BTreeMap<IssueId, u32> = BTreeMap::from([(start, 0)]);
    let mut queue = VecDeque::from([start]);
    while let Some(at) = queue.pop_front() {
        let here = away[&at];
        if here == steps {
            continue;
        }
        for (one, other) in &lines {
            let next = if *one == at {
                *other
            } else if *other == at {
                *one
            } else {
                continue;
            };
            if sees(next) && !away.contains_key(&next) {
                away.insert(next, here + 1);
                queue.push_back(next);
            }
        }
    }
    let mut ordered: Vec<(u32, IssueId)> = away.iter().map(|(id, steps)| (*steps, *id)).collect();
    ordered.sort();
    let more = ordered.len() > MAP_ISSUES_MOST;
    ordered.truncate(MAP_ISSUES_MOST);
    let kept: BTreeMap<IssueId, u32> = ordered.into_iter().map(|(steps, id)| (id, steps)).collect();
    let drawn = lines
        .into_iter()
        .filter(|(one, other)| kept.contains_key(one) && kept.contains_key(other))
        .collect();
    (kept, drawn, more)
}

/// The world of a case, its graph, and the map the reader is answered.
async fn map_of_case(case: &Case) -> Result<(Stored, Result<RelationMap, AppError>), String> {
    let world = World::new().await?;
    let mut projects = Vec::new();
    for name in PROJECTS {
        projects.push(a_project(&world, name).await?);
    }
    let stored = store_graph(&world, &projects, case).await?;
    let seen: Vec<ProjectId> = projects
        .iter()
        .zip(case.sees)
        .filter_map(|(project, sees)| sees.then_some(*project))
        .collect();
    // The reader is a team member where they see, and a stranger everywhere else.
    let reader = world.person("reader@example.test").await.map_err(text)?;
    let role = starting_role(&world, StartingRole::TeamMember).await?;
    for project in seen {
        assign(&world, &reader, role, project).await.map_err(text)?;
    }
    let mut kinds = Vec::new();
    if case.blocks {
        kinds.push(stored.kinds.0);
    }
    if case.related {
        kinds.push(stored.kinds.1);
    }
    let answer = issue_map(
        &world.ctx(),
        IssueMapQuery {
            session_token: reader.session_token.clone(),
            issue: stored.issues[case.start].0.to_string(),
            steps: case.steps,
            shown: KindsShown {
                kinds: Some(kinds),
                part_of: case.part_of,
            },
        },
    )
    .await
    .map(bb_policy::Authorized::into_value);
    Ok((stored, answer))
}

/// Whether the reader sees the issue in the case.
fn seen_in(stored: &Stored, case: &Case, issue: IssueId) -> bool {
    stored
        .issues
        .iter()
        .any(|(id, project)| *id == issue && case.sees[*project])
}

async fn no_chain_through_hidden(case: Case) -> Result<(), TestCaseError> {
    let (stored, answer) = map_of_case(&case).await.map_err(TestCaseError::fail)?;
    let start = stored.issues[case.start].0;
    let map = match answer {
        // A start the reader cannot see is not there, as an issue that does not exist.
        Err(error) => {
            prop_assert!(!seen_in(&stored, &case, start));
            prop_assert_eq!(error, AppError::Refused(ErrorCode::NotFound));
            return Ok(());
        }
        Ok(map) => map,
    };
    prop_assert!(seen_in(&stored, &case, start));
    for each in &map.issues {
        prop_assert!(
            seen_in(&stored, &case, each.issue.id),
            "a hidden issue is on the map"
        );
    }
    for line in &map.lines {
        prop_assert!(seen_in(&stored, &case, line.from) && seen_in(&stored, &case, line.to));
    }
    // Every issue past the start is one line away from an issue one step closer: the chain to it
    // runs on the map, through issues the reader sees.
    let steps: BTreeMap<IssueId, u32> = map
        .issues
        .iter()
        .filter_map(|each| Some((each.issue.id, each.steps?)))
        .collect();
    for (issue, away) in &steps {
        if *away == 0 {
            continue;
        }
        let closer = map.lines.iter().any(|line| {
            let other = if line.from == *issue {
                line.to
            } else if line.to == *issue {
                line.from
            } else {
                return false;
            };
            steps.get(&other) == Some(&(away - 1))
        });
        prop_assert!(closer, "{issue} has no chain back to the start on the map");
    }
    Ok(())
}

async fn the_map_of_what_is_seen(case: Case) -> Result<(), TestCaseError> {
    let (stored, answer) = map_of_case(&case).await.map_err(TestCaseError::fail)?;
    let Ok(map) = answer else {
        prop_assert!(!seen_in(&stored, &case, stored.issues[case.start].0));
        return Ok(());
    };
    let (issues, lines, more) = model(&stored, &case);
    let answered: BTreeMap<IssueId, u32> = map
        .issues
        .iter()
        .filter_map(|each| Some((each.issue.id, each.steps?)))
        .collect();
    prop_assert_eq!(answered.len(), map.issues.len());
    prop_assert_eq!(answered, issues);
    let drawn: BTreeSet<(IssueId, IssueId)> =
        map.lines.iter().map(|line| (line.from, line.to)).collect();
    prop_assert_eq!(drawn, lines);
    prop_assert_eq!(map.more, more);
    Ok(())
}

fn runtime() -> Result<tokio::runtime::Runtime, TestCaseError> {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .map_err(|error| TestCaseError::fail(error.to_string()))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    /// No issue on the map, and no line, is one the reader cannot see, and every issue reaches the
    /// start through issues they see.
    #[test]
    fn iss_16_no_chain_passes_through_a_hidden_issue(case in case()) {
        runtime()?.block_on(no_chain_through_hidden(case))?;
    }

    /// The answer is the map of the graph with the hidden issues taken out: the same issues at the
    /// same steps away, the same lines, and the same `more`.
    #[test]
    fn iss_16_the_map_is_the_map_of_the_issues_the_reader_sees(case in case()) {
        runtime()?.block_on(the_map_of_what_is_seen(case))?;
    }
}

#[tokio::test]
async fn iss_16_more_is_counted_on_the_walk_the_reader_sees() -> Result<(), String> {
    let world = World::new().await?;
    let open = a_project(&world, "OPEN").await?;
    let secret = a_project(&world, "SECRET").await?;
    let reader = member(
        &world,
        "reader@example.test",
        StartingRole::TeamMember,
        open,
    )
    .await
    .map_err(text)?;
    let (blocks, _) = first_and_last_kind(&world).await?;
    let start = an_issue(&world, open).await?;
    // The start has exactly as many visible neighbours as the map has room for, and as many
    // hidden ones again: the hidden ones are no reason for `more`.
    let mut neighbours = Vec::new();
    for at in 0..(MAP_ISSUES_MOST * 2 - 2) {
        let project = if at % 2 == 0 { open } else { secret };
        neighbours.push(an_issue(&world, project).await?);
    }
    let mut ids = SequentialIds::default();
    {
        let mut shared = world.store.lock();
        for other in &neighbours {
            shared.state.relations.push(Relation {
                id: RelationId::generate(&mut ids).map_err(|error| error.to_string())?,
                kind: blocks,
                from: start,
                to: *other,
                added_by: world.admin.person.id,
                added_at: Timestamp::UNIX_EPOCH,
            });
        }
    }
    let ask = |steps| IssueMapQuery {
        session_token: reader.session_token.clone(),
        issue: start.to_string(),
        steps,
        shown: KindsShown::default(),
    };
    let map = issue_map(&world.ctx(), ask(Some(1)))
        .await
        .map_err(text)?
        .into_value();
    assert_eq!(map.issues.len(), MAP_ISSUES_MOST);
    assert!(!map.more);
    // One more visible issue, and the map says there are more.
    let extra = an_issue(&world, open).await?;
    world.store.lock().state.relations.push(Relation {
        id: RelationId::generate(&mut ids).map_err(|error| error.to_string())?,
        kind: blocks,
        from: extra,
        to: start,
        added_by: world.admin.person.id,
        added_at: Timestamp::UNIX_EPOCH,
    });
    let map = issue_map(&world.ctx(), ask(None))
        .await
        .map_err(text)?
        .into_value();
    assert_eq!(map.issues.len(), MAP_ISSUES_MOST);
    assert!(map.more);
    // More steps than the search language walks are no request.
    let refused = issue_map(&world.ctx(), ask(Some(MAX_STEPS + 1)))
        .await
        .err();
    assert_eq!(refused, Some(AppError::Refused(ErrorCode::InvalidRequest)));
    Ok(())
}

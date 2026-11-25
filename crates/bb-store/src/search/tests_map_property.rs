//! The walks of the search and of the relation map on both databases (ISS-16, QRY-04, AI-33): random
//! graphs of relations and sub-issues over three projects, a person who reads a random set of them,
//! and every step of the map compared with the same step worked out in memory over the issues the
//! person reads; and the cut of an agent's work, which `connected to`, `under` and the map all keep.

use std::collections::BTreeSet;

use bb_app::access::PolicyStore as _;
use bb_app::issues::IssueStore as _;
use bb_app::ports::ChangeStore as _;
use bb_app::relations::RelationStore as _;
use bb_app::search::{MapStep, SearchRequest, SearchStore as _, WorkOf};
use bb_core::id::{IssueId, ProjectId, RelationId, UserId};
use bb_core::issue::Issue;
use bb_core::relation::{IssueParent, Relation};
use bb_testing::{TestResult, each_database};
use proptest::prelude::*;
use proptest::strategy::ValueTree as _;
use proptest::test_runner::TestRunner;

use super::tests::{Data, project_row, request, request_with};
use super::tests_agent::agent_fields;
use super::tests_agent_work::{Given, Made, NOW, given, hand_over};
use super::tests_more::titles_of;
use crate::AuthStorage;
use crate::test_support::{at, event, issue};

/// How many random graphs each database is given.
const CASES: usize = 16;
/// The first number of the codes of the issues of the graphs, past those of the seed.
const FIRST_CODE: i64 = 1_000;

/// A random graph: the project of each issue (0 the seed's, 1 the seed's hidden one, 2 a third),
/// the lines (two issues and whether the line is *part of*), which projects the person reads, and
/// whether *part of* is walked.
#[derive(Clone, Debug)]
struct Graph {
    issues: Vec<usize>,
    lines: Vec<(usize, usize, bool)>,
    reads: [bool; 3],
    part_of: bool,
}

fn graph() -> impl Strategy<Value = Graph> {
    (2_usize..10)
        .prop_flat_map(|count| {
            (
                proptest::collection::vec(0_usize..3, count),
                proptest::collection::vec((0..count, 0..count, any::<bool>()), 0..count * 2),
                any::<[bool; 3]>(),
                any::<bool>(),
            )
        })
        .prop_map(|(issues, lines, reads, part_of)| Graph {
            issues,
            lines,
            reads,
            part_of,
        })
}

/// The issues, relations and sub-issues of a graph, stored in one change.
async fn store_graph(
    storage: &AuthStorage,
    data: &mut Data,
    projects: [ProjectId; 3],
    (graph, round): (&Graph, usize),
) -> Result<(Vec<Issue>, Vec<Relation>, Vec<IssueParent>), Box<dyn std::error::Error + Send + Sync>>
{
    let base = FIRST_CODE + i64::try_from(round * 100)?;
    let made: Vec<Issue> = graph
        .issues
        .iter()
        .zip(base..)
        .map(|(project, code)| issue(&mut data.source, projects[*project], data.issue_type, code))
        .collect();
    let me: UserId = data.me;
    let mut relations = Vec::new();
    let mut parents: Vec<IssueParent> = Vec::new();
    for (one, other, part_of) in &graph.lines {
        if one == other {
            continue;
        }
        let (first, second) = (made[*one].id, made[*other].id);
        if *part_of {
            // The earlier issue is the parent, so sub-issues never make a circle.
            let (child, parent) = if one > other {
                (first, second)
            } else {
                (second, first)
            };
            if parents.iter().all(|row| row.child != child) {
                parents.push(IssueParent {
                    child,
                    parent,
                    set_by: me,
                    set_at: at(0),
                });
            }
        } else if !relations
            .iter()
            .any(|relation: &Relation| relation.from == first && relation.to == second)
        {
            relations.push(Relation {
                id: RelationId::generate(&mut data.source)?,
                kind: data.blocks,
                from: first,
                to: second,
                added_by: me,
                added_at: at(0),
            });
        }
    }
    let (issues, lines, rows) = (made.clone(), relations.clone(), parents.clone());
    storage
        .record_change(&event(&mut data.source, 7), move |tx| {
            Box::pin(async move {
                for each in &issues {
                    AuthStorage::insert_issue(tx, each, &[]).await?;
                }
                for relation in &lines {
                    AuthStorage::insert_relation(tx, relation).await?;
                }
                for parent in &rows {
                    AuthStorage::set_issue_parent(tx, parent).await?;
                }
                Ok(())
            })
        })
        .await?;
    Ok((made, relations, parents))
}

/// The next step worked out in memory: the issues one line away from `near` that the person reads
/// and that are not known yet, by ID.
fn next_in_memory(
    (relations, parents): (&[Relation], &[IssueParent]),
    near: &[IssueId],
    known: &[IssueId],
    reads: impl Fn(IssueId) -> bool,
    part_of: bool,
) -> Vec<IssueId> {
    let mut lines: Vec<(IssueId, IssueId)> = relations
        .iter()
        .map(|relation| (relation.from, relation.to))
        .collect();
    if part_of {
        lines.extend(parents.iter().map(|row| (row.child, row.parent)));
    }
    let found: BTreeSet<IssueId> = lines
        .iter()
        .filter_map(|(one, other)| {
            if near.contains(one) {
                Some(*other)
            } else if near.contains(other) {
                Some(*one)
            } else {
                None
            }
        })
        .filter(|issue| !known.contains(issue) && reads(*issue))
        .collect();
    found.into_iter().collect()
}

/// Walks one graph step by step, with the statement of the store and in memory, and says where they
/// differ.
async fn walk_both(
    storage: &AuthStorage,
    asked: &SearchRequest,
    (graph, kind): (&Graph, bb_core::id::RelationKindId),
    stored: &(Vec<Issue>, Vec<Relation>, Vec<IssueParent>),
) -> Result<(), String> {
    let (issues, relations, parents) = stored;
    let reads = |id: IssueId| {
        issues
            .iter()
            .zip(&graph.issues)
            .any(|(each, project)| each.id == id && graph.reads[*project])
    };
    let start = issues[0].id;
    let mut known = vec![start];
    let mut near = vec![start];
    for away in 1..=bb_searchlang::MAX_STEPS {
        let wanted = next_in_memory((relations, parents), &near, &known, reads, graph.part_of);
        let ask = |most: u64| MapStep {
            near: near.clone(),
            known: known.clone(),
            kinds: vec![kind],
            part_of: graph.part_of,
            most,
            began: std::time::Instant::now(),
        };
        let failed = |failure| format!("{failure:?}");
        let found = storage.map_step(asked, &ask(1_000)).await.map_err(failed)?;
        if found != wanted {
            return Err(format!(
                "step {away} of {graph:?}: {found:?} is not {wanted:?}"
            ));
        }
        // Stopped inside the statement, the step keeps the first by ID.
        let first = storage.map_step(asked, &ask(1)).await.map_err(failed)?;
        if first != wanted[..wanted.len().min(1)] {
            return Err(format!("step {away} of {graph:?}, cut to one: {first:?}"));
        }
        known.extend(found.iter().copied());
        near = found;
        if near.is_empty() {
            break;
        }
    }
    Ok(())
}

#[tokio::test]
async fn iss_16_every_step_of_the_map_is_the_step_over_what_the_person_reads() -> TestResult {
    each_database("map_step_property", async |db| {
        let Given {
            fixture, mut data, ..
        } = given(db).await?;
        let storage = &fixture.storage;
        let third = ProjectId::generate(&mut data.source)?;
        let row = project_row(third, "THIRD");
        storage
            .record_change(&event(&mut data.source, 6), move |tx| {
                Box::pin(async move { AuthStorage::insert_project(tx, &row).await })
            })
            .await?;
        let projects = [data.project, data.hidden.project, third];
        let mut runner = TestRunner::deterministic();
        for round in 0..CASES {
            let mut drawn = graph()
                .new_tree(&mut runner)
                .map_err(|error| error.to_string())?
                .current();
            // The map starts from an issue the person reads.
            drawn.reads[drawn.issues[0]] = true;
            let stored = store_graph(storage, &mut data, projects, (&drawn, round)).await?;
            let mut asked = request(&data, "", None, 0);
            asked.sight.all = projects
                .iter()
                .zip(drawn.reads)
                .filter_map(|(project, reads)| reads.then_some(*project))
                .collect();
            walk_both(storage, &asked, (&drawn, data.blocks), &stored).await?;
        }
        Ok(())
    })
    .await
}

/// The cut of an agent's work: the issues of the seed handed over to the agent, the rest outside.
async fn cut_to(
    given: &mut Given,
    handed: &[usize],
) -> Result<SearchRequest, Box<dyn std::error::Error + Send + Sync>> {
    let mut made = Made::new();
    let issues: Vec<IssueId> = handed.iter().map(|at| given.data.issues[*at].id).collect();
    hand_over(
        &given.fixture.storage,
        &mut made,
        (given.owner, given.fixer),
        &issues,
        (Vec::new(), Vec::new()),
    )
    .await?;
    let mut asked = request_with(&given.data, "", None, 100, agent_fields(), Vec::new());
    asked.only = Some(WorkOf {
        agent: given.fixer,
        at: at(NOW),
    });
    Ok(asked)
}

#[tokio::test]
async fn ai_33_the_walks_of_an_agent_pass_only_through_its_work() -> TestResult {
    each_database("ai_33_walks_cut_to_work", async |db| {
        let mut given = given(db).await?;
        // The seed: Printer jams (1) blocks Login broken (2), which blocks Idea (6); Docs typo (3)
        // sits under Login broken and Crash on save (5) under Docs typo. The agent holds Printer
        // jams, Idea and Crash on save, not the two between.
        let asked = cut_to(&mut given, &[0, 5, 4]).await?;
        let storage = &given.fixture.storage;
        let found = async |text: &str, only: Option<WorkOf>| {
            let mut ask = request_with(&given.data, text, None, 100, agent_fields(), Vec::new());
            ask.only = only;
            titles_of(storage, &ask).await
        };
        // Without the cut both walks reach the far issue; with it, neither passes the one between.
        for text in ["connected to ACME-1 within 2 steps", "under ACME-2"] {
            assert!(found(text, asked.only).await?.is_empty(), "{text}");
        }
        assert!(
            found("connected to ACME-1 within 2 steps", None)
                .await?
                .contains(&"Idea".to_owned())
        );
        assert!(
            found("under ACME-2", None)
                .await?
                .contains(&"Crash on save".to_owned())
        );
        let mut person = asked.clone();
        person.only = None;
        // The map's step keeps the same cut.
        let step = MapStep {
            near: vec![given.data.issues[0].id],
            known: vec![given.data.issues[0].id],
            kinds: vec![given.data.blocks],
            part_of: true,
            most: 10,
            began: std::time::Instant::now(),
        };
        let failed = |failure| format!("{failure:?}");
        assert!(
            storage
                .map_step(&asked, &step)
                .await
                .map_err(failed)?
                .is_empty()
        );
        assert_eq!(
            storage.map_step(&person, &step).await.map_err(failed)?,
            [given.data.issues[1].id]
        );
        Ok(())
    })
    .await
}

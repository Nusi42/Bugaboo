//! The part of the canonical dump that is about the issues: how many there are in each project,
//! of which type, how far along, how many sub-issues, relations of each kind (and how long the
//! longest chain of *blocks* is, and how many circles there are), tags, labels, values of own
//! fields, comments (and how many were edited), and the states left behind (a lock held, a draft
//! waiting). Only counts and names: IDs and the readable IDs of random projects are random by
//! design.
//!
//! TEST-SAMPLE-DATA holds these numbers against the plan in `tools/dev/sample.json`.

use std::collections::{BTreeMap, BTreeSet};

use bb_app::editing::EditStore as _;
use bb_app::fields::FieldStore as _;
use bb_app::issues::IssueStore as _;
use bb_app::projects::ProjectStore as _;
use bb_app::relations::RelationStore as _;
use bb_app::tags::TagStore as _;
use bb_core::event::{DomainEvent, EntityKind, RecordKind};
use bb_core::id::{EntityId, IssueId, ProjectId};
use bb_core::issue::Issue;
use bb_core::message::Message;
use bb_core::relation::{RelationKind, STARTING_RELATION_KINDS};
use jiff::Timestamp;
use serde::Serialize;

use crate::error::SampleError;
use crate::generate::Sample;

/// How many issues a page of a project holds when the dump reads them all.
const PAGE: u64 = 1000;

/// The issues of a project (with its sub-projects counted apart).
#[derive(Debug, Default, Serialize)]
pub(crate) struct DumpProjectIssues {
    short_name: String,
    total: u64,
    by_type: BTreeMap<String, u64>,
    by_status: BTreeMap<String, u64>,
    by_priority: BTreeMap<String, u64>,
}

/// The issues and what belongs to them.
#[derive(Debug, Default, Serialize)]
pub(crate) struct DumpIssues {
    /// How many issues the sample was asked to make.
    planned: u32,
    projects: Vec<DumpProjectIssues>,
    by_type: BTreeMap<String, u64>,
    sub_issues: u64,
    goals_with_sub_issues: u64,
    /// Relations by the forward name of their kind.
    relations: BTreeMap<String, u64>,
    longest_blocks_chain: u64,
    blocks_circles: u64,
    tags_on_issues: u64,
    labels_on_issues: u64,
    /// Values of own fields by the name of the field.
    own_field_values: BTreeMap<String, u64>,
    comments: u64,
    edited_comments: u64,
    edit_locks: u64,
    drafts: u64,
}

impl DumpIssues {
    /// The events about comments, counted: those that wrote a text, and those that changed one.
    fn count_comments(&mut self, events: &[DomainEvent]) {
        let comment = RecordKind::Comment.code();
        for event in events.iter().filter(|e| e.entity.kind.as_str() == comment) {
            if let Some(text) = event.changes.iter().find(|change| change.field == "text") {
                if text.before.is_none() {
                    self.comments += 1;
                } else {
                    self.edited_comments += 1;
                }
            }
        }
    }
}

impl Sample {
    /// Reads the issues and what belongs to them from the store, and counts the comments in the
    /// events.
    pub(crate) async fn issues(&self, events: &[DomainEvent]) -> Result<DumpIssues, SampleError> {
        let mut dump = DumpIssues {
            planned: self.left_behind.issues,
            ..DumpIssues::default()
        };
        dump.count_comments(events);
        if self.left_behind.issues == 0 {
            return Ok(dump);
        }
        let storage = &self.storage;
        let projects: BTreeMap<ProjectId, String> = storage
            .project_tree()
            .await?
            .into_iter()
            .map(|project| (project.id, project.short_name.to_string()))
            .collect();
        let types: BTreeMap<_, _> = storage
            .issue_types()
            .await?
            .into_iter()
            .map(|issue_type| (issue_type.id, issue_type.name))
            .collect();

        let mut issues: Vec<Issue> = Vec::new();
        for (id, short_name) in &projects {
            let mut here = DumpProjectIssues {
                short_name: short_name.clone(),
                ..DumpProjectIssues::default()
            };
            let mut before = None;
            loop {
                let page = storage.issues_page(*id, None, before, PAGE).await?;
                before = page.last().map(|issue| issue.id);
                let full = page.len() as u64 == PAGE;
                for issue in page {
                    let kind = types.get(&issue.issue_type).cloned().unwrap_or_default();
                    here.total += 1;
                    *dump.by_type.entry(kind.clone()).or_default() += 1;
                    *here.by_type.entry(kind).or_default() += 1;
                    *here
                        .by_status
                        .entry(issue.status.code().to_owned())
                        .or_default() += 1;
                    *here
                        .by_priority
                        .entry(issue.priority.code().to_owned())
                        .or_default() += 1;
                    issues.push(issue);
                }
                if !full {
                    break;
                }
            }
            if here.total > 0 {
                dump.projects.push(here);
            }
        }
        dump.projects
            .sort_by(|one, other| one.short_name.cmp(&other.short_name));

        self.count_sub_issues_and_relations(&issues, &mut dump)
            .await?;
        let ids: Vec<IssueId> = issues.iter().map(|issue| issue.id).collect();
        dump.tags_on_issues += storage.tags_of_issues(ids.clone()).await?.len() as u64;
        dump.labels_on_issues += storage.labels_of_issues(ids.clone()).await?.len() as u64;
        let fields = storage.field_definitions().await?;
        let kind: EntityKind = RecordKind::Issue.into();
        let entities: Vec<EntityId> = ids.iter().map(|id| EntityId::from(*id)).collect();
        for value in storage.field_values(kind, entities).await? {
            if let Some(definition) = fields
                .iter()
                .find(|definition| definition.field.id == value.field)
            {
                *dump
                    .own_field_values
                    .entry(definition.field.name.clone())
                    .or_default() += 1;
            }
        }
        for what in &self.left_behind.locks {
            dump.edit_locks += u64::from(storage.edit_lock(what.clone()).await?.is_some());
        }
        for (author, target) in &self.left_behind.drafts {
            let kept = storage
                .draft(*author, target.clone(), Timestamp::UNIX_EPOCH)
                .await?;
            dump.drafts += u64::from(kept.is_some());
        }
        Ok(dump)
    }

    /// The sub-issues under the goals, and the relations of every issue by kind, with the longest
    /// chain of *blocks* and the circles.
    async fn count_sub_issues_and_relations(
        &self,
        issues: &[Issue],
        dump: &mut DumpIssues,
    ) -> Result<(), SampleError> {
        let storage = &self.storage;
        let kinds: Vec<RelationKind> = storage.relation_kinds().await?;
        // The kind *blocks* is the first of the starting kinds, found by its name.
        let blocks_name = STARTING_RELATION_KINDS
            .first()
            .map(|starting| Message::from(starting.forward).render());
        let blocks = kinds
            .iter()
            .find(|kind| Some(&kind.forward) == blocks_name.as_ref())
            .map(|kind| kind.id);
        let mut edges: BTreeMap<IssueId, Vec<IssueId>> = BTreeMap::new();
        for issue in issues {
            let children = storage.sub_issues_of(issue.id).await?.len() as u64;
            dump.sub_issues += children;
            dump.goals_with_sub_issues += u64::from(children > 0);
            for relation in storage.relations_of(issue.id).await? {
                // Each relation is read from both ends: count it at the end it starts from.
                if relation.from != issue.id {
                    continue;
                }
                let name = kinds
                    .iter()
                    .find(|kind| kind.id == relation.kind)
                    .map(|kind| kind.forward.clone())
                    .unwrap_or_default();
                *dump.relations.entry(name).or_default() += 1;
                if Some(relation.kind) == blocks {
                    edges.entry(relation.from).or_default().push(relation.to);
                }
            }
        }
        let (longest, circles) = shape_of(&edges);
        dump.longest_blocks_chain = longest;
        dump.blocks_circles = circles;
        Ok(())
    }
}

/// The length of the longest chain of steps in a graph that has no circle in it (the issues of a
/// circle are left out, so it makes no difference where it would be cut), and the number of
/// circles.
fn shape_of(edges: &BTreeMap<IssueId, Vec<IssueId>>) -> (u64, u64) {
    let reach: BTreeMap<IssueId, BTreeSet<IssueId>> = edges
        .keys()
        .map(|start| (*start, reachable(*start, edges)))
        .collect();
    let cyclic: BTreeSet<IssueId> = reach
        .iter()
        .filter(|(start, found)| found.contains(start))
        .map(|(start, _)| *start)
        .collect();
    // A circle is the set of issues that reach each other: named by its smallest issue.
    let circles: BTreeSet<IssueId> = cyclic
        .iter()
        .filter_map(|one| {
            cyclic
                .iter()
                .find(|other| reach[one].contains(*other) && reach[*other].contains(one))
                .copied()
        })
        .collect();
    let mut depth: BTreeMap<IssueId, u64> = BTreeMap::new();
    let longest = edges
        .keys()
        .filter(|start| !cyclic.contains(start))
        .map(|start| chain_from(*start, edges, &cyclic, &mut depth))
        .max()
        .unwrap_or(0);
    (longest, circles.len() as u64)
}

/// Every issue that can be reached from `start` by following the edges.
fn reachable(start: IssueId, edges: &BTreeMap<IssueId, Vec<IssueId>>) -> BTreeSet<IssueId> {
    let mut found = BTreeSet::new();
    let mut open = vec![start];
    while let Some(at) = open.pop() {
        for next in edges.get(&at).map_or(&[][..], Vec::as_slice) {
            if found.insert(*next) {
                open.push(*next);
            }
        }
    }
    found
}

/// The longest chain that starts at `at` and passes through none of the issues of a circle.
fn chain_from(
    at: IssueId,
    edges: &BTreeMap<IssueId, Vec<IssueId>>,
    cyclic: &BTreeSet<IssueId>,
    depth: &mut BTreeMap<IssueId, u64>,
) -> u64 {
    if let Some(known) = depth.get(&at) {
        return *known;
    }
    let longest = edges
        .get(&at)
        .map_or(&[][..], Vec::as_slice)
        .iter()
        .filter(|next| !cyclic.contains(next))
        .map(|next| 1 + chain_from(*next, edges, cyclic, depth))
        .max()
        .unwrap_or(0);
    depth.insert(at, longest);
    longest
}

#[cfg(test)]
mod tests {
    use bb_app::context::SystemIds;

    use super::*;

    #[test]
    fn the_longest_chain_leaves_out_a_circle_and_the_circle_is_counted_once() {
        let issues: Vec<IssueId> = (0..9)
            .map(|_| IssueId::generate(&mut SystemIds).unwrap())
            .collect();
        let mut edges: BTreeMap<IssueId, Vec<IssueId>> = BTreeMap::new();
        // A chain of five steps: 0 > 1 > 2 > 3 > 4 > 5, a circle 6 > 7 > 8 > 6, and a step from
        // the chain into the circle that must not make the chain longer.
        for pair in issues[..6].windows(2) {
            edges.entry(pair[0]).or_default().push(pair[1]);
        }
        for (one, other) in [(6, 7), (7, 8), (8, 6), (5, 6)] {
            edges.entry(issues[one]).or_default().push(issues[other]);
        }
        assert_eq!(shape_of(&edges), (5, 1));
        assert_eq!(shape_of(&BTreeMap::new()), (0, 0));
    }
}

//! The relations of the sample: how many of each kind, between which issues, including long
//! chains of *blocks* (deeper than four steps) and one deliberate circle, to see the warning.

use std::collections::BTreeSet;

use crate::issues_plan::PlannedIssue;
use crate::random::SeededRandom;

/// The place of the kind *blocks* among the starting kinds of relation.
pub(crate) const BLOCKS: usize = 0;
/// The place of *duplicate of*.
const DUPLICATE: usize = 1;
/// How the kinds share the random relations (in the order of the starting kinds: blocks,
/// duplicate of, causes, related to), in percent.
const SHARES: [usize; 4] = [30, 12, 18, 40];
/// How many issues one chain of *blocks* has: five steps from the first to the last.
const CHAIN: usize = 6;
/// One chain of *blocks* for so many relations.
const ONE_CHAIN_FOR: usize = 120;

/// A relation as planned: the issue it is read from, the other, and the place of the kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PlannedRelation {
    pub(crate) from: usize,
    pub(crate) to: usize,
    pub(crate) kind: usize,
}

/// The relations planned so far and the pairs they are between, so no pair is related twice.
#[derive(Default)]
struct Made {
    relations: Vec<PlannedRelation>,
    pairs: BTreeSet<(usize, usize)>,
}

impl Made {
    fn add(&mut self, from: usize, to: usize, kind: usize) {
        if from != to && self.pairs.insert((from.min(to), from.max(to))) {
            self.relations.push(PlannedRelation { from, to, kind });
        }
    }
}

/// Plans `count` relations between the issues (fewer for a sample too small for them): mostly
/// inside one top-level project, with the chains and the circle first.
pub(crate) fn plan_relations(
    count: usize,
    issues: &[PlannedIssue],
    random: &mut SeededRandom,
) -> Vec<PlannedRelation> {
    let mut families: Vec<Vec<usize>> = Vec::new();
    for (at, issue) in issues.iter().enumerate() {
        if families.len() <= issue.family {
            families.resize(issue.family + 1, Vec::new());
        }
        families[issue.family].push(at);
    }
    families.retain(|members| members.len() >= CHAIN);
    let mut made = Made::default();

    // The chains and the circle, when the count has room for them.
    if !families.is_empty() && count >= CHAIN + 3 {
        for _ in 0..(count / ONE_CHAIN_FOR).max(1) {
            let members = &families[random.below(families.len())];
            let start = random.below(members.len() - CHAIN + 1);
            for step in start..start + CHAIN - 1 {
                made.add(members[step], members[step + 1], BLOCKS);
            }
        }
        // The circle: three issues that block each other round and round.
        let members = &families[random.below(families.len())];
        let first = random.below(members.len() - 2);
        let (x, y, z) = (members[first], members[first + 1], members[first + 2]);
        made.add(x, y, BLOCKS);
        made.add(y, z, BLOCKS);
        made.add(z, x, BLOCKS);
    }

    // The rest: a pair in the same top-level project, now and then one from another. An earlier
    // issue blocks a later one (so the random relations never make a circle), and a later one is
    // the duplicate of an earlier.
    let mut tries = 0;
    while made.relations.len() < count && tries < count * 20 + 100 && issues.len() > 1 {
        tries += 1;
        let (one, other) = if random.chance(80) && !families.is_empty() {
            let members = &families[random.below(families.len())];
            (
                members[random.below(members.len())],
                members[random.below(members.len())],
            )
        } else {
            (random.below(issues.len()), random.below(issues.len()))
        };
        let (low, high) = (one.min(other), one.max(other));
        let kind = kind_by_share(random);
        if kind == DUPLICATE {
            made.add(high, low, kind);
        } else {
            made.add(low, high, kind);
        }
    }
    made.relations
        .truncate(count.max(CHAIN + 3).min(made.relations.len()));
    made.relations
}

/// A kind, by the shares of [`SHARES`].
fn kind_by_share(random: &mut SeededRandom) -> usize {
    let mut draw = random.below(100);
    for (kind, share) in SHARES.iter().enumerate() {
        if draw < *share {
            return kind;
        }
        draw -= share;
    }
    SHARES.len() - 1
}

#[cfg(test)]
mod tests {
    use bb_core::issue::Priority;

    use super::*;

    fn issues(count: usize) -> Vec<PlannedIssue> {
        (0..count)
            .map(|at| PlannedIssue {
                place: "APP".to_owned(),
                family: at % 2,
                kind: 0,
                title: String::new(),
                description: String::new(),
                priority: Priority::Normal,
                assignees: Vec::new(),
                fields: Vec::new(),
                parent: None,
                tags: Vec::new(),
                labels: Vec::new(),
            })
            .collect()
    }

    #[test]
    fn there_are_as_many_relations_as_asked_each_pair_once_with_chains_and_one_circle() {
        let issues = issues(2000);
        let relations = plan_relations(600, &issues, &mut SeededRandom::new(9));
        assert_eq!(relations.len(), 600);
        let pairs: BTreeSet<_> = relations
            .iter()
            .map(|r| (r.from.min(r.to), r.from.max(r.to)))
            .collect();
        assert_eq!(pairs.len(), relations.len());
        for kind in 0..SHARES.len() {
            assert!(relations.iter().any(|r| r.kind == kind), "{kind}");
        }
        // A chain deeper than four steps: follow *blocks* from some issue, towards later issues.
        let blocks: Vec<_> = relations.iter().filter(|r| r.kind == BLOCKS).collect();
        let depth = |from: usize| {
            let (mut depth, mut at) = (0, from);
            while let Some(next) = blocks.iter().find(|r| r.from == at && r.to > r.from) {
                depth += 1;
                at = next.to;
            }
            depth
        };
        assert!(blocks.iter().any(|r| depth(r.from) > 4));
        // The one circle of three: each relation of it is followed by two more that lead back.
        let in_circle = blocks
            .iter()
            .filter(|a| {
                blocks.iter().any(|b| {
                    b.from == a.to && blocks.iter().any(|c| c.from == b.to && c.to == a.from)
                })
            })
            .count();
        assert_eq!(in_circle, 3);
    }

    #[test]
    fn a_small_sample_gets_the_relations_it_can_hold() {
        let relations = plan_relations(9, &issues(30), &mut SeededRandom::new(1));
        assert_eq!(relations.len(), 9);
        assert!(plan_relations(5, &[], &mut SeededRandom::new(1)).is_empty());
    }
}

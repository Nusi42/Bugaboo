//! What the issues of the large dataset stand on, read once: the issue types, kinds of relation,
//! own fields, tags, labels and files, the words in both languages, and for every project and issue
//! type the road an issue takes through the workflow it is under (its *ladder*).

use std::collections::VecDeque;

use bb_app::projects::ProjectStore as _;
use bb_app::tags::TagStore as _;
use bb_app::workflows::WorkflowStore as _;
use bb_core::id::{FieldId, IssueTypeId, RelationKindId, TagId};
use bb_core::issue::StatusGroup;
use bb_core::own_fields::ChoiceColor;
use bb_core::tags::{Label, Tag};
use bb_core::version::FIRST_VERSION;
use bb_core::workflow::{StatusKey, StoredWorkflow, find};
use bb_dev_words::LargePlan;
use bb_dev_words::sample::{FieldPlan, IssuePlan};
use jiff::Timestamp;
use serde::Deserialize;

use crate::error::SampleError;
use crate::issues_setup::{MadeField, Setup};
use crate::issues_words::IssueWords;
use crate::large_ids::{ordered, rng_for};
use crate::large_org::{Org, Words};

/// How many different file records the dataset has: a file record is one row; issues share them.
pub(crate) const FILE_RECORDS: usize = 64;

#[derive(Deserialize)]
pub(crate) struct Comments {
    pub(crate) comments: Vec<String>,
}

/// The German words of the dataset.
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct German {
    pub(crate) titles: Vec<String>,
    pub(crate) descriptions: Vec<String>,
    pub(crate) comments: Vec<String>,
}

/// The road an issue takes through a workflow: from its first status to the nearest one in the
/// group *Done*, and whether a status change on the way asks for a person.
#[derive(Clone, Debug)]
pub(crate) struct Ladder {
    pub(crate) steps: Vec<(StatusKey, StatusGroup)>,
    pub(crate) needs_person: bool,
}

/// Everything the making of issues reads and never changes.
pub(crate) struct Bank {
    pub(crate) plan: LargePlan,
    pub(crate) names: IssuePlan,
    pub(crate) words: IssueWords,
    pub(crate) english: Vec<String>,
    pub(crate) german: German,
    pub(crate) types: Vec<IssueTypeId>,
    pub(crate) kinds: Vec<RelationKindId>,
    pub(crate) fields: Vec<(FieldPlan, MadeField)>,
    pub(crate) cost_center: FieldId,
    pub(crate) tags: Vec<Tag>,
    pub(crate) labels: Vec<Label>,
    pub(crate) ladders: Vec<Vec<Ladder>>,
    /// The short name of the project at the top of each place's family.
    pub(crate) families: Vec<String>,
    pub(crate) files: Vec<bb_core::files::StoredFile>,
}

/// What changes while issues are made, project by project.
pub(crate) struct Counters {
    pub(crate) last: Vec<i64>,
    pub(crate) recent: Vec<VecDeque<bb_core::id::IssueId>>,
    pub(crate) goals: Vec<Vec<(bb_core::id::IssueId, usize)>>,
    pub(crate) codes: Vec<std::collections::HashSet<String>>,
}

impl Counters {
    pub(crate) fn new(places: usize) -> Self {
        Self {
            last: vec![0; places],
            recent: vec![VecDeque::new(); places],
            goals: vec![Vec::new(); places],
            codes: vec![std::collections::HashSet::new(); places],
        }
    }
}

/// The ladder of a workflow.
fn ladder_of(workflow: &StoredWorkflow) -> Ladder {
    let content = &workflow.content;
    let Some(first) = content.initial().cloned() else {
        return Ladder {
            steps: Vec::new(),
            needs_person: false,
        };
    };
    let group = |key: &StatusKey| {
        content
            .status(key)
            .map_or(StatusGroup::NotStarted, |status| status.group)
    };
    // Breadth first, so the road is the shortest one.
    let mut roads: Vec<Vec<StatusKey>> = vec![vec![first.clone()]];
    let mut seen = vec![first];
    let mut at = 0;
    while at < roads.len() {
        let road = roads[at].clone();
        at += 1;
        let Some(last) = road.last() else { continue };
        if group(last) == StatusGroup::Done && road.len() > 1 {
            return finish(content, &road, group);
        }
        for change in content.changes_from(last) {
            if !seen.contains(&change.to) {
                seen.push(change.to.clone());
                let mut next = road.clone();
                next.push(change.to.clone());
                roads.push(next);
            }
        }
    }
    let alone = roads.into_iter().next().unwrap_or_default();
    finish(content, &alone, group)
}

fn finish(
    content: &bb_core::workflow::WorkflowContent,
    road: &[StatusKey],
    group: impl Fn(&StatusKey) -> StatusGroup,
) -> Ladder {
    let needs_person = road.windows(2).any(|pair| {
        content
            .change(&pair[0], &pair[1])
            .is_some_and(|change| !change.rules.fields().is_empty())
    });
    Ladder {
        steps: road.iter().map(|key| (key.clone(), group(key))).collect(),
        needs_person,
    }
}

/// Reads what the making of issues stands on.
///
/// # Errors
/// [`SampleError`] when the store fails or the words are unusable.
pub(crate) async fn read(
    world: &crate::generate::World<'_>,
    setup: &Setup,
    cost_center: FieldId,
    org: &Org,
    (plan, names): (&LargePlan, &IssuePlan),
    (at, words): (Timestamp, &Words),
) -> Result<Bank, SampleError> {
    let storage = world.storage;
    let uses = storage.workflow_uses().await?;
    let mut workflows: Vec<(bb_core::id::WorkflowId, Ladder)> = Vec::new();
    for used in &uses {
        if workflows.iter().all(|(id, _)| *id != used.workflow)
            && let Some(found) = storage.stored_workflow(used.workflow).await?
        {
            workflows.push((used.workflow, ladder_of(&found)));
        }
    }
    let ladders = org
        .places
        .iter()
        .map(|place| {
            setup
                .types
                .iter()
                .map(|issue_type| {
                    find(&place.chain, Some(*issue_type), &uses)
                        .and_then(|used| {
                            workflows
                                .iter()
                                .find(|(id, _)| *id == used.workflow)
                                .map(|(_, ladder)| ladder.clone())
                        })
                        .unwrap_or(Ladder {
                            steps: Vec::new(),
                            needs_person: false,
                        })
                })
                .collect()
        })
        .collect();
    let tree = storage.project_tree().await?;
    let families = org
        .places
        .iter()
        .map(|place| {
            let top = place.chain.last().copied().unwrap_or(place.id);
            tree.iter()
                .find(|project| project.id == top)
                .map_or_else(String::new, |project| {
                    project.short_name.as_str().to_owned()
                })
        })
        .collect();
    let tags = names
        .tags
        .iter()
        .enumerate()
        .map(|(at_tag, name)| {
            Ok(Tag {
                id: ordered(
                    TagId::from_parts,
                    at,
                    &mut rng_for(world.seed, 0x7A6, at_tag as u64),
                )?,
                name: name.clone(),
                color: ChoiceColor::nth(at_tag),
                created_at: at,
                version: FIRST_VERSION,
            })
        })
        .collect::<Result<_, SampleError>>()?;
    let labels: Vec<Label> = storage
        .labels()
        .await?
        .into_iter()
        .filter(|label| label.retired_at.is_none())
        .collect();
    Ok(Bank {
        plan: plan.clone(),
        names: names.clone(),
        words: IssueWords::bundled()?,
        english: words.english.comments.clone(),
        german: words.german.clone(),
        types: setup.types.clone(),
        kinds: setup.kinds.clone(),
        fields: names
            .fields
            .iter()
            .cloned()
            .zip(setup.fields.iter().cloned())
            .collect(),
        cost_center,
        tags,
        labels,
        ladders,
        families,
        files: crate::large_files::records(world.seed, at),
    })
}

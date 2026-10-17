//! The preview of a change to many issues is what then happens (ISS-09), as a property: for any
//! person (an administrator, a reader, an editor who may not change the status, an editor), any
//! selection (issues in a visible and in a hidden project, in the trash, that have the label
//! already, that are in the status asked for, that do not exist) and any action, the change skips
//! the issues the preview said it would skip for the reasons it gave, leaves them as they were, and
//! changes the others as the preview's detail says.

use std::collections::BTreeMap;

use bb_core::access::{AppliesTo, FieldLevel};
use bb_core::fields::Field;
use bb_core::id::{IssueId, LabelId, ProjectId, RelationKindId};
use bb_core::issue::{Issue, Priority};
use bb_core::many_changes::SkipReason;
use bb_core::relation::Direction;
use bb_core::workflow::{StatusKey, starter};
use bb_policy::Permission;
use proptest::prelude::*;

use super::many_changes::{apply, done, preview, targets};
use super::scenarios::{a_project, token};
use super::scenarios_issues::an_issue;
use super::scenarios_tags::{a_label, labelled};
use super::world::World;
use crate::auth::AppError;
use crate::history::{ChangeValue, FieldChangeView};
use crate::issues::{ChangeIssue, DeleteIssue, IssueStore, change_issue, delete_issue};
use crate::many_changes::{
    ManyChange, ManyChangeAction, ManyChangeDetail, ManyChangePreview, ManyChangeResult,
    ManyChangeRow, ManyChangeTarget,
};
use crate::relations::{RelationKindsQuery, RelationStore, relation_kinds};
use crate::roles::{
    AssignRole, CreateRole, Given, SetFieldLevel, Subject, assign_role, create_role, set_level,
};
use crate::tags::TagStore;

/// One issue of the selection: where it is and how it stands.
#[derive(Clone, Copy, Debug)]
struct Seed {
    status: usize,
    in_secret_project: bool,
    has_label: bool,
    in_trash: bool,
}

/// The status a seed stands for: *Open*, *In progress* or *Done*.
fn status_of(index: usize) -> StatusKey {
    [starter::open(), starter::in_progress(), starter::done()][index].clone()
}

fn seed() -> impl Strategy<Value = Seed> {
    (0_usize..3, any::<bool>(), any::<bool>(), any::<bool>()).prop_map(
        |(status, in_secret_project, has_label, in_trash)| Seed {
            status,
            in_secret_project,
            has_label,
            in_trash,
        },
    )
}

/// What the world holds: the projects, the label, the kind of relation and the issue to relate to.
struct Setup {
    open: ProjectId,
    into: ProjectId,
    label: LabelId,
    kind: RelationKindId,
    anchor: IssueId,
    issues: Vec<IssueId>,
}

/// What an issue is like, to compare before and after.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Stands {
    version: i64,
    status: StatusKey,
    priority: Priority,
    project: ProjectId,
    in_trash: bool,
    labels: Vec<LabelId>,
    relations: usize,
}

fn broke(error: impl std::fmt::Display) -> TestCaseError {
    TestCaseError::fail(error.to_string())
}

async fn stands(world: &World, issue: IssueId) -> Result<Stands, TestCaseError> {
    let ctx = world.ctx();
    let store = ctx.store();
    let found: Issue = store
        .issue(issue)
        .await
        .map_err(broke)?
        .ok_or_else(|| broke("no issue"))?;
    let mut labels: Vec<LabelId> = store
        .issue_labels(issue)
        .await
        .map_err(broke)?
        .iter()
        .map(|link| link.label)
        .collect();
    labels.sort();
    let relations = store.relations_of(issue).await.map_err(broke)?.len();
    Ok(Stands {
        version: found.version,
        status: found.status,
        priority: found.priority,
        project: found.project,
        in_trash: found.deleted_at.is_some(),
        labels,
        relations,
    })
}

async fn set_up(world: &World, seeds: &[Seed]) -> Result<Setup, TestCaseError> {
    let admin = token(world);
    let (open, into, secret) = (
        a_project(world, "OPEN").await.map_err(broke)?,
        a_project(world, "INTO").await.map_err(broke)?,
        a_project(world, "SECRET").await.map_err(broke)?,
    );
    let label = a_label(world, "urgent", None).await.map_err(broke)?;
    let kinds = relation_kinds(
        &world.ctx(),
        RelationKindsQuery {
            session_token: admin.clone(),
            cursor: None,
            limit: 50,
        },
    )
    .await
    .map_err(broke)?
    .into_value();
    let kind = kinds.items.first().ok_or_else(|| broke("no kind"))?.id;
    let anchor = an_issue(world, open).await.map_err(broke)?;
    let mut issues = Vec::new();
    for seed in seeds {
        let project = if seed.in_secret_project { secret } else { open };
        let issue = an_issue(world, project).await.map_err(broke)?;
        if seed.status != 0 {
            change_issue(
                &world.ctx(),
                ChangeIssue {
                    session_token: admin.clone(),
                    issue: issue.to_string(),
                    version: 1,
                    issue_type: None,
                    title: None,
                    status: Some(status_of(seed.status)),
                    priority: None,
                    assigned_to: None,
                    fields: BTreeMap::new(),
                },
            )
            .await
            .map_err(broke)?;
        }
        if seed.has_label {
            labelled(world, issue, label).await.map_err(broke)?;
        }
        if seed.in_trash {
            delete_issue(
                &world.ctx(),
                DeleteIssue {
                    session_token: admin.clone(),
                    issue: issue.to_string(),
                },
            )
            .await
            .map_err(broke)?;
        }
        issues.push(issue);
    }
    Ok(Setup {
        open,
        into,
        label,
        kind,
        anchor,
        issues,
    })
}

/// The session of the person who changes the issues: 0 is the administrator, 1 a reader, 2 an
/// editor who may see the status but not change it, 3 an editor. The people see OPEN and INTO and
/// not SECRET.
async fn session_of(world: &World, who: usize, setup: &Setup) -> Result<String, TestCaseError> {
    if who == 0 {
        return Ok(token(world));
    }
    let admin = token(world);
    let mut permissions = vec![Permission::SeeProjects, Permission::SeeIssues];
    if who >= 2 {
        permissions.extend([
            Permission::ChangeIssues,
            Permission::DeleteIssues,
            Permission::MoveIssues,
            Permission::CreateIssues,
        ]);
    }
    let role = create_role(
        &world.ctx(),
        CreateRole {
            session_token: admin.clone(),
            name: "Somebody".to_owned(),
            permissions,
            two_step: false,
        },
    )
    .await
    .map_err(broke)?
    .into_value();
    if who == 2 {
        set_level(
            &world.ctx(),
            SetFieldLevel {
                session_token: admin.clone(),
                role: role.id,
                project: None,
                issue_type: None,
                field: Field::Status.into(),
                level: Some(FieldLevel::CanSee),
            },
        )
        .await
        .map_err(broke)?;
    }
    let bob = world.person("bob@example.test").await.map_err(broke)?;
    for project in [setup.open, setup.into] {
        assign_role(
            &world.ctx(),
            AssignRole {
                session_token: admin.clone(),
                subject: Subject::Person(bob.person.id),
                given: Given::Role(role.id),
                project: Some(project),
                applies_to: AppliesTo::ProjectAndSubProjects,
            },
        )
        .await
        .map_err(broke)?;
    }
    Ok(bob.session_token)
}

fn action_of(which: usize, setup: &Setup) -> ManyChangeAction {
    match which {
        0 => done(),
        1 => ManyChangeAction::Change {
            change: ManyChange {
                priority: Some(Priority::High),
                ..ManyChange::default()
            },
        },
        2 => ManyChangeAction::AddLabel { label: setup.label },
        3 => ManyChangeAction::Delete,
        4 => ManyChangeAction::Move {
            project: setup.into.to_string(),
        },
        _ => ManyChangeAction::AddRelation {
            relation_kind: setup.kind,
            direction: Direction::Forward,
            other: setup.anchor.to_string(),
        },
    }
}

fn text_of(value: Option<&ChangeValue>) -> Option<&str> {
    match value {
        Some(ChangeValue::Text { text }) => Some(text),
        _ => None,
    }
}

fn changed_field<'a>(row: &'a ManyChangeRow, field: &str) -> Option<&'a FieldChangeView> {
    match &row.detail {
        Some(ManyChangeDetail::Changes { fields, .. }) => {
            fields.iter().find(|each| each.field == field)
        }
        _ => None,
    }
}

/// What the preview's detail says for an issue it did not skip, against how the issue stands after
/// the change; and the issue as it was for one it did skip.
fn compare(
    action: &ManyChangeAction,
    setup: &Setup,
    row: &ManyChangeRow,
    before: &Stands,
    after: &Stands,
) -> Result<(), TestCaseError> {
    if row.skipped.is_some() {
        prop_assert_eq!(before, after, "a skipped issue is left as it was");
        return Ok(());
    }
    if !matches!(action, ManyChangeAction::AddRelation { .. }) {
        // A relation is its own record and does not count the version of the issue.
        prop_assert!(after.version > before.version, "a changed issue counts up");
    }
    match action {
        ManyChangeAction::Change { change } if change.status.is_some() => {
            let shown = changed_field(row, "status").ok_or_else(|| broke("no status in detail"))?;
            prop_assert_eq!(text_of(shown.after.as_ref()), Some(after.status.as_str()));
            prop_assert_eq!(Some(&after.status), change.status.as_ref());
        }
        ManyChangeAction::Change { change } => {
            let shown =
                changed_field(row, "priority").ok_or_else(|| broke("no priority in detail"))?;
            prop_assert_eq!(text_of(shown.after.as_ref()), Some(after.priority.code()));
            prop_assert_eq!(Some(after.priority), change.priority);
        }
        ManyChangeAction::AddLabel { label } => {
            prop_assert!(after.labels.contains(label));
            let shown = changed_field(row, "labels").ok_or_else(|| broke("no labels in detail"))?;
            let named = matches!(&shown.after, Some(ChangeValue::Names { names }) if names.iter().any(|name| name == "urgent"));
            prop_assert!(named);
        }
        ManyChangeAction::Delete => prop_assert!(after.in_trash),
        ManyChangeAction::Move { .. } => {
            prop_assert_eq!(after.project, setup.into);
            let shown =
                changed_field(row, "project").ok_or_else(|| broke("no project in detail"))?;
            let moved = matches!(&shown.after, Some(ChangeValue::Project { project }) if project.id == setup.into);
            prop_assert!(moved);
        }
        ManyChangeAction::AddRelation { .. } => {
            prop_assert_eq!(after.relations, before.relations + 1);
            let related = matches!(row.detail, Some(ManyChangeDetail::Relation { .. }));
            prop_assert!(related);
        }
    }
    Ok(())
}

async fn the_preview_is_what_happens(
    seeds: Vec<Seed>,
    missing: usize,
    who: usize,
    which: usize,
) -> Result<(), TestCaseError> {
    let world = World::new().await.map_err(broke)?;
    let setup = set_up(&world, &seeds).await?;
    let session = session_of(&world, who, &setup).await?;
    let action = action_of(which, &setup);
    let mut asked: Vec<IssueId> = setup.issues.clone();
    for number in 0..missing {
        asked.push(
            IssueId::from_parts(
                jiff::Timestamp::UNIX_EPOCH,
                [u8::try_from(number + 1).map_err(broke)?; 10],
            )
            .map_err(broke)?,
        );
    }
    let looked: Result<ManyChangePreview, AppError> =
        preview(&world, &session, &asked, &action).await;
    let looked = match looked {
        Ok(looked) => looked,
        Err(refusal) => {
            // A change the person may not even ask for is refused the same way when it is made.
            let wanted = asked
                .iter()
                .map(|issue| ManyChangeTarget {
                    issue: issue.to_string(),
                    version: 1,
                })
                .collect();
            let refused = apply(&world, &session, wanted, &action).await.err();
            prop_assert_eq!(refused, Some(refusal));
            return Ok(());
        }
    };
    let mut before = Vec::new();
    for issue in &setup.issues {
        before.push(stands(&world, *issue).await?);
    }
    let wanted = targets(&looked);
    if wanted.is_empty() {
        // Nothing the person can read: there is nothing to send, and the preview said so.
        prop_assert!(looked.rows.iter().all(|row| row.skipped.is_some()));
        return Ok(());
    }
    let result: ManyChangeResult = apply(&world, &session, wanted, &action)
        .await
        .map_err(broke)?;
    // The issues the preview could read were sent, in the order asked; those that are not there
    // for this person have no version and were not.
    let sent: Vec<&ManyChangeRow> = looked
        .rows
        .iter()
        .filter(|row| row.version.is_some())
        .collect();
    prop_assert_eq!(sent.len(), result.outcomes.len());
    for (row, outcome) in sent.iter().zip(&result.outcomes) {
        prop_assert_eq!(&row.address, &outcome.address);
        prop_assert_eq!(row.skipped, outcome.skipped);
    }
    for (index, issue) in setup.issues.iter().enumerate() {
        let row = &looked.rows[index];
        let after = stands(&world, *issue).await?;
        if row.version.is_none() {
            prop_assert_eq!(row.skipped, Some(SkipReason::NotFound));
            prop_assert_eq!(&before[index], &after, "an issue not there is not touched");
        } else {
            compare(&action, &setup, row, &before[index], &after)?;
        }
    }
    for row in &looked.rows[setup.issues.len()..] {
        prop_assert_eq!(row.skipped, Some(SkipReason::NotFound));
    }
    // The summary counts what the rows say.
    let skipped = looked
        .rows
        .iter()
        .filter(|row| row.skipped.is_some())
        .count();
    prop_assert_eq!(
        (looked.summary.will_change, looked.summary.skipped),
        (
            u32::try_from(looked.rows.len() - skipped).map_err(broke)?,
            u32::try_from(skipped).map_err(broke)?
        )
    );
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    /// The preview says, for each issue, what the change then does to it.
    #[test]
    fn iss_09_the_preview_equals_what_happens(
        seeds in proptest::collection::vec(seed(), 1..7),
        missing in 0_usize..3,
        who in 0_usize..4,
        which in 0_usize..6,
    ) {
        let runtime = tokio::runtime::Builder::new_current_thread().build().unwrap();
        runtime.block_on(the_preview_is_what_happens(seeds, missing, who, which))?;
    }
}

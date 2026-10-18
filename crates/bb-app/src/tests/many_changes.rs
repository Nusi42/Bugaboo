//! Changing many issues at once against the in-memory store (ISS-09): the preview is what then
//! happens, an issue the person cannot change is skipped with its reason, an issue that changed
//! after the preview is named and left alone, a move skips what the other project cannot take, and
//! more than a thousand issues become a job that goes on step by step.

use std::collections::BTreeMap;

use bb_core::access::AppliesTo;
use bb_core::id::IssueId;
use bb_core::many_changes::{MANY_CHANGES_JOB_ABOVE, ManyChangeStatus, SkipReason};
use bb_core::workflow::starter;
use bb_policy::Permission;

use super::scenarios::{a_project, token};
use super::scenarios_issues::{a_type, an_issue};
use super::scenarios_tags::{a_label, a_label_group, labelled, tagged};
use super::world::{Outcome, World};
use crate::issues::{
    ChangeIssue, CreateIssue, IssueTypesQuery, change_issue, create_issue, issue_types,
};
use crate::many_changes::{
    ApplyManyChanges, ManyChange, ManyChangeAction, ManyChangeDetail, ManyChangeNote,
    ManyChangePreview, ManyChangePreviewQuery, ManyChangeResult, ManyChangeRunQuery,
    ManyChangeTarget, apply_many_changes, many_changes_run, preview_many_changes,
};
use crate::roles::{AssignRole, CreateRole, Given, Subject, assign_role, create_role};

pub(super) fn done() -> ManyChangeAction {
    ManyChangeAction::Change {
        change: ManyChange {
            status: Some(starter::done()),
            ..ManyChange::default()
        },
    }
}

pub(super) async fn preview(
    world: &World,
    session: &str,
    issues: &[IssueId],
    action: &ManyChangeAction,
) -> Result<ManyChangePreview, crate::auth::AppError> {
    Ok(preview_many_changes(
        &world.ctx(),
        ManyChangePreviewQuery {
            session_token: session.to_owned(),
            issues: issues.iter().map(ToString::to_string).collect(),
            action: action.clone(),
        },
    )
    .await?
    .into_value())
}

/// What the preview read, as the targets the change takes back.
pub(super) fn targets(preview: &ManyChangePreview) -> Vec<ManyChangeTarget> {
    preview
        .rows
        .iter()
        .filter_map(|row| {
            row.version.map(|version| ManyChangeTarget {
                issue: row.address.clone(),
                version,
            })
        })
        .collect()
}

pub(super) async fn apply(
    world: &World,
    session: &str,
    targets: Vec<ManyChangeTarget>,
    action: &ManyChangeAction,
) -> Result<ManyChangeResult, crate::auth::AppError> {
    Ok(apply_many_changes(
        &world.ctx(),
        ApplyManyChanges {
            session_token: session.to_owned(),
            issues: targets,
            action: action.clone(),
        },
    )
    .await?
    .into_value())
}

#[tokio::test]
async fn iss_09_an_issue_you_cannot_change_is_skipped_with_its_reason() -> Outcome {
    let world = World::new().await?;
    let admin = token(&world);
    let (open, closed_off) = (
        a_project(&world, "OPEN").await?,
        a_project(&world, "SHUT").await?,
    );
    let (mine, theirs, hidden) = (
        an_issue(&world, open).await?,
        an_issue(&world, open).await?,
        an_issue(&world, closed_off).await?,
    );
    // Bob sees the issues of OPEN and may not change any of them; he does not know SHUT.
    let reader = create_role(
        &world.ctx(),
        CreateRole {
            session_token: admin.clone(),
            name: "Reader".to_owned(),
            permissions: vec![Permission::SeeProjects, Permission::SeeIssues],
            two_step: false,
        },
    )
    .await?
    .into_value();
    let bob = world.person("bob@example.test").await?;
    assign_role(
        &world.ctx(),
        AssignRole {
            session_token: admin,
            subject: Subject::Person(bob.person.id),
            given: Given::Role(reader.id),
            project: Some(open),
            applies_to: AppliesTo::ProjectAndSubProjects,
        },
    )
    .await?;
    let asked = preview(&world, &bob.session_token, &[mine, theirs, hidden], &done()).await?;
    let reasons: Vec<Option<SkipReason>> = asked.rows.iter().map(|row| row.skipped).collect();
    assert_eq!(
        reasons,
        vec![
            Some(SkipReason::NotAllowed),
            Some(SkipReason::NotAllowed),
            Some(SkipReason::NotFound)
        ]
    );
    assert_eq!((asked.summary.will_change, asked.summary.skipped), (0, 3));
    // The hidden issue is not named at all.
    assert!(asked.rows[2].issue.is_none() && asked.rows[2].version.is_none());
    // Applying says the same, and changes nothing.
    let before = world.store.event_count();
    let result = apply(
        &world,
        &bob.session_token,
        vec![ManyChangeTarget {
            issue: mine.to_string(),
            version: 1,
        }],
        &done(),
    )
    .await?;
    assert_eq!(result.outcomes[0].skipped, Some(SkipReason::NotAllowed));
    assert_eq!(world.store.event_count(), before);
    Ok(())
}

#[tokio::test]
async fn iss_09_an_issue_changed_after_the_preview_is_left_alone_and_named() -> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "ACME").await?;
    let (first, second) = (
        an_issue(&world, project).await?,
        an_issue(&world, project).await?,
    );
    let admin = token(&world);
    let asked = preview(&world, &admin, &[first, second], &done()).await?;
    assert_eq!(asked.summary.will_change, 2);
    // Somebody changes the first issue between the preview and the change.
    change_issue(
        &world.ctx(),
        ChangeIssue {
            session_token: admin.clone(),
            issue: first.to_string(),
            version: 1,
            issue_type: None,
            title: Some("Changed meanwhile".to_owned()),
            status: None,
            priority: None,
            assigned_to: None,
            fields: BTreeMap::new(),
        },
    )
    .await?;
    let result = apply(&world, &admin, targets(&asked), &done()).await?;
    let skipped: Vec<Option<SkipReason>> =
        result.outcomes.iter().map(|each| each.skipped).collect();
    assert_eq!(skipped, vec![Some(SkipReason::ChangedSince), None]);
    // The named issue is left as the other person made it.
    let left = result.outcomes[0].issue.as_ref().ok_or("no line")?;
    assert_eq!(
        left.status.as_ref().map(|status| status.key.clone()),
        Some(starter::open())
    );
    assert_eq!(
        result.outcomes[1]
            .issue
            .as_ref()
            .and_then(|line| line.status.as_ref())
            .map(|status| status.key.clone()),
        Some(starter::open())
    );
    Ok(())
}

#[tokio::test]
async fn iss_02_a_move_keeps_a_retired_type_and_names_it_in_memory() -> Outcome {
    let world = World::new().await?;
    let admin = token(&world);
    let (from, to) = (
        a_project(&world, "FROM").await?,
        a_project(&world, "INTO").await?,
    );
    let types = issue_types(
        &world.ctx(),
        IssueTypesQuery {
            project: None,
            session_token: admin.clone(),
            cursor: None,
            limit: 50,
        },
    )
    .await?
    .into_value();
    let (kept, retired) = (types.items[0].id, types.items[1].id);
    let mut made = Vec::new();
    for issue_type in [kept, retired] {
        made.push(
            create_issue(
                &world.ctx(),
                CreateIssue {
                    session_token: admin.clone(),
                    project: from.to_string(),
                    issue_type,
                    title: "An issue".to_owned(),
                    description: None,
                    status: None,
                    priority: None,
                    assigned_to: None,
                    fields: BTreeMap::new(),
                },
            )
            .await?
            .into_value()
            .id,
        );
    }
    // The second type is retired: no project offers it to new issues, and the issue keeps it.
    crate::issues::change_issue_type(
        &world.ctx(),
        crate::issues::ChangeIssueType {
            session_token: admin.clone(),
            issue_type: retired,
            version: 1,
            name: None,
            icon: None,
            retired: Some(true),
        },
    )
    .await?;
    let action = ManyChangeAction::Move {
        project: to.to_string(),
    };
    let asked = preview(&world, &admin, &made, &action).await?;
    // Neither is skipped: a retired type is not missing. The preview says that INTO does not use it.
    let reasons: Vec<Option<SkipReason>> = asked.rows.iter().map(|row| row.skipped).collect();
    assert_eq!(reasons, vec![None, None]);
    let notes: Vec<Vec<ManyChangeNote>> = asked
        .rows
        .iter()
        .map(|row| match &row.detail {
            Some(ManyChangeDetail::Changes { notes, .. }) => notes.clone(),
            _ => Vec::new(),
        })
        .collect();
    assert_eq!(
        notes,
        vec![
            Vec::new(),
            vec![ManyChangeNote::TypeNotUsed {
                issue_type: types.items[1].name.clone(),
                project: "A project".to_owned(),
            }],
        ]
    );
    let result = apply(&world, &admin, targets(&asked), &action).await?;
    assert!(result.outcomes.iter().all(|each| each.skipped.is_none()));
    // The moved issue keeps its retired type.
    let kept = crate::issues::issue(
        &world.ctx(),
        crate::issues::IssueQuery {
            session_token: admin.clone(),
            issue: made[1].to_string(),
        },
    )
    .await?
    .into_value();
    assert_eq!(kept.issue.issue_type.id, retired);
    Ok(())
}

#[tokio::test]
async fn iss_09_a_move_skips_an_issue_with_a_tag_the_other_project_does_not_allow() -> Outcome {
    let world = World::new().await?;
    let admin = token(&world);
    let (from, to) = (
        a_project(&world, "FROM").await?,
        a_project(&world, "INTO").await?,
    );
    let (limited, free) = (an_issue(&world, from).await?, an_issue(&world, from).await?);
    let tag = tagged(&world, limited, "billing").await?;
    crate::tags::change_tag(
        &world.ctx(),
        crate::tags::ChangeTag {
            session_token: admin.clone(),
            tag,
            version: 1,
            name: None,
            color: None,
            projects: Some(vec![from]),
        },
    )
    .await?;
    let action = ManyChangeAction::Move {
        project: to.to_string(),
    };
    let asked = preview(&world, &admin, &[limited, free], &action).await?;
    let reasons: Vec<Option<SkipReason>> = asked.rows.iter().map(|row| row.skipped).collect();
    assert_eq!(reasons, vec![Some(SkipReason::TagNotHere), None]);
    let result = apply(&world, &admin, targets(&asked), &action).await?;
    assert_eq!(result.outcomes[0].skipped, Some(SkipReason::TagNotHere));
    Ok(())
}

#[tokio::test]
async fn iss_09_adding_a_label_names_the_issues_that_have_it_already() -> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "ACME").await?;
    let (has_it, lacks_it) = (
        an_issue(&world, project).await?,
        an_issue(&world, project).await?,
    );
    let label = a_label(&world, "urgent", None).await?;
    labelled(&world, has_it, label).await?;
    let action = ManyChangeAction::AddLabel { label };
    let asked = preview(&world, &token(&world), &[has_it, lacks_it], &action).await?;
    let reasons: Vec<Option<SkipReason>> = asked.rows.iter().map(|row| row.skipped).collect();
    assert_eq!(reasons, vec![Some(SkipReason::NothingToChange), None]);
    Ok(())
}

#[tokio::test]
async fn iss_09_a_label_put_on_after_the_preview_makes_the_change_skip_and_name_the_issue()
-> Outcome {
    let world = World::new().await?;
    let admin = token(&world);
    let project = a_project(&world, "ACME").await?;
    let (first, second) = (
        an_issue(&world, project).await?,
        an_issue(&world, project).await?,
    );
    let group = a_label_group(&world, "Severity", true).await?;
    let (low, high, urgent) = (
        a_label(&world, "low", Some(group)).await?,
        a_label(&world, "high", Some(group)).await?,
        a_label(&world, "urgent", Some(group)).await?,
    );
    labelled(&world, first, low).await?;
    let action = ManyChangeAction::AddLabel { label: high };
    let asked = preview(&world, &admin, &[first, second], &action).await?;
    assert_eq!(asked.summary.will_change, 2);
    // Somebody takes another label of the Pick one group meanwhile: apply would replace it, not
    // the one the preview named.
    labelled(&world, first, urgent).await?;
    let result = apply(&world, &admin, targets(&asked), &action).await?;
    let skipped: Vec<Option<SkipReason>> =
        result.outcomes.iter().map(|each| each.skipped).collect();
    assert_eq!(skipped, vec![Some(SkipReason::ChangedSince), None]);
    assert!(result.outcomes[0].issue.is_some(), "the issue is named");
    Ok(())
}

#[tokio::test]
async fn iss_09_more_than_a_thousand_run_as_a_job() -> Outcome {
    let world = World::new().await?;
    let admin = token(&world);
    let project = a_project(&world, "ACME").await?;
    let kind = a_type(&world).await?;
    let mut issues = Vec::new();
    for number in 0..=MANY_CHANGES_JOB_ABOVE {
        issues.push(
            create_issue(
                &world.ctx(),
                CreateIssue {
                    session_token: admin.clone(),
                    project: project.to_string(),
                    issue_type: kind,
                    title: format!("Issue {number}"),
                    description: None,
                    status: None,
                    priority: None,
                    assigned_to: None,
                    fields: BTreeMap::new(),
                },
            )
            .await?
            .into_value()
            .id,
        );
    }
    assert_eq!(issues.len(), MANY_CHANGES_JOB_ABOVE + 1);
    let asked = preview(&world, &admin, &issues, &done()).await?;
    assert!(asked.runs_in_background);
    let events_before = world.store.event_count();
    let result = apply(&world, &admin, targets(&asked), &done()).await?;
    let started = result.run.ok_or("a big change must start a run")?;
    assert_eq!(
        (started.status, started.done),
        (ManyChangeStatus::Queued, 0)
    );
    // Only the run is written before a worker takes the first step.
    assert_eq!(world.store.event_count(), events_before + 1);
    let mut rounds = 0;
    loop {
        let taken = world.ctx().run_due_jobs().await?;
        if taken == 0 {
            break;
        }
        rounds += 1;
        assert!(rounds < 100, "the run does not end");
    }
    let finished = many_changes_run(
        &world.ctx(),
        ManyChangeRunQuery {
            session_token: admin,
            run: started.id,
        },
    )
    .await?
    .into_value();
    assert_eq!(finished.status, ManyChangeStatus::Done);
    assert_eq!((finished.done, finished.changed), (1001, 1001));
    assert_eq!(finished.skipped_count, 0);
    // One event for each issue, the one that makes the run, and the one that says it is done (the
    // person who asked is told by it, M3-F125).
    assert_eq!(world.store.event_count(), events_before + 1 + 1001 + 1);
    // Steps of a hundred: eleven of them.
    assert_eq!(rounds, 11);
    Ok(())
}

#[tokio::test]
async fn adr_0017_a_step_that_stopped_before_it_counted_an_issue_counts_it_when_it_goes_on()
-> Outcome {
    use bb_core::id::ManyChangeRunId;
    use bb_core::many_changes::{ManyChangeItem, ManyChangeRun};

    let world = World::new().await?;
    let project = a_project(&world, "ACME").await?;
    let mut issues = Vec::new();
    for _ in 0..3 {
        issues.push(an_issue(&world, project).await?);
    }
    let run = ManyChangeRun {
        id: ManyChangeRunId::generate(&mut bb_core::id::SequentialIds::default())?,
        person: world.admin.person.id,
        action: serde_json::to_value(done())?,
        items: issues
            .iter()
            .map(|issue| ManyChangeItem {
                issue: *issue,
                version: 1,
            })
            .collect(),
        status: ManyChangeStatus::Queued,
        done: 0,
        changed: 0,
        skipped: Vec::new(),
        created_at: world.ctx().now(),
        finished_at: None,
    };
    world.store.lock().state.many_changes_runs.push(run.clone());
    let payload = crate::many_changes::step_payload(run.id);
    world.ctx().run_many_changes_step(&payload).await?;
    let finished = world.store.lock().state.many_changes_runs[0].clone();
    assert_eq!(
        (finished.status, finished.changed),
        (ManyChangeStatus::Done, 3)
    );
    // The worker died after it changed the third issue and before it counted it: the run is
    // back at two issues counted, and the issue is changed.
    {
        let mut shared = world.store.lock();
        let stopped = &mut shared.state.many_changes_runs[0];
        stopped.status = ManyChangeStatus::Running;
        stopped.done = 2;
        stopped.changed = 2;
        stopped.finished_at = None;
    }
    world.ctx().run_many_changes_step(&payload).await?;
    let again = world.store.lock().state.many_changes_runs[0].clone();
    assert_eq!(again.status, ManyChangeStatus::Done);
    assert_eq!(
        (again.done, again.changed),
        (3, 3),
        "the issue counts as changed"
    );
    assert!(
        again.skipped.is_empty(),
        "it is not reported as changed since the preview"
    );
    Ok(())
}

#[tokio::test]
async fn iss_09_a_run_is_read_only_by_the_person_who_asked_for_it() -> Outcome {
    let world = World::new().await?;
    let admin = token(&world);
    let project = a_project(&world, "ACME").await?;
    let kind = a_type(&world).await?;
    let mut targets = Vec::new();
    for number in 0..=MANY_CHANGES_JOB_ABOVE {
        let made = create_issue(
            &world.ctx(),
            CreateIssue {
                session_token: admin.clone(),
                project: project.to_string(),
                issue_type: kind,
                title: format!("Issue {number}"),
                description: None,
                status: None,
                priority: None,
                assigned_to: None,
                fields: BTreeMap::new(),
            },
        )
        .await?
        .into_value();
        targets.push(ManyChangeTarget {
            issue: made.id.to_string(),
            version: made.version,
        });
    }
    let result = apply(&world, &admin, targets, &done()).await?;
    let run = result.run.ok_or("no run")?;
    let bob = world.person("bob@example.test").await?;
    let read = many_changes_run(
        &world.ctx(),
        ManyChangeRunQuery {
            session_token: bob.session_token,
            run: run.id,
        },
    )
    .await
    .err();
    assert_eq!(
        read,
        Some(crate::auth::AppError::Refused(bb_core::ErrorCode::NotFound))
    );
    Ok(())
}

#[tokio::test]
async fn iss_09_a_selection_of_nothing_or_of_too_many_is_refused() -> Outcome {
    let world = World::new().await?;
    let admin = token(&world);
    let none = preview_many_changes(
        &world.ctx(),
        ManyChangePreviewQuery {
            session_token: admin.clone(),
            issues: Vec::new(),
            action: done(),
        },
    )
    .await
    .err();
    assert_eq!(
        none,
        Some(crate::auth::AppError::Refused(
            bb_core::ErrorCode::InvalidRequest
        ))
    );
    let nothing_asked = preview_many_changes(
        &world.ctx(),
        ManyChangePreviewQuery {
            session_token: admin,
            issues: vec!["x".to_owned()],
            action: ManyChangeAction::Change {
                change: ManyChange::default(),
            },
        },
    )
    .await
    .err();
    assert_eq!(
        nothing_asked,
        Some(crate::auth::AppError::Refused(
            bb_core::ErrorCode::InvalidRequest
        ))
    );
    Ok(())
}

/// A run of three issues that waits for its first step, with that step in the queue.
pub(super) async fn run_with_a_step_waiting(
    world: &World,
) -> Result<bb_core::many_changes::ManyChangeRun, Box<dyn std::error::Error + Send + Sync>> {
    use bb_core::id::{JobId, ManyChangeRunId};
    use bb_core::many_changes::{ManyChangeItem, ManyChangeRun};

    use crate::jobs::{JobKind, JobStore as _};

    let project = a_project(world, "ACME").await?;
    let mut items = Vec::new();
    for _ in 0..3 {
        items.push(ManyChangeItem {
            issue: an_issue(world, project).await?,
            version: 1,
        });
    }
    let now = world.ctx().now();
    let run = ManyChangeRun {
        id: ManyChangeRunId::generate(&mut bb_core::id::SequentialIds::default())?,
        person: world.admin.person.id,
        action: serde_json::to_value(done())?,
        items,
        status: ManyChangeStatus::Queued,
        done: 0,
        changed: 0,
        skipped: Vec::new(),
        created_at: now,
        finished_at: None,
    };
    world.store.lock().state.many_changes_runs.push(run.clone());
    world
        .store
        .queue_job(
            JobId::generate(&mut bb_core::id::SequentialIds::default())?,
            &JobKind::ManyChangesStep.name()?,
            &crate::many_changes::step_payload(run.id),
            now,
            now,
        )
        .await
        .map_err(|error| format!("{error:?}"))?;
    Ok(run)
}

#[tokio::test]
async fn adr_0017_a_step_that_keeps_failing_ends_its_run_as_failed() -> Outcome {
    use bb_core::job::JobState;

    let world = World::new().await?;
    let run = run_with_a_step_waiting(&world).await?;
    // The store cannot save the progress for every start of the step: the job goes through all
    // its starts and lands on the dead-letter list, and the run says so instead of staying
    // "running". The saving that ends the run works again.
    world
        .store
        .fail_many_changes_progress(usize::try_from(crate::jobs::JOB_ATTEMPTS_MAX)?);
    for _ in 0..crate::jobs::JOB_ATTEMPTS_MAX {
        // A start that failed waits before the next one, as the store makes it.
        world.clock.advance(60);
        world.ctx().run_due_jobs().await?;
    }
    let shared = world.store.lock();
    assert_eq!(shared.state.jobs[0].state, JobState::Dead);
    let ended = &shared.state.many_changes_runs[0];
    assert_eq!(ended.id, run.id);
    assert_eq!(ended.status, ManyChangeStatus::Failed);
    assert!(ended.finished_at.is_some());
    Ok(())
}

#[tokio::test]
async fn adr_0017_a_step_that_is_refused_is_not_started_again() -> Outcome {
    use bb_core::id::JobId;
    use bb_core::job::JobState;

    use crate::jobs::{JobKind, JobStore as _};

    let world = World::new().await?;
    let now = world.ctx().now();
    // A step whose payload names no run can never work, whatever the store does.
    world
        .store
        .queue_job(
            JobId::generate(&mut bb_core::id::SequentialIds::default())?,
            &JobKind::ManyChangesStep.name()?,
            &serde_json::json!({}),
            now,
            now,
        )
        .await
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(world.ctx().run_due_jobs().await?, 1);
    let shared = world.store.lock();
    assert_eq!(shared.state.jobs[0].state, JobState::Dead);
    assert_eq!(shared.state.jobs[0].attempts, 1, "no second start");
    Ok(())
}

#[tokio::test]
async fn iss_09_a_stopped_run_changes_nothing_more() -> Outcome {
    let world = World::new().await?;
    let run = run_with_a_step_waiting(&world).await?;
    world.store.lock().state.many_changes_runs[0].status = ManyChangeStatus::Stopped;
    world
        .ctx()
        .run_many_changes_step(&crate::many_changes::step_payload(run.id))
        .await?;
    let after = world.store.lock().state.many_changes_runs[0].clone();
    assert_eq!(
        (after.status, after.done, after.changed),
        (ManyChangeStatus::Stopped, 0, 0)
    );
    Ok(())
}

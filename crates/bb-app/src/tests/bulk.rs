//! Changing many issues at once against the in-memory store (ISS-09): the preview is what then
//! happens, an issue the person cannot change is skipped with its reason, an issue that changed
//! after the preview is named and left alone, a move skips what the other project cannot take, and
//! more than a thousand issues become a job that goes on step by step.

use std::collections::BTreeMap;

use bb_core::access::AppliesTo;
use bb_core::bulk::{BULK_JOB_ABOVE, BulkStatus, SkipReason};
use bb_core::id::IssueId;
use bb_core::issue::Status;
use bb_policy::Permission;

use super::scenarios::{a_project, token};
use super::scenarios_issues::an_issue;
use super::scenarios_tags::{a_label, a_label_group, labelled, tagged};
use super::world::{Outcome, World};
use crate::bulk::{
    ApplyBulk, BulkAction, BulkChange, BulkPreview, BulkPreviewQuery, BulkResult, BulkRunQuery,
    BulkTarget, apply_bulk, bulk_run, preview_bulk,
};
use crate::issues::{
    ChangeIssue, CreateIssue, IssueTypesQuery, change_issue, create_issue, issue_types,
};
use crate::roles::{AssignRole, CreateRole, Given, Subject, assign_role, create_role};

pub(super) fn done() -> BulkAction {
    BulkAction::Change {
        change: BulkChange {
            status: Some(Status::Done),
            ..BulkChange::default()
        },
    }
}

pub(super) async fn preview(
    world: &World,
    session: &str,
    issues: &[IssueId],
    action: &BulkAction,
) -> Result<BulkPreview, crate::auth::AppError> {
    Ok(preview_bulk(
        &world.ctx(),
        BulkPreviewQuery {
            session_token: session.to_owned(),
            issues: issues.iter().map(ToString::to_string).collect(),
            action: action.clone(),
        },
    )
    .await?
    .into_value())
}

/// What the preview read, as the targets the change takes back.
pub(super) fn targets(preview: &BulkPreview) -> Vec<BulkTarget> {
    preview
        .rows
        .iter()
        .filter_map(|row| {
            row.version.map(|version| BulkTarget {
                issue: row.address.clone(),
                version,
            })
        })
        .collect()
}

pub(super) async fn apply(
    world: &World,
    session: &str,
    targets: Vec<BulkTarget>,
    action: &BulkAction,
) -> Result<BulkResult, crate::auth::AppError> {
    Ok(apply_bulk(
        &world.ctx(),
        ApplyBulk {
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
        vec![BulkTarget {
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
    assert_eq!(left.status, Some(Status::Open));
    assert_eq!(
        result.outcomes[1].issue.as_ref().map(|line| line.status),
        Some(Some(Status::Open))
    );
    Ok(())
}

#[tokio::test]
async fn iss_09_a_move_skips_issues_whose_type_the_target_lacks() -> Outcome {
    let world = World::new().await?;
    let admin = token(&world);
    let (from, to) = (
        a_project(&world, "FROM").await?,
        a_project(&world, "INTO").await?,
    );
    let types = issue_types(
        &world.ctx(),
        IssueTypesQuery {
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
    // The second type is retired: no project offers it.
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
    let action = BulkAction::Move {
        project: to.to_string(),
    };
    let asked = preview(&world, &admin, &made, &action).await?;
    let reasons: Vec<Option<SkipReason>> = asked.rows.iter().map(|row| row.skipped).collect();
    assert_eq!(reasons, vec![None, Some(SkipReason::TypeMissing)]);
    let result = apply(&world, &admin, targets(&asked), &action).await?;
    let moved: Vec<Option<String>> = result
        .outcomes
        .iter()
        .map(|each| each.skipped.map(|reason| reason.code().to_owned()))
        .collect();
    assert_eq!(moved, vec![None, Some("type-missing".to_owned())]);
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
    let action = BulkAction::Move {
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
    let action = BulkAction::AddLabel { label };
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
    let action = BulkAction::AddLabel { label: high };
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
    let kind = types_first(&world).await?;
    let mut issues = Vec::new();
    for number in 0..=BULK_JOB_ABOVE {
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
    assert_eq!(issues.len(), BULK_JOB_ABOVE + 1);
    let asked = preview(&world, &admin, &issues, &done()).await?;
    assert!(asked.runs_in_background);
    let events_before = world.store.event_count();
    let result = apply(&world, &admin, targets(&asked), &done()).await?;
    let started = result.run.ok_or("a big change must start a run")?;
    assert_eq!((started.status, started.done), (BulkStatus::Queued, 0));
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
    let finished = bulk_run(
        &world.ctx(),
        BulkRunQuery {
            session_token: admin,
            run: started.id,
        },
    )
    .await?
    .into_value();
    assert_eq!(finished.status, BulkStatus::Done);
    assert_eq!((finished.done, finished.changed), (1001, 1001));
    assert_eq!(finished.skipped_count, 0);
    // One event for each issue, and the one of the run.
    assert_eq!(world.store.event_count(), events_before + 1 + 1001);
    // Steps of a hundred: eleven of them.
    assert_eq!(rounds, 11);
    Ok(())
}

#[tokio::test]
async fn adr_0017_a_step_that_stopped_before_it_counted_an_issue_counts_it_when_it_goes_on()
-> Outcome {
    use bb_core::bulk::{BulkItem, BulkRun};
    use bb_core::id::BulkRunId;

    let world = World::new().await?;
    let project = a_project(&world, "ACME").await?;
    let mut issues = Vec::new();
    for _ in 0..3 {
        issues.push(an_issue(&world, project).await?);
    }
    let run = BulkRun {
        id: BulkRunId::generate(&mut bb_core::id::SequentialIds::default())?,
        person: world.admin.person.id,
        action: serde_json::to_value(done())?,
        items: issues
            .iter()
            .map(|issue| BulkItem {
                issue: *issue,
                version: 1,
            })
            .collect(),
        status: BulkStatus::Queued,
        done: 0,
        changed: 0,
        skipped: Vec::new(),
        created_at: world.ctx().now(),
        finished_at: None,
    };
    world.store.lock().state.bulk_runs.push(run.clone());
    let payload = crate::bulk::step_payload(run.id);
    world.ctx().run_bulk_step(&payload).await?;
    let finished = world.store.lock().state.bulk_runs[0].clone();
    assert_eq!((finished.status, finished.changed), (BulkStatus::Done, 3));
    // The worker died after it changed the third issue and before it counted it: the run is
    // back at two issues counted, and the issue is changed.
    {
        let mut shared = world.store.lock();
        let stopped = &mut shared.state.bulk_runs[0];
        stopped.status = BulkStatus::Running;
        stopped.done = 2;
        stopped.changed = 2;
        stopped.finished_at = None;
    }
    world.ctx().run_bulk_step(&payload).await?;
    let again = world.store.lock().state.bulk_runs[0].clone();
    assert_eq!(again.status, BulkStatus::Done);
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

async fn types_first(world: &World) -> Result<bb_core::id::IssueTypeId, String> {
    let types = issue_types(
        &world.ctx(),
        IssueTypesQuery {
            session_token: token(world),
            cursor: None,
            limit: 50,
        },
    )
    .await
    .map_err(|error| error.to_string())?
    .into_value();
    types
        .items
        .first()
        .map(|found| found.id)
        .ok_or_else(|| "no issue type".to_owned())
}

#[tokio::test]
async fn iss_09_a_run_is_read_only_by_the_person_who_asked_for_it() -> Outcome {
    let world = World::new().await?;
    let admin = token(&world);
    let project = a_project(&world, "ACME").await?;
    let kind = types_first(&world).await?;
    let mut targets = Vec::new();
    for number in 0..=BULK_JOB_ABOVE {
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
        targets.push(BulkTarget {
            issue: made.id.to_string(),
            version: made.version,
        });
    }
    let result = apply(&world, &admin, targets, &done()).await?;
    let run = result.run.ok_or("no run")?;
    let bob = world.person("bob@example.test").await?;
    let read = bulk_run(
        &world.ctx(),
        BulkRunQuery {
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
    let none = preview_bulk(
        &world.ctx(),
        BulkPreviewQuery {
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
    let nothing_asked = preview_bulk(
        &world.ctx(),
        BulkPreviewQuery {
            session_token: admin,
            issues: vec!["x".to_owned()],
            action: BulkAction::Change {
                change: BulkChange::default(),
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
async fn run_with_a_step_waiting(
    world: &World,
) -> Result<bb_core::bulk::BulkRun, Box<dyn std::error::Error + Send + Sync>> {
    use bb_core::bulk::{BulkItem, BulkRun};
    use bb_core::id::{BulkRunId, JobId};

    use crate::jobs::{JobStore as _, kind, kind_name};

    let project = a_project(world, "ACME").await?;
    let mut items = Vec::new();
    for _ in 0..3 {
        items.push(BulkItem {
            issue: an_issue(world, project).await?,
            version: 1,
        });
    }
    let now = world.ctx().now();
    let run = BulkRun {
        id: BulkRunId::generate(&mut bb_core::id::SequentialIds::default())?,
        person: world.admin.person.id,
        action: serde_json::to_value(done())?,
        items,
        status: BulkStatus::Queued,
        done: 0,
        changed: 0,
        skipped: Vec::new(),
        created_at: now,
        finished_at: None,
    };
    world.store.lock().state.bulk_runs.push(run.clone());
    world
        .store
        .queue_job(
            JobId::generate(&mut bb_core::id::SequentialIds::default())?,
            &kind_name(kind::BULK_STEP)?,
            &crate::bulk::step_payload(run.id),
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
        .fail_bulk_progress(usize::try_from(crate::jobs::JOB_ATTEMPTS_MAX)?);
    for _ in 0..crate::jobs::JOB_ATTEMPTS_MAX {
        world.ctx().run_due_jobs().await?;
    }
    let shared = world.store.lock();
    assert_eq!(shared.state.jobs[0].state, JobState::Dead);
    let ended = &shared.state.bulk_runs[0];
    assert_eq!(ended.id, run.id);
    assert_eq!(ended.status, BulkStatus::Failed);
    assert!(ended.finished_at.is_some());
    Ok(())
}

#[tokio::test]
async fn adr_0017_a_step_that_is_refused_is_not_started_again() -> Outcome {
    use bb_core::id::JobId;
    use bb_core::job::JobState;

    use crate::jobs::{JobStore as _, kind, kind_name};

    let world = World::new().await?;
    let now = world.ctx().now();
    // A step whose payload names no run can never work, whatever the store does.
    world
        .store
        .queue_job(
            JobId::generate(&mut bb_core::id::SequentialIds::default())?,
            &kind_name(kind::BULK_STEP)?,
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
    world.store.lock().state.bulk_runs[0].status = BulkStatus::Stopped;
    world
        .ctx()
        .run_bulk_step(&crate::bulk::step_payload(run.id))
        .await?;
    let after = world.store.lock().state.bulk_runs[0].clone();
    assert_eq!(
        (after.status, after.done, after.changed),
        (BulkStatus::Stopped, 0, 0)
    );
    Ok(())
}

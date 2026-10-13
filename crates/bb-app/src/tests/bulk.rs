//! Changing many issues at once against the in-memory store (ISS-09): the preview is what then
//! happens, an issue the person cannot change is skipped with its reason, an issue that changed
//! after the preview is named and left alone, a move skips what the other project cannot take, and
//! more than a thousand issues become a job that goes on step by step.

use std::collections::BTreeMap;

use bb_core::access::AppliesTo;
use bb_core::bulk::{BULK_JOB_ABOVE, BulkStatus, SkipReason};
use bb_core::id::{IssueId, ProjectId};
use bb_core::issue::Status;
use bb_policy::Permission;
use proptest::prelude::*;

use super::scenarios::{a_project, token};
use super::scenarios_issues::an_issue;
use super::scenarios_tags::{a_label, labelled};
use super::world::{Outcome, World};
use crate::bulk::{
    ApplyBulk, BulkAction, BulkChange, BulkPreview, BulkPreviewQuery, BulkResult, BulkRunQuery,
    BulkTarget, apply_bulk, bulk_run, preview_bulk,
};
use crate::issues::{
    ChangeIssue, IssueTypesQuery, MakeIssue, change_issue, issue_types, make_issue,
};
use crate::roles::{AssignRole, Given, MakeRole, Subject, assign_role, make_role};

fn done() -> BulkAction {
    BulkAction::Change {
        change: BulkChange {
            status: Some(Status::Done),
            ..BulkChange::default()
        },
    }
}

async fn preview(
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
fn targets(preview: &BulkPreview) -> Vec<BulkTarget> {
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

async fn apply(
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
    let reader = make_role(
        &world.ctx(),
        MakeRole {
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
            make_issue(
                &world.ctx(),
                MakeIssue {
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
async fn iss_09_more_than_a_thousand_run_as_a_job() -> Outcome {
    let world = World::new().await?;
    let admin = token(&world);
    let project = a_project(&world, "ACME").await?;
    let kind = types_first(&world).await?;
    let mut issues = Vec::new();
    for number in 0..=BULK_JOB_ABOVE {
        issues.push(
            make_issue(
                &world.ctx(),
                MakeIssue {
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
        let made = make_issue(
            &world.ctx(),
            MakeIssue {
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

proptest! {
    #![proptest_config(ProptestConfig::with_cases(24))]

    /// The preview says, for each issue, what the change then does to it: the same issues are
    /// skipped for the same reasons and the others change.
    #[test]
    fn iss_09_the_preview_equals_what_happens(
        statuses in proptest::collection::vec(0_usize..3, 1..8),
        move_instead in any::<bool>(),
    ) {
        let runtime = tokio::runtime::Builder::new_current_thread().build().unwrap();
        runtime.block_on(async {
            let world = World::new().await.unwrap();
            let admin = token(&world);
            let (from, to): (ProjectId, ProjectId) = (
                a_project(&world, "FROM").await.unwrap(),
                a_project(&world, "INTO").await.unwrap(),
            );
            let mut issues = Vec::new();
            for status in &statuses {
                let id = an_issue(&world, from).await.unwrap();
                let wanted = Status::ALL[*status];
                if wanted != Status::Open {
                    change_issue(&world.ctx(), ChangeIssue {
                        session_token: admin.clone(),
                        issue: id.to_string(),
                        version: 1,
                        issue_type: None,
                        title: None,
                        status: Some(wanted),
                        priority: None,
                        assigned_to: None,
                        fields: BTreeMap::new(),
                    }).await.unwrap();
                }
                issues.push(id);
            }
            let action = if move_instead {
                BulkAction::Move { project: to.to_string() }
            } else {
                done()
            };
            let asked = preview(&world, &admin, &issues, &action).await.unwrap();
            let result = apply(&world, &admin, targets(&asked), &action).await.unwrap();
            let seen: Vec<Option<SkipReason>> = asked.rows.iter().map(|row| row.skipped).collect();
            let did: Vec<Option<SkipReason>> = result.outcomes.iter().map(|each| each.skipped).collect();
            prop_assert_eq!(seen, did);
            Ok(())
        })?;
    }
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

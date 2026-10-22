//! The rules of a status change on the use cases (WF-03, WF-04, ADR-0031): who can do it, only if,
//! must be filled in and ask for, and that a rule never gives a right.

use bb_core::ErrorCode;
use bb_core::access::{AppliesTo, FieldLevel};
use bb_core::fields::{Field, FieldKey};
use bb_core::id::{IssueId, ProjectId, RoleId};
use bb_core::workflow::starter::{DONE, IN_PROGRESS, OPEN};
use bb_core::workflow::{Condition, StatusKey, StatusRules, Test, WhoCan};
use bb_policy::StartingRole;

use super::scenarios::a_project;
use super::scenarios_issues::an_issue;
use super::scenarios_workflows::{a_workflow, published_with};
use super::workflows::change;
use super::world::{Outcome, World};
use crate::access::PolicyStore as _;
use crate::auth::{AppError, Started};
use crate::issues::{ChangeIssue, IssueQuery, change_issue, issue};
use crate::roles::{
    AssignRole, CopyRole, Given, SetFieldLevel, Subject, assign_role, copy_role, set_level,
};
use crate::workflows::StatusChangeView;

pub(super) fn key(text: &str) -> StatusKey {
    StatusKey::new(text).unwrap()
}

pub(super) async fn starting_role(world: &World, role: StartingRole) -> Result<RoleId, String> {
    world
        .store
        .roles()
        .await
        .map_err(|error| error.to_string())?
        .into_iter()
        .find(|record| {
            record
                .role
                .starting
                .as_ref()
                .is_some_and(|word| word.as_str() == role.code())
        })
        .map(|record| record.role.id)
        .ok_or_else(|| "no such starting role".to_owned())
}

/// A person with a starting role in the project and below it.
pub(super) async fn member(
    world: &World,
    email: &str,
    role: StartingRole,
    project: ProjectId,
) -> Result<Started, AppError> {
    let person = world.person(email).await?;
    let role = starting_role(world, role)
        .await
        .map_err(AppError::Internal)?;
    assign(world, &person, role, project).await?;
    Ok(person)
}

/// Gives the person the role in the project and below it.
pub(super) async fn assign(
    world: &World,
    person: &Started,
    role: RoleId,
    project: ProjectId,
) -> Result<(), AppError> {
    assign_role(
        &world.ctx(),
        AssignRole {
            session_token: world.admin.session_token.clone(),
            subject: Subject::Person(person.person.id),
            given: Given::Role(role),
            project: Some(project),
            applies_to: AppliesTo::ProjectAndSubProjects,
        },
    )
    .await?;
    Ok(())
}

/// The rules of the change from `from` to `to`, written into the workflow of the project.
pub(super) async fn with_rules(
    world: &World,
    project: &str,
    rules: &[(&str, &str, StatusRules)],
) -> Result<(), AppError> {
    let editor = a_workflow(world, project)
        .await
        .map_err(AppError::Internal)?;
    published_with(world, &editor, |sent| {
        for (from, to, wanted) in rules {
            for each in &mut sent.changes {
                if each.from == *from && each.to == *to {
                    each.rules = wanted.clone();
                }
            }
        }
    })
    .await
}

/// The status changes the person is offered on the issue.
pub(super) async fn offered(
    world: &World,
    token: &str,
    id: IssueId,
) -> Result<Vec<StatusChangeView>, AppError> {
    Ok(issue(
        &world.ctx(),
        IssueQuery {
            session_token: token.to_owned(),
            issue: id.to_string(),
        },
    )
    .await?
    .into_value()
    .issue
    .status_changes)
}

/// Whether the issue offers a status change to this status to the person.
pub(super) async fn offers(
    world: &World,
    token: &str,
    id: IssueId,
    to: &str,
) -> Result<bool, AppError> {
    Ok(offered(world, token, id)
        .await?
        .iter()
        .any(|each| each.to.key == key(to)))
}

pub(super) async fn status_of(world: &World, token: &str, id: IssueId) -> Result<String, AppError> {
    Ok(issue(
        &world.ctx(),
        IssueQuery {
            session_token: token.to_owned(),
            issue: id.to_string(),
        },
    )
    .await?
    .into_value()
    .issue
    .status
    .key
    .to_string())
}

/// Changes the status of an issue as the person; what the change is refused with, if it is.
pub(super) async fn go(
    world: &World,
    token: &str,
    id: IssueId,
    version: i64,
    status: &str,
) -> Result<(), AppError> {
    change_issue(
        &world.ctx(),
        change(token, &id.to_string(), version, status)?,
    )
    .await
    .map(drop)
}

fn rule_for_roles(roles: Vec<RoleId>) -> StatusRules {
    StatusRules {
        who: WhoCan {
            roles,
            ..WhoCan::default()
        },
        ..StatusRules::default()
    }
}

#[tokio::test]
async fn wf_03_who_can_do_it_allows_only_the_named_roles_and_people() -> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "ACME").await?;
    let team = member(
        &world,
        "bob@example.test",
        StartingRole::TeamMember,
        project,
    )
    .await?;
    let lead = member(
        &world,
        "lena@example.test",
        StartingRole::ProjectLead,
        project,
    )
    .await?;
    let team_role = starting_role(&world, StartingRole::TeamMember).await?;
    let assigned = StatusRules {
        who: WhoCan {
            assigned: true,
            ..WhoCan::default()
        },
        ..StatusRules::default()
    };
    with_rules(
        &world,
        "ACME",
        &[
            (OPEN, IN_PROGRESS, rule_for_roles(vec![team_role])),
            (IN_PROGRESS, DONE, assigned),
        ],
    )
    .await?;
    let id = an_issue(&world, project).await?;
    // The project lead is not named: the button is not offered, and the change is refused.
    let buttons = offered(&world, &lead.session_token, id).await?;
    let keys: Vec<&str> = buttons.iter().map(|each| each.to.key.as_str()).collect();
    assert_eq!(keys, ["done"], "no button for a change they may not make");
    let by_lead = go(&world, &lead.session_token, id, 1, IN_PROGRESS).await;
    assert_eq!(
        by_lead.err(),
        Some(AppError::Refused(ErrorCode::WorkflowNotNamed))
    );
    // The team member is named by role.
    go(&world, &team.session_token, id, 1, IN_PROGRESS).await?;
    // *The person assigned*: the team member is not yet, so the change to Done is not theirs.
    let not_yet = go(&world, &team.session_token, id, 2, DONE).await;
    assert_eq!(
        not_yet.err(),
        Some(AppError::Refused(ErrorCode::WorkflowNotNamed))
    );
    let assign = ChangeIssue {
        assigned_to: Some(vec![team.person.id]),
        status: None,
        ..change(&world.admin.session_token, &id.to_string(), 2, IN_PROGRESS)?
    };
    change_issue(&world.ctx(), assign).await?;
    go(&world, &team.session_token, id, 3, DONE).await?;
    assert_eq!(status_of(&world, &team.session_token, id).await?, DONE);
    Ok(())
}

#[tokio::test]
async fn wf_03_only_if_refuses_with_a_sentence() -> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "ACME").await?;
    let team = member(
        &world,
        "bob@example.test",
        StartingRole::TeamMember,
        project,
    )
    .await?;
    let only_if_assigned = StatusRules {
        only_if: vec![Condition {
            field: FieldKey::BuiltIn(Field::AssignedTo),
            test: Test::Filled,
        }],
        ..StatusRules::default()
    };
    with_rules(&world, "ACME", &[(OPEN, IN_PROGRESS, only_if_assigned)]).await?;
    let id = an_issue(&world, project).await?;
    // The sentence is the code of the error, and it names the field the person may see.
    let by_team = go(&world, &team.session_token, id, 1, IN_PROGRESS).await;
    assert_eq!(
        by_team.err(),
        Some(AppError::RefusedAtField {
            code: ErrorCode::WorkflowOnlyIf,
            field: "assigned-to".to_owned(),
        })
    );
    // Once the condition holds, the same change goes through.
    let assign = ChangeIssue {
        status: None,
        assigned_to: Some(vec![team.person.id]),
        ..change(&team.session_token, &id.to_string(), 1, IN_PROGRESS)?
    };
    change_issue(&world.ctx(), assign).await?;
    go(&world, &team.session_token, id, 2, IN_PROGRESS).await?;
    Ok(())
}

#[tokio::test]
async fn wf_03_a_rule_never_gives_a_right_change_issues_does_not() -> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "ACME").await?;
    let viewer = member(&world, "vera@example.test", StartingRole::Viewer, project).await?;
    // The rule names the reporter and the person assigned: a Viewer has no *Change issues* and
    // still cannot change the status of an issue.
    let reporter_or_assigned = StatusRules {
        who: WhoCan {
            reporter: true,
            assigned: true,
            ..WhoCan::default()
        },
        ..StatusRules::default()
    };
    with_rules(&world, "ACME", &[(OPEN, IN_PROGRESS, reporter_or_assigned)]).await?;
    let id = an_issue(&world, project).await?;
    let by_viewer = go(&world, &viewer.session_token, id, 1, IN_PROGRESS).await;
    assert_eq!(
        by_viewer.err(),
        Some(AppError::Refused(ErrorCode::Forbidden))
    );
    assert!(offered(&world, &viewer.session_token, id).await?.is_empty());
    Ok(())
}

#[tokio::test]
async fn wf_04_must_be_filled_in_names_the_missing_field() -> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "ACME").await?;
    let team = member(
        &world,
        "bob@example.test",
        StartingRole::TeamMember,
        project,
    )
    .await?;
    let assigned = FieldKey::BuiltIn(Field::AssignedTo);
    let must_assign = StatusRules {
        must_fill: vec![assigned],
        ..StatusRules::default()
    };
    with_rules(&world, "ACME", &[(OPEN, IN_PROGRESS, must_assign)]).await?;
    let id = an_issue(&world, project).await?;
    // The button is offered with what it needs.
    let buttons = offered(&world, &team.session_token, id).await?;
    let start = buttons
        .iter()
        .find(|each| each.to.key == key(IN_PROGRESS))
        .ok_or("no button")?;
    assert_eq!(start.needs, [assigned]);
    // The change without it is refused at that box; with it in the same request, it goes through.
    let bare = go(&world, &team.session_token, id, 1, IN_PROGRESS).await;
    assert_eq!(
        bare.err(),
        Some(AppError::RefusedAtField {
            code: ErrorCode::FieldRequired,
            field: "assigned-to".to_owned(),
        })
    );
    let both = ChangeIssue {
        assigned_to: Some(vec![team.person.id]),
        ..change(&team.session_token, &id.to_string(), 1, IN_PROGRESS)?
    };
    change_issue(&world.ctx(), both).await?;
    Ok(())
}

/// A person whose role is a copy of a Team member's that does not see *Assigned to*.
pub(super) async fn person_who_cannot_see_who_is_assigned(
    world: &World,
    project: ProjectId,
) -> Result<Started, AppError> {
    let person = world.person("blind@example.test").await?;
    let team_role = starting_role(world, StartingRole::TeamMember)
        .await
        .map_err(AppError::Internal)?;
    let admin = world.admin.session_token.clone();
    let copy = copy_role(
        &world.ctx(),
        CopyRole {
            session_token: admin.clone(),
            role: team_role,
            name: "Blind".to_owned(),
        },
    )
    .await?
    .into_value();
    set_level(
        &world.ctx(),
        SetFieldLevel {
            session_token: admin.clone(),
            role: copy.id,
            project: None,
            issue_type: None,
            field: FieldKey::BuiltIn(Field::AssignedTo),
            level: Some(FieldLevel::Hidden),
        },
    )
    .await?;
    assign_role(
        &world.ctx(),
        AssignRole {
            session_token: admin,
            subject: Subject::Person(person.person.id),
            given: Given::Role(copy.id),
            project: Some(project),
            applies_to: AppliesTo::ProjectAndSubProjects,
        },
    )
    .await?;
    Ok(person)
}

#[tokio::test]
async fn wf_04_ask_for_never_asks_for_a_hidden_field() -> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "ACME").await?;
    let ahead = member(
        &world,
        "bob@example.test",
        StartingRole::TeamMember,
        project,
    )
    .await?;
    let blind = person_who_cannot_see_who_is_assigned(&world, project).await?;
    let ask_who = StatusRules {
        ask_for: vec![FieldKey::BuiltIn(Field::AssignedTo)],
        ..StatusRules::default()
    };
    with_rules(&world, "ACME", &[(OPEN, IN_PROGRESS, ask_who)]).await?;
    let id = an_issue(&world, project).await?;
    // Who sees the field is asked for it.
    let buttons = offered(&world, &ahead.session_token, id).await?;
    let start = buttons
        .iter()
        .find(|each| each.to.key == key(IN_PROGRESS))
        .ok_or("no button")?;
    assert_eq!(start.needs, [FieldKey::BuiltIn(Field::AssignedTo)]);
    // Who does not see it is not asked, and the refusal names no field (SEC-06): the change is not
    // offered, and asking for it fails with a sentence that has no field in it.
    assert!(!offers(&world, &blind.session_token, id, IN_PROGRESS).await?);
    let by_blind = go(&world, &blind.session_token, id, 1, IN_PROGRESS).await;
    assert_eq!(
        by_blind.err(),
        Some(AppError::Refused(ErrorCode::WorkflowCannotFill))
    );
    Ok(())
}

#[tokio::test]
async fn sec_06_a_refusal_of_a_status_change_names_no_field_the_person_cannot_see() -> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "ACME").await?;
    let blind = person_who_cannot_see_who_is_assigned(&world, project).await?;
    let hidden = FieldKey::BuiltIn(Field::AssignedTo);
    let id = an_issue(&world, project).await?;
    for rules in [
        StatusRules {
            ask_for: vec![hidden],
            ..StatusRules::default()
        },
        StatusRules {
            must_fill: vec![hidden],
            ..StatusRules::default()
        },
        StatusRules {
            only_if: vec![Condition {
                field: hidden,
                test: Test::Empty,
            }],
            ..StatusRules::default()
        },
    ] {
        with_rules(&world, "ACME", &[(OPEN, IN_PROGRESS, rules)]).await?;
        let refused = go(&world, &blind.session_token, id, 1, IN_PROGRESS).await;
        // A refusal about a field would be `RefusedAtField`: this one is about the change.
        assert_eq!(
            refused.err(),
            Some(AppError::Refused(ErrorCode::WorkflowCannotFill))
        );
    }
    Ok(())
}

#[tokio::test]
async fn wf_03_the_person_assigned_counts_only_for_who_sees_who_is_assigned() -> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "ACME").await?;
    let sighted = member(
        &world,
        "bob@example.test",
        StartingRole::TeamMember,
        project,
    )
    .await?;
    let blind = person_who_cannot_see_who_is_assigned(&world, project).await?;
    let only_assigned = StatusRules {
        who: WhoCan {
            assigned: true,
            ..WhoCan::default()
        },
        ..StatusRules::default()
    };
    with_rules(&world, "ACME", &[(OPEN, IN_PROGRESS, only_assigned)]).await?;
    let id = an_issue(&world, project).await?;
    let admin = world.admin.session_token.clone();
    let mut assign_both = change(&admin, &id.to_string(), 1, OPEN)?;
    assign_both.status = None;
    assign_both.assigned_to = Some(vec![sighted.person.id, blind.person.id]);
    change_issue(&world.ctx(), assign_both).await?;
    // Both are assigned; the buttons of the one who is not shown the field do not tell it.
    assert!(offers(&world, &sighted.session_token, id, IN_PROGRESS).await?);
    assert!(!offers(&world, &blind.session_token, id, IN_PROGRESS).await?);
    Ok(())
}

/// An issue of the project that the administrator has put into *In progress*.
pub(super) async fn issue_in_progress(
    world: &World,
    project: ProjectId,
) -> Result<IssueId, AppError> {
    let id = an_issue(world, project).await.map_err(AppError::Internal)?;
    let admin = world.admin.session_token.clone();
    change_issue(
        &world.ctx(),
        change(&admin, &id.to_string(), 1, IN_PROGRESS)?,
    )
    .await?;
    Ok(id)
}

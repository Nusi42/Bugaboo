//! *Check access* names the level of a field for one issue type (SEC-03, M3-F44, ADR-0027): the
//! answer says whether the levels differ by type, so the screen asks which type is meant only then.
//! An issue read whole is masked by the levels of its own type, the view of the issue inside it too.

use bb_core::access::FieldLevel;
use bb_core::fields::{Field, FieldKey};
use bb_core::id::{IssueTypeId, ProjectId, RoleId};
use bb_policy::StartingRole;

use super::offers::{another_type, refused};
use super::scenarios::{a_project, text, token};
use super::scenarios_issues::a_type;
use super::workflows_rules::{assign, starting_role};
use super::world::{Outcome, World};
use crate::auth::{AppError, Started};
use crate::issues::{CreateIssue, IssueQuery, create_issue, issue};
use crate::roles::{
    CheckAccessQuery, CopyRole, LevelPlace, SetFieldLevel, check_access, copy_role, set_level,
};

use bb_core::ErrorCode;

/// The cost center, which the role below sees for every type but one.
const FIELD: Field = Field::EstimatedTime;

/// A person with a copy of the role of a project lead in ACME, and two issue types.
struct CarefulLead {
    person: Started,
    role: RoleId,
    acme: ProjectId,
    task: IssueTypeId,
    bug: IssueTypeId,
}

async fn careful_lead(world: &World) -> Result<CarefulLead, String> {
    let acme = a_project(world, "ACME").await?;
    let (task, bug) = (
        a_type(world).await?,
        another_type(world).await.map_err(text)?,
    );
    let person = world.person("lena@example.test").await.map_err(text)?;
    let lead = starting_role(world, StartingRole::ProjectLead).await?;
    let copy = copy_role(
        &world.ctx(),
        CopyRole {
            session_token: token(world),
            role: lead,
            name: "Careful lead".to_owned(),
        },
    )
    .await
    .map_err(text)?
    .into_value();
    assign(world, &person, copy.id, acme).await.map_err(text)?;
    Ok(CarefulLead {
        person,
        role: copy.id,
        acme,
        task,
        bug,
    })
}

/// The role of the careful lead does not see `field` on issues of `issue_type`.
async fn hidden_for(
    world: &World,
    role: RoleId,
    issue_type: IssueTypeId,
    field: Field,
) -> Result<(), AppError> {
    set_level(
        &world.ctx(),
        SetFieldLevel {
            session_token: token(world),
            role,
            project: None,
            issue_type: Some(issue_type),
            field: FieldKey::BuiltIn(field),
            level: Some(FieldLevel::Hidden),
        },
    )
    .await?;
    Ok(())
}

#[tokio::test]
async fn sec_03_check_access_answers_for_one_issue_type_in_memory() -> Outcome {
    let world = World::new().await?;
    let admin = token(&world);
    let CarefulLead {
        person,
        role,
        acme,
        task,
        bug,
    } = careful_lead(&world).await?;
    let check = |issue_type| CheckAccessQuery {
        session_token: admin.clone(),
        person: person.person.id,
        place: Some(acme),
        issue_type,
    };
    let level_of = |answer: &crate::roles::AccessAnswer| {
        answer
            .fields
            .iter()
            .find(|each| each.subject == FieldKey::BuiltIn(FIELD))
            .map(|each| {
                (
                    each.level,
                    each.roles
                        .first()
                        .map(|says| (says.level, says.set.clone())),
                )
            })
    };
    // While no level is set for a type, the levels are the same for every type and the screen
    // asks nothing.
    let plain = check_access(&world.ctx(), check(None)).await?.into_value();
    assert!(!plain.differs_by_type);
    assert!(plain.issue_type.is_none());
    let same = check_access(&world.ctx(), check(Some(bug)))
        .await?
        .into_value();
    assert!(!same.differs_by_type);
    assert_eq!(level_of(&same), level_of(&plain));
    // The role sees the cost center of bugs less than it does elsewhere.
    hidden_for(&world, role, bug, FIELD).await?;
    let all = check_access(&world.ctx(), check(None)).await?.into_value();
    assert!(all.differs_by_type, "the screen asks which issue type");
    let for_bug = check_access(&world.ctx(), check(Some(bug)))
        .await?
        .into_value();
    assert_eq!(for_bug.issue_type.as_ref().map(|each| each.id), Some(bug));
    let (level, says) = level_of(&for_bug).ok_or("no cost center")?;
    assert_eq!(level, FieldLevel::Hidden);
    let says = says.ok_or("no role says")?;
    assert_eq!(says, (FieldLevel::Hidden, LevelPlace::RoleOwn));
    // For another type the level is the one for all types, with the reason it has.
    let for_task = check_access(&world.ctx(), check(Some(task)))
        .await?
        .into_value();
    assert_eq!(
        level_of(&for_task).map(|found| found.0),
        level_of(&all).map(|found| found.0)
    );
    assert_ne!(
        level_of(&for_task).map(|found| found.0),
        Some(FieldLevel::Hidden)
    );
    // A type that is not there is not found.
    let nobody = IssueTypeId::from_parts(jiff::Timestamp::UNIX_EPOCH, [9; 10])
        .map_err(|error| AppError::Internal(error.to_string()))?;
    let missing = check_access(&world.ctx(), check(Some(nobody))).await;
    assert!(refused(&missing, ErrorCode::NotFound));
    Ok(())
}

/// An issue of `issue_type` in `project` with a description.
async fn described(
    world: &World,
    project: ProjectId,
    issue_type: IssueTypeId,
) -> Result<String, AppError> {
    let made = CreateIssue {
        description: Some("Steps to see it".to_owned()),
        ..CreateIssue::of(
            &token(world),
            &project.to_string(),
            issue_type,
            "Login screen",
        )
    };
    Ok(create_issue(&world.ctx(), made)
        .await?
        .into_value()
        .id
        .to_string())
}

#[tokio::test]
async fn sec_03_an_issue_read_whole_masks_its_view_by_the_levels_of_its_type() -> Outcome {
    let world = World::new().await?;
    let CarefulLead {
        person,
        role,
        acme,
        task,
        bug,
    } = careful_lead(&world).await?;
    hidden_for(&world, role, bug, Field::Description).await?;
    let query = |id: String| IssueQuery {
        session_token: person.session_token.clone(),
        issue: id,
    };
    // The description in the view of the issue, and the one the answer renders beside it.
    let description = |answer: &serde_json::Value| {
        (
            answer["issue"].get("description").is_some(),
            answer.get("description_html").is_some(),
        )
    };
    // The answer and the view of the issue inside it have the mask of the issue's type: a bug
    // shows no description, a task does.
    let bug_issue = issue(&world.ctx(), query(described(&world, acme, bug).await?)).await?;
    assert_eq!(description(&bug_issue.masked_value()?), (false, false));
    let task_issue = issue(&world.ctx(), query(described(&world, acme, task).await?)).await?;
    assert_eq!(description(&task_issue.masked_value()?), (true, true));
    Ok(())
}

//! Which roles may change a field here (M3-F53, ADR-0031): the answer names only roles somebody
//! holds in the project, nothing about a field the reader may not see, and no role at all to a
//! reader who is not shown the people of the project by role.

use bb_core::ErrorCode;
use bb_core::access::{FieldLevel, PeopleLevel};
use bb_core::fields::{Field, FieldKey};
use bb_core::id::ProjectId;
use bb_core::people_kinds::PeopleKind;
use bb_policy::StartingRole;

use super::scenarios::a_project;
use super::workflows_rules::{member, starting_role};
use super::world::{Outcome, World};
use crate::access::{WhoMayChange, WhoMayChangeQuery, who_may_change};
use crate::auth::AppError;
use crate::roles::{CopyRole, SetFieldLevel, SetPeopleLevel, copy_role, set_level};

const STATUS: FieldKey = FieldKey::BuiltIn(Field::Status);

async fn asked(
    world: &World,
    token: &str,
    project: &str,
    field: FieldKey,
) -> Result<WhoMayChange, AppError> {
    Ok(who_may_change(
        &world.ctx(),
        WhoMayChangeQuery {
            session_token: token.to_owned(),
            project: project.to_owned(),
            field,
            issue_type: None,
        },
    )
    .await?
    .into_value())
}

fn names(answer: &WhoMayChange) -> Vec<&str> {
    answer.roles.iter().map(|role| role.name.as_str()).collect()
}

#[tokio::test]
async fn m3_f53_the_answer_names_the_roles_held_in_the_project_that_may_change_the_field() -> Outcome
{
    let world = World::new().await?;
    let project = a_project(&world, "ACME").await?;
    let team = member(
        &world,
        "tim@example.test",
        StartingRole::TeamMember,
        project,
    )
    .await?;
    let answer = asked(&world, &team.session_token, "ACME", STATUS).await?;
    assert!(!answer.roles_hidden);
    let found = names(&answer);
    assert!(found.contains(&"Team member"), "{found:?}");
    // Nobody holds the role *Viewer* in the project, and a viewer could not change anything.
    assert!(!found.contains(&"Viewer"), "{found:?}");
    Ok(())
}

#[tokio::test]
async fn m3_f53_a_field_the_reader_cannot_see_is_not_found() -> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "ACME").await?;
    let team_role = starting_role(&world, StartingRole::TeamMember).await?;
    let copy = copy_role(
        &world.ctx(),
        CopyRole {
            session_token: world.admin.session_token.clone(),
            role: team_role,
            name: "Blind".to_owned(),
        },
    )
    .await?
    .into_value();
    set_level(
        &world.ctx(),
        SetFieldLevel {
            session_token: world.admin.session_token.clone(),
            role: copy.id,
            project: None,
            issue_type: None,
            field: STATUS,
            level: Some(FieldLevel::Hidden),
        },
    )
    .await?;
    let person = world.person("blind@example.test").await?;
    super::workflows_rules::assign(&world, &person, copy.id, project).await?;
    let refused = asked(&world, &person.session_token, "ACME", STATUS).await;
    assert_eq!(
        refused.err(),
        Some(AppError::Refused(ErrorCode::NotFound)),
        "the same answer as for a field that is not there"
    );
    Ok(())
}

#[tokio::test]
async fn m3_f53_a_reader_who_is_not_shown_the_people_of_the_project_is_named_no_role() -> Outcome {
    let world = World::new().await?;
    let project: ProjectId = a_project(&world, "ACME").await?;
    let team_role = starting_role(&world, StartingRole::TeamMember).await?;
    let copy = copy_role(
        &world.ctx(),
        CopyRole {
            session_token: world.admin.session_token.clone(),
            role: team_role,
            name: "Unaware".to_owned(),
        },
    )
    .await?
    .into_value();
    set_level(
        &world.ctx(),
        SetPeopleLevel {
            session_token: world.admin.session_token.clone(),
            role: copy.id,
            project: None,
            kind: PeopleKind::ProjectMembers,
            level: Some(PeopleLevel::Hidden),
        },
    )
    .await?;
    let person = world.person("unaware@example.test").await?;
    super::workflows_rules::assign(&world, &person, copy.id, project).await?;
    // Another person holds a role there that may change the field: the reader learns nothing of it.
    member(
        &world,
        "tim@example.test",
        StartingRole::TeamMember,
        project,
    )
    .await?;
    let answer = asked(&world, &person.session_token, "ACME", STATUS).await?;
    assert!(answer.roles.is_empty());
    assert!(answer.roles_hidden);
    Ok(())
}

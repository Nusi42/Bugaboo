//! *Check access* names the level of a field for one issue type (SEC-03, M3-F44, ADR-0027): the
//! answer says whether the levels differ by type, so the screen asks which type is meant only then.

use bb_core::access::FieldLevel;
use bb_core::fields::{Field, FieldKey};
use bb_policy::StartingRole;

use super::offers::{another_type, refused};
use super::scenarios::{a_project, token};
use super::scenarios_issues::a_type;
use super::workflows_rules::{assign, starting_role};
use super::world::{Outcome, World};
use crate::auth::AppError;
use crate::roles::{
    CheckAccessQuery, CopyRole, LevelPlace, SetFieldLevel, check_access, copy_role, set_level,
};

use bb_core::ErrorCode;

/// The cost center, which the role below sees for every type but one.
const FIELD: Field = Field::CostCenter;

#[tokio::test]
async fn sec_03_check_access_names_the_level_per_issue_type() -> Outcome {
    let world = World::new().await?;
    let admin = token(&world);
    let acme = a_project(&world, "ACME").await?;
    let (task, bug) = (a_type(&world).await?, another_type(&world).await?);
    let person = world.person("lena@example.test").await?;
    let lead = starting_role(&world, StartingRole::ProjectLead).await?;
    let copy = copy_role(
        &world.ctx(),
        CopyRole {
            session_token: admin.clone(),
            role: lead,
            name: "Careful lead".to_owned(),
        },
    )
    .await?
    .into_value();
    assign(&world, &person, copy.id, acme).await?;
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
    set_level(
        &world.ctx(),
        SetFieldLevel {
            session_token: admin.clone(),
            role: copy.id,
            project: None,
            issue_type: Some(bug),
            field: FieldKey::BuiltIn(FIELD),
            level: Some(FieldLevel::Hidden),
        },
    )
    .await?;
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
    let nobody = bb_core::id::IssueTypeId::from_parts(jiff::Timestamp::UNIX_EPOCH, [9; 10])
        .map_err(|error| AppError::Internal(error.to_string()))?;
    let missing = check_access(&world.ctx(), check(Some(nobody))).await;
    assert!(refused(&missing, ErrorCode::NotFound));
    Ok(())
}

//! Names that were stored before one rule told them apart (`bb_core::names`): two issue types,
//! fields, choices or kinds of relation that differ only in the spaces inside them stay as they are.
//! They can be given another icon or look, be retired and be saved again; only a name that is new or
//! changed is judged, and it is refused when it is the same name as another one.

use bb_core::ErrorCode;
use bb_core::id::{ChoiceId, FieldId, IssueTypeId, RelationKindId};
use bb_core::issue::IssueIcon;
use bb_core::own_fields::{Edit, FieldType};

use super::scenarios::token;
use super::scenarios_fields::a_field;
use super::world::{Outcome, World};
use crate::auth::AppError;
use crate::fields::{ChangeChoice, ChangeField, change_choice, change_field};
use crate::issues::{ChangeIssueType, change_issue_type};
use crate::relations::{ChangeRelationKind, change_relation_kind};

fn refused<T>(answer: &Result<T, AppError>) -> bool {
    matches!(answer, Err(AppError::Refused(ErrorCode::NameTaken)))
}

#[tokio::test]
async fn iss_02_two_stored_issue_types_that_differ_only_in_spaces_can_be_retired_and_not_renamed()
-> Outcome {
    let world = World::new().await?;
    let (one, two) = {
        let mut shared = world.store.lock();
        let mut made = shared.state.issue_types[0].clone();
        made.project = None;
        made.version = 1;
        made.retired_at = None;
        let mut spaced = made.clone();
        made.id = IssueTypeId::generate(&mut world.ctx().ids())?;
        made.name = "Spaced  question".to_owned();
        spaced.id = IssueTypeId::generate(&mut world.ctx().ids())?;
        spaced.name = "Spaced question".to_owned();
        let ids = (made.id, spaced.id);
        shared.state.issue_types.push(made);
        shared.state.issue_types.push(spaced);
        ids
    };
    let change = |issue_type, name: Option<&str>, icon, retired| ChangeIssueType {
        session_token: token(&world),
        issue_type,
        version: 1,
        name: name.map(str::to_owned),
        icon,
        retired,
    };
    // A new name that is the other one's is refused.
    let renamed = change_issue_type(
        &world.ctx(),
        change(one, Some("spaced   QUESTION"), None, None),
    )
    .await;
    assert!(refused(&renamed), "{renamed:?}");
    // Another icon, and a retire, are not a new name.
    let icon = change_issue_type(&world.ctx(), change(one, None, Some(IssueIcon::Bug), None)).await;
    assert!(icon.is_ok(), "{icon:?}");
    let retired = change_issue_type(&world.ctx(), change(two, None, None, Some(true))).await;
    assert!(retired.is_ok(), "{retired:?}");
    Ok(())
}

#[tokio::test]
async fn iss_04_two_stored_kinds_of_relation_that_differ_only_in_spaces_can_be_retired_and_not_renamed()
-> Outcome {
    let world = World::new().await?;
    let (one, two) = {
        let mut shared = world.store.lock();
        let mut made = shared.state.relation_kinds[0].clone();
        made.version = 1;
        made.retired_at = None;
        let mut spaced = made.clone();
        made.id = RelationKindId::generate(&mut world.ctx().ids())?;
        made.forward = "points  at".to_owned();
        made.backward = "is pointed at by".to_owned();
        spaced.id = RelationKindId::generate(&mut world.ctx().ids())?;
        spaced.forward = "points at".to_owned();
        spaced.backward = "is  pointed  at  by".to_owned();
        let ids = (made.id, spaced.id);
        shared.state.relation_kinds.push(made);
        shared.state.relation_kinds.push(spaced);
        ids
    };
    let change = |kind, forward: Option<&str>, retired| ChangeRelationKind {
        session_token: token(&world),
        kind,
        version: 1,
        forward: forward.map(str::to_owned),
        backward: None,
        retired,
    };
    let renamed = change_relation_kind(&world.ctx(), change(two, Some("POINTS   AT"), None)).await;
    assert!(refused(&renamed), "{renamed:?}");
    let retired = change_relation_kind(&world.ctx(), change(one, None, Some(true))).await;
    assert!(retired.is_ok(), "{retired:?}");
    // A kind shows its names as they are stored, also when both read the same.
    let shown = world.store.lock().state.relation_kinds.clone();
    assert!(shown.iter().any(|kind| kind.forward == "points  at"));
    Ok(())
}

#[tokio::test]
async fn cf_02_two_stored_fields_and_choices_that_differ_only_in_spaces_can_be_retired_and_not_renamed()
-> Outcome {
    let world = World::new().await?;
    let field = a_field(&world, "Customer area", FieldType::PickOne, &["Very small"]).await?;
    let (first, second, choice) = {
        let mut shared = world.store.lock();
        let at = shared
            .state
            .field_definitions
            .iter()
            .position(|found| found.field.id == field.id)
            .ok_or("no field")?;
        let mut twin = shared.state.field_definitions[at].clone();
        twin.field.id = FieldId::generate(&mut world.ctx().ids())?;
        twin.field.name = "Customer  area".to_owned();
        twin.choices.clear();
        let second = twin.field.id;
        shared.state.field_definitions.push(twin);
        let mut spaced = shared.state.field_definitions[at].choices[0].clone();
        spaced.id = ChoiceId::generate(&mut world.ctx().ids())?;
        spaced.name = "Very  small".to_owned();
        let choice = spaced.id;
        shared.state.field_definitions[at].choices.push(spaced);
        (field.id, second, choice)
    };
    let version = |id: FieldId| {
        world
            .store
            .lock()
            .state
            .field_definitions
            .iter()
            .find(|found| found.field.id == id)
            .map(|found| found.field.version)
            .ok_or("no field")
    };
    let field_change = |field: FieldId, name: Option<&str>, retired| -> Result<ChangeField, &str> {
        Ok(ChangeField {
            session_token: token(&world),
            field,
            version: version(field)?,
            name: name.map(str::to_owned),
            help: None,
            rules: None,
            default_value: Edit::Keep,
            look: None,
            never_to_assistant: None,
            scope: None,
            retired,
        })
    };
    // A look or a retire is not a new name.
    let retired = change_field(&world.ctx(), field_change(second, None, Some(true))?).await;
    assert!(retired.is_ok(), "{retired:?}");
    let renamed = change_field(
        &world.ctx(),
        field_change(first, Some("customer   AREA"), None)?,
    )
    .await;
    assert!(refused(&renamed), "{renamed:?}");
    // The same for the choices of a field.
    let choice_change =
        |choice: ChoiceId, name: Option<&str>, retired| -> Result<ChangeChoice, &str> {
            Ok(ChangeChoice {
                session_token: token(&world),
                field: first,
                choice,
                version: version(first)?,
                name: name.map(str::to_owned),
                color: None,
                icon: Edit::Keep,
                retired,
            })
        };
    let retired = change_choice(&world.ctx(), choice_change(choice, None, Some(true))?).await;
    assert!(retired.is_ok(), "{retired:?}");
    let original = field.choices[0].id;
    let renamed = change_choice(
        &world.ctx(),
        choice_change(original, Some("VERY   small"), None)?,
    )
    .await;
    assert!(refused(&renamed), "{renamed:?}");
    Ok(())
}

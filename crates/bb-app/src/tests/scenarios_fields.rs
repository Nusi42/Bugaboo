//! The scenarios of the own-field use cases (ADR-0007, M1-F18): the preparation, the call, and how
//! many events the call added. `events.rs` names each use case once and runs them.

use bb_core::edit::EditLock;
use bb_core::event::{EntityRef, RecordKind};
use bb_core::id::{ChoiceId, EditorId, FieldId, LockToken};
use bb_core::own_fields::edit_part;
use bb_core::own_fields::{Edit, FieldFor, FieldType, Rules, Scope};
use jiff::Timestamp;

use super::scenarios::{Added, a_project, measured, text, token};
use super::scenarios_issues::an_issue;
use super::world::World;
use crate::fields::{
    AddChoice, ChangeChoice, ChangeField, CombineChoices, CreateField, FieldView, NewChoice,
    ReorderChoices, SaveFieldText, add_choice, change_choice, change_field, combine_choices,
    create_field, reorder_choices, save_field_text,
};

/// The input that makes a field of this type for issues, with these choices.
pub(crate) fn new_field(
    world: &World,
    name: &str,
    kind: FieldType,
    choices: &[&str],
) -> CreateField {
    CreateField {
        session_token: token(world),
        name: name.to_owned(),
        help: None,
        kind,
        applies_to: FieldFor::Issues,
        rules: Rules::default(),
        default_value: None,
        look: None,
        never_to_assistant: false,
        scope: Scope::default(),
        choices: choices
            .iter()
            .map(|name| NewChoice {
                name: (*name).to_owned(),
                color: None,
                icon: None,
            })
            .collect(),
    }
}

/// Creates a field and says what it looks like now.
pub(crate) async fn a_field(
    world: &World,
    name: &str,
    kind: FieldType,
    choices: &[&str],
) -> Result<FieldView, String> {
    Ok(
        create_field(&world.ctx(), new_field(world, name, kind, choices))
            .await
            .map_err(text)?
            .into_value(),
    )
}

pub(crate) async fn create_field_scenario(world: &World) -> Added {
    measured(
        &world.store,
        create_field(
            &world.ctx(),
            new_field(world, "Customer", FieldType::Text, &[]),
        ),
    )
    .await
}

pub(crate) async fn change_field_scenario(world: &World) -> Added {
    let field = a_field(world, "Customer", FieldType::Text, &[]).await?;
    measured(
        &world.store,
        change_field(
            &world.ctx(),
            ChangeField {
                session_token: token(world),
                field: field.id,
                version: field.version,
                name: Some("Client".to_owned()),
                help: Some("Who asked".to_owned()),
                rules: None,
                default_value: Edit::Keep,
                look: None,
                never_to_assistant: None,
                scope: None,
                retired: None,
            },
        ),
    )
    .await
}

pub(crate) async fn add_choice_scenario(world: &World) -> Added {
    let field = a_field(world, "Severity", FieldType::PickOne, &["Minor"]).await?;
    measured(
        &world.store,
        add_choice(
            &world.ctx(),
            AddChoice {
                session_token: token(world),
                field: field.id,
                version: field.version,
                name: "Critical".to_owned(),
                color: None,
                icon: None,
            },
        ),
    )
    .await
}

pub(crate) async fn change_choice_scenario(world: &World) -> Added {
    let field = a_field(world, "Severity", FieldType::PickOne, &["Minor"]).await?;
    measured(
        &world.store,
        change_choice(
            &world.ctx(),
            ChangeChoice {
                session_token: token(world),
                field: field.id,
                choice: field.choices[0].id,
                version: field.version,
                name: Some("Small".to_owned()),
                color: None,
                icon: Edit::Keep,
                retired: None,
            },
        ),
    )
    .await
}

/// The IDs of the choices of a field, in order.
fn choices_of(field: &FieldView) -> Vec<ChoiceId> {
    field.choices.iter().map(|choice| choice.id).collect()
}

pub(crate) async fn reorder_choices_scenario(world: &World) -> Added {
    let field = a_field(world, "Severity", FieldType::PickOne, &["Minor", "Major"]).await?;
    let mut order = choices_of(&field);
    order.reverse();
    measured(
        &world.store,
        reorder_choices(
            &world.ctx(),
            ReorderChoices {
                session_token: token(world),
                field: field.id,
                version: field.version,
                order,
            },
        ),
    )
    .await
}

pub(crate) async fn combine_choices_scenario(world: &World) -> Added {
    let field = a_field(world, "Severity", FieldType::PickOne, &["Minor", "Small"]).await?;
    let (from, into): (ChoiceId, ChoiceId) = (field.choices[1].id, field.choices[0].id);
    measured(
        &world.store,
        combine_choices(
            &world.ctx(),
            CombineChoices {
                session_token: token(world),
                field: FieldId::from_bytes(field.id.to_bytes()).map_err(text)?,
                choice: from,
                into,
                version: field.version,
            },
        ),
    )
    .await
}

/// The lock Ada holds on the long text `field` of the issue, to the end of the century.
pub(crate) fn lock_the_text(
    world: &World,
    issue: bb_core::id::IssueId,
    field: FieldId,
) -> LockToken {
    let token = LockToken::from_random([9; 16]);
    world.store.lock().state.locks.push(EditLock {
        record: EntityRef {
            kind: RecordKind::Issue.into(),
            id: issue.into(),
        },
        part: edit_part(field).unwrap_or_else(|_| unreachable!()),
        holder: world.admin.person.id,
        editor: EditorId::from_random([8; 16]),
        token,
        taken_at: Timestamp::UNIX_EPOCH,
        expires_at: Timestamp::MAX,
    });
    token
}

pub(crate) async fn save_field_text_scenario(world: &World) -> Added {
    let project = a_project(world, "ACME").await?;
    let issue = an_issue(world, project).await?;
    let field = a_field(world, "Notes", FieldType::LongText, &[]).await?;
    let token = lock_the_text(world, issue, field.id);
    measured(
        &world.store,
        save_field_text(
            &world.ctx(),
            SaveFieldText {
                session_token: token_of(world),
                issue: issue.to_string(),
                field: field.id,
                token,
                version: 1,
                text: "Steps to reproduce".to_owned(),
            },
        ),
    )
    .await
}

fn token_of(world: &World) -> String {
    token(world)
}

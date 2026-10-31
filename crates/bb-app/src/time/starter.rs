//! The starter fields of time entries, *Activity* and *Cost center* (TIME-13, ADR-0035 decision
//! 1): own fields that Bugaboo makes at the first start and on the upgrade of an existing server,
//! each with its mark, once. Administrators rename, change or retire them like any own field; the
//! mark is how Bugaboo finds them again, never the name.

use bb_core::ErrorCode;
use bb_core::event::Actor;
use bb_core::id::FieldId;
use bb_core::message::Message;
use bb_core::own_fields::{FieldFor, FieldType, Look, OwnField, Rules, Scope};
use bb_core::time::entry::StarterField;
use bb_core::version::FIRST_VERSION;

use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::fields::{Definition, NewChoice, NewField, next_position};

/// Make the starter fields of time entries that are not there yet (the server does this itself at
/// every start).
pub struct CreateStarterTimeFields;

/// Makes each starter field of time entries that has no field with its mark yet: one change, with
/// its event, for each field made. A start that finds both made does nothing.
///
/// # Errors
/// Fails when the store fails.
pub async fn create_starter_time_fields<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    _input: CreateStarterTimeFields,
) -> Result<(), AppError> {
    let made = ctx
        .store()
        .starter_fields()
        .await
        .map_err(|error| internal::<S>(&error))?;
    for starter in StarterField::ALL {
        if made.iter().any(|mark| mark.starter == *starter) {
            continue;
        }
        let definition = ctx.starter_definition(*starter).await?;
        let plan = ctx
            .plan_new_field(Actor::System, definition, Some(*starter))
            .await?;
        let (event, why) = (plan.event.clone(), plan.why.clone());
        let result = ctx
            .record_change(&event, move |transaction| {
                Box::pin(async move { plan.write::<S>(transaction).await })
            })
            .await;
        // A field of the organization that has the name already keeps it: the starter field is
        // then not made, and nothing is lost.
        match NewField::outcome::<S>(&why, result) {
            Ok(_) | Err(AppError::Refused(ErrorCode::NameTaken)) => {}
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The field a starter field is made as: a *Pick one* list of time entries, used everywhere,
    /// with its starting choices, not required.
    async fn starter_definition(&self, starter: StarterField) -> Result<Definition, AppError> {
        let id = FieldId::generate(&mut self.ids())
            .map_err(|error| AppError::Internal(error.to_string()))?;
        let names: Vec<NewChoice> = starter
            .starting_choices()
            .iter()
            .map(|name| NewChoice {
                name: Message::from(*name).render(),
                color: None,
                icon: None,
            })
            .collect();
        let choices = self.new_choices(id, &names)?;
        let definitions = self.field_catalogue().await?;
        let kind = FieldType::PickOne;
        Ok(Definition {
            field: OwnField {
                id,
                name: Message::from(starter.name()).render(),
                help: String::new(),
                kind,
                applies_to: FieldFor::TimeEntries,
                rules: Rules::default(),
                default_value: String::new(),
                look: Look::starting(kind),
                never_to_assistant: false,
                position: next_position(&definitions),
                retired_at: None,
                version: FIRST_VERSION,
            },
            scope: Scope::default(),
            choices,
        })
    }
}

//! The starter fields of time entries, *Activity* and *Cost center*, as the sample and the large
//! dataset fill them in: found by their marks, never by their names, with the IDs of their choices
//! taken from the fields. *Cost center* starts empty, so the choices of the plan are added first,
//! through the use case of the screen. Written once for both (coding standards §1).

use bb_app::fields::{AddChoice, FieldQuery, add_choice, field};
use bb_app::time::TimeStore as _;
use bb_core::id::{ChoiceId, FieldId};
use bb_core::time::entry::StarterField;
use bb_dev_words::sample::CostCenterPlan;
use serde_json::Value;

use crate::error::{SampleError, Step};
use crate::generate::World;
use crate::issues_setup::{id_in, json};

/// A starter field with the IDs of its choices.
#[derive(Clone, Debug)]
pub(crate) struct Starter {
    pub(crate) field: FieldId,
    pub(crate) choices: Vec<ChoiceId>,
}

/// *Activity* and *Cost center*.
#[derive(Clone, Debug)]
pub(crate) struct StarterFields {
    pub(crate) activity: Starter,
    pub(crate) cost_center: Starter,
}

fn choices_of(view: &Value) -> Result<Vec<ChoiceId>, SampleError> {
    view["choices"]
        .as_array()
        .map_or(&[][..], Vec::as_slice)
        .iter()
        .map(|choice| id_in(choice, "id"))
        .collect()
}

/// The two starter fields, with `cost_centers` (each a number and a name) added as choices of *Cost center*, by the
/// administrator whose session is `token`.
pub(crate) async fn starter_fields(
    world: &World<'_>,
    token: &str,
    cost_centers: &[CostCenterPlan],
) -> Result<StarterFields, SampleError> {
    let marks = world
        .storage
        .starter_fields()
        .await
        .map_err(|error| SampleError::step(Step::Time, &error))?;
    let mark = |starter: StarterField| -> Result<FieldId, SampleError> {
        marks
            .iter()
            .find(|found| found.starter == starter)
            .map(|found| found.field)
            .ok_or_else(|| SampleError::NotMade(starter.code().to_owned()))
    };
    let (activity, cost_center) = (
        mark(StarterField::Activity)?,
        mark(StarterField::CostCenter)?,
    );
    let ctx = world.services.ctx(world.request());
    let read = |field_id: FieldId| FieldQuery {
        session_token: token.to_owned(),
        field: field_id,
    };
    let mut view = json(field(&ctx, read(cost_center)).await, Step::Time)?;
    for center in cost_centers {
        world.tick();
        view = json(
            add_choice(
                &ctx,
                AddChoice {
                    session_token: token.to_owned(),
                    field: cost_center,
                    version: view["version"].as_i64().unwrap_or(1),
                    name: center.name.clone(),
                    number: Some(center.number.clone()),
                    color: None,
                    icon: None,
                },
            )
            .await,
            Step::Time,
        )?;
    }
    let activity_view = json(field(&ctx, read(activity)).await, Step::Time)?;
    Ok(StarterFields {
        activity: Starter {
            field: activity,
            choices: choices_of(&activity_view)?,
        },
        cost_center: Starter {
            field: cost_center,
            choices: choices_of(&view)?,
        },
    })
}

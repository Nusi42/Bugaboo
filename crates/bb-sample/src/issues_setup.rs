//! What the issues of the sample stand on: the starting issue types and kinds of relation, how
//! each project gives out its IDs, the own fields, and the groups of labels with their labels,
//! made through the use cases the screens call, as the first administrator.
//!
//! The answers of the use cases are read as the JSON a client would get (`masked_value`), so the
//! sample needs no access to the inside of a view.

use std::collections::BTreeMap;
use std::str::FromStr;

use bb_app::access::{Authorized, Masked};
use bb_app::auth::AppError;
use bb_app::fields::{CreateField, NewChoice, create_field};
use bb_app::issues::{IssueTypesQuery, issue_types};
use bb_app::projects::ProjectStore as _;
use bb_app::relations::{RelationKindsQuery, relation_kinds};
use bb_app::settings::{ChangeSetting, SettingsScope, change_setting};
use bb_app::tags::{
    CreateLabel, CreateLabelGroup, TagStore as _, create_label, create_label_group,
};
use bb_core::id::{ChoiceId, FieldId, IssueTypeId, LabelId, ProjectId, RelationKindId};
use bb_core::own_fields::{FieldFor, FieldType, Rules, Scope};
use bb_core::paging::PAGE_SIZE_MAX;
use bb_core::settings::{ISSUES_ID_MODE, SettingValue};
use bb_dev_words::sample::{FieldPlan, IssuePlan};
use serde::Serialize;
use serde_json::Value;

use crate::error::{SampleError, Step};
use crate::generate::World;

/// An own field that was made, with the IDs of its choices in the order of the plan.
#[derive(Clone, Debug)]
pub(crate) struct MadeField {
    pub(crate) id: FieldId,
    pub(crate) kind: FieldType,
    pub(crate) choices: Vec<ChoiceId>,
}

/// The IDs the sample needs of what it set up, in the order of the plan.
#[derive(Debug)]
pub(crate) struct Setup {
    /// The issue types, in the order of `types` of the plan.
    pub(crate) types: Vec<IssueTypeId>,
    /// The kinds of relation, in the order they are offered in.
    pub(crate) kinds: Vec<RelationKindId>,
    /// The own fields, in the order of `fields` of the plan.
    pub(crate) fields: Vec<MadeField>,
    /// The labels, in the order of [`crate::fields::labels`].
    pub(crate) labels: Vec<LabelId>,
}

/// An answer of a use case as the JSON a client would get.
pub(crate) fn json<T: Masked + Serialize>(
    answer: Result<Authorized<T>, AppError>,
    step: Step,
) -> Result<Value, SampleError> {
    answer
        .map_err(|error| SampleError::step(step, &error))?
        .masked_value()
        .map_err(|error| SampleError::step(step, &error))
}

/// The ID that a member of a JSON object holds.
pub(crate) fn id_in<T: FromStr>(value: &Value, member: &str) -> Result<T, SampleError> {
    value[member]
        .as_str()
        .and_then(|text| text.parse().ok())
        .ok_or_else(|| SampleError::NotMade(member.to_owned()))
}

impl Setup {
    /// Sets up what the issues stand on, as the administrator whose session is `token`.
    pub(crate) async fn create(
        world: &World<'_>,
        token: &str,
        names: &IssuePlan,
    ) -> Result<Self, SampleError> {
        let ctx = world.services.ctx(world.request());
        let step = Step::IssueTypes;

        let found = json(
            issue_types(
                &ctx,
                IssueTypesQuery {
                    project: None,
                    session_token: token.to_owned(),
                    cursor: None,
                    limit: PAGE_SIZE_MAX,
                },
            )
            .await,
            step,
        )?;
        let mut types = Vec::new();
        for share in &names.types {
            let issue_type = found["items"]
                .as_array()
                .and_then(|items| items.iter().find(|item| item["name"] == share.name))
                .ok_or_else(|| SampleError::NotMade(share.name.clone()))?;
            types.push(id_in(issue_type, "id")?);
        }
        let found = json(
            relation_kinds(
                &ctx,
                RelationKindsQuery {
                    session_token: token.to_owned(),
                    cursor: None,
                    limit: PAGE_SIZE_MAX,
                },
            )
            .await,
            step,
        )?;
        let kinds = found["items"]
            .as_array()
            .map_or(&[][..], Vec::as_slice)
            .iter()
            .map(|kind| id_in(kind, "id"))
            .collect::<Result<Vec<_>, _>>()?;

        for project in &names.projects {
            if project.id_mode != "random" {
                world.tick();
                change_setting(
                    &ctx,
                    ChangeSetting {
                        session_token: token.to_owned(),
                        scope: SettingsScope::ProjectAt(project.short_name.clone()),
                        key: ISSUES_ID_MODE.definition().key().to_owned(),
                        value: SettingValue::Text(project.id_mode.clone()),
                    },
                )
                .await
                .map_err(|error| SampleError::step(Step::Settings, &error))?;
            }
        }
        let projects = project_ids(world).await?;
        let mut fields = Vec::new();
        for plan in &names.fields {
            world.tick();
            fields.push(create_one_field(world, token, plan, names, &types, &projects).await?);
        }
        let labels = create_labels(world, token, names).await?;
        crate::own_workflow::create(world, token, &names.work.own_workflow).await?;
        Ok(Self {
            types,
            kinds,
            fields,
            labels,
        })
    }
}

/// The IDs of the projects, by short name.
async fn project_ids(world: &World<'_>) -> Result<BTreeMap<String, ProjectId>, SampleError> {
    Ok(world
        .storage
        .project_tree()
        .await?
        .into_iter()
        .map(|project| (project.short_name.to_string(), project.id))
        .collect())
}

/// Makes one own field for issues, used where the plan says.
async fn create_one_field(
    world: &World<'_>,
    token: &str,
    plan: &FieldPlan,
    names: &IssuePlan,
    types: &[IssueTypeId],
    projects: &BTreeMap<String, ProjectId>,
) -> Result<MadeField, SampleError> {
    let kind = FieldType::from_code(&plan.kind)
        .ok_or_else(|| SampleError::Layout(format!("the field kind {} is unknown", plan.kind)))?;
    let mut scope = Scope::default();
    for name in &plan.issue_types {
        let at = names
            .types
            .iter()
            .position(|share| &share.name == name)
            .and_then(|at| types.get(at))
            .ok_or_else(|| SampleError::NotMade(name.clone()))?;
        scope.issue_types.push(*at);
    }
    for name in &plan.projects {
        let project = projects
            .get(name)
            .ok_or_else(|| SampleError::NotMade(name.clone()))?;
        scope.projects.push(*project);
    }
    let made = json(
        create_field(
            &world.services.ctx(world.request()),
            CreateField {
                session_token: token.to_owned(),
                name: plan.name.clone(),
                help: None,
                kind,
                applies_to: FieldFor::Issues,
                rules: Rules::default(),
                default_value: None,
                look: None,
                never_to_assistant: false,
                scope,
                choices: plan
                    .choices
                    .iter()
                    .map(|name| NewChoice {
                        name: name.clone(),
                        color: None,
                        icon: None,
                    })
                    .collect(),
            },
        )
        .await,
        Step::Fields,
    )?;
    let choices = made["choices"]
        .as_array()
        .map_or(&[][..], Vec::as_slice)
        .iter()
        .map(|choice| id_in(choice, "id"))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(MadeField {
        id: id_in(&made, "id")?,
        kind,
        choices,
    })
}

/// Makes the groups of labels and the labels, in the order of [`crate::fields::labels`]. The
/// answers of these use cases are not masked views, so the IDs are looked up by name.
async fn create_labels(
    world: &World<'_>,
    token: &str,
    names: &IssuePlan,
) -> Result<Vec<LabelId>, SampleError> {
    let ctx = world.services.ctx(world.request());
    let step = |error: AppError| SampleError::step(Step::Labels, &error);
    for group in &names.label_groups {
        world.tick();
        create_label_group(
            &ctx,
            CreateLabelGroup {
                project: None,
                session_token: token.to_owned(),
                name: group.name.clone(),
                pick_one: group.pick_one,
            },
        )
        .await
        .map_err(step)?;
        let groups = world.storage.label_groups().await?;
        let id = groups
            .iter()
            .find(|found| found.name == group.name)
            .map(|found| found.id)
            .ok_or_else(|| SampleError::NotMade(group.name.clone()))?;
        for label in &group.labels {
            world.tick();
            create_label(
                &ctx,
                CreateLabel {
                    session_token: token.to_owned(),
                    name: label.clone(),
                    color: None,
                    group: Some(id),
                },
            )
            .await
            .map_err(step)?;
        }
    }
    for label in &names.labels {
        world.tick();
        create_label(
            &ctx,
            CreateLabel {
                session_token: token.to_owned(),
                name: label.clone(),
                color: None,
                group: None,
            },
        )
        .await
        .map_err(step)?;
    }
    let made = world.storage.labels().await?;
    crate::fields::labels(names)
        .iter()
        .map(|wanted| {
            made.iter()
                .find(|found| found.name == wanted.name)
                .map(|found| found.id)
                .ok_or_else(|| SampleError::NotMade(wanted.name.clone()))
        })
        .collect()
}

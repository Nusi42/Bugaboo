//! The scenarios of the plan of the timeline for the table of events: setting the time factor and
//! accepting a plan warning, each one event about the issue. And the rule of the tool registry: no
//! tool changes the time factor, and an agent may not set it.

use bb_core::plan::warning::WarningKind;
use bb_core::time::Minutes;

use super::scenarios::{Added, a_project, measured, text, token};
use super::scenarios_issues::a_type;
use super::world::World;
use crate::issues::{CreateIssue, create_issue};
use crate::plan::{AcceptWarning, FactorGiven, SetTimeFactor, accept_warning, set_time_factor};

/// An issue in a new project, assigned to the administrator, with an estimate of a day.
async fn an_assigned_issue(world: &World) -> Result<String, String> {
    let project = a_project(world, "PLAN").await?;
    Ok(create_issue(
        &world.ctx(),
        CreateIssue {
            session_token: token(world),
            project: project.to_string(),
            issue_type: a_type(world).await?,
            title: "Login screen".to_owned(),
            description: None,
            status: None,
            priority: None,
            assigned_to: Some(vec![world.admin.person.id]),
            start_date: Some(jiff::civil::date(2027, 3, 1)),
            due_date: None,
            estimated_time: Some(Minutes::new(480)),
            fields: std::collections::BTreeMap::new(),
            parent: None,
        },
    )
    .await
    .map_err(text)?
    .into_value()
    .id
    .to_string())
}

pub(crate) async fn set_time_factor_scenario(world: &World) -> Added {
    let issue = an_assigned_issue(world).await?;
    measured(
        &world.store,
        set_time_factor(
            &world.ctx(),
            SetTimeFactor {
                session_token: token(world),
                issue,
                version: 1,
                given: FactorGiven::Percent(150),
            },
        ),
    )
    .await
}

pub(crate) async fn accept_warning_scenario(world: &World) -> Added {
    let issue = an_assigned_issue(world).await?;
    measured(
        &world.store,
        accept_warning(
            &world.ctx(),
            AcceptWarning {
                session_token: token(world),
                issue,
                version: 1,
                // Taken as it is said: it depends on the other work on the screen.
                kind: WarningKind::TooMuchAtOnce,
                options: bb_core::saved_search::TimelineOptions::default(),
            },
        ),
    )
    .await
}

#[test]
fn plan_14_no_tool_of_the_registry_changes_the_time_factor() {
    use crate::ai::tools::{DESIGNED_OUT, registry};
    use bb_core::fields::Field;
    // The use case is designed out of the registry, and no tool runs it or names the field in what
    // it takes: the assistant, AI apps and agents have no way to set it (ADR-0030, amendment M8).
    assert!(DESIGNED_OUT.contains(&"set_time_factor"));
    let words = [
        Field::TimeFactor.code().to_owned(),
        Field::TimeFactor.code().replace('-', "_"),
        Field::TimeFactor.search_word(),
    ];
    for spec in registry::<super::fake::FakeStore>() {
        let tool = spec.tool;
        assert!(!DESIGNED_OUT.contains(&spec.use_case), "{tool:?}");
        let schema = spec.input_schema().to_string();
        for word in &words {
            assert!(
                !schema.contains(word.as_str()),
                "{tool:?} takes {word}: {schema}"
            );
        }
    }
}

/// The preview of *Move it after* for an issue, as the administrator, after 4 March 2027.
async fn preview(
    world: &World,
    issue: String,
) -> Result<bb_policy::Authorized<crate::plan::MovePreviewPage>, crate::auth::AppError> {
    let ctx = world.ctx();
    crate::plan::move_after_preview(
        &ctx,
        crate::plan::MoveAfterQuery {
            session_token: token(world),
            issue,
            after: jiff::civil::date(2027, 3, 4),
            options: bb_core::saved_search::TimelineOptions::default(),
        },
    )
    .await
}

#[tokio::test]
async fn plan_13_a_preview_of_move_it_after_stops_at_the_size_of_one_run() -> super::world::Outcome
{
    use bb_core::id::{IssueId, RelationId, UserId};
    use bb_core::many_changes::MANY_CHANGES_JOB_ABOVE;
    use bb_core::relation::Relation;
    use jiff::Timestamp;

    use crate::relations::{RelationKindsQuery, relation_kinds};

    let world = World::new().await?;
    let issue = an_assigned_issue(&world).await?;
    let kinds = relation_kinds(
        &world.ctx(),
        RelationKindsQuery {
            session_token: token(&world),
            cursor: None,
            limit: 50,
        },
    )
    .await?
    .into_value();
    // The starting kind *blocks* has the date effect.
    let blocks = kinds.items.first().ok_or("no kind")?.id;
    let first: IssueId = issue.parse().map_err(|_| "no issue ID")?;
    // Before the chain grows the issue has a bar and moves.
    assert_eq!(
        preview(&world, issue.clone())
            .await?
            .into_value()
            .issues
            .len(),
        1
    );
    {
        // A chain of one more issue than a run takes waits for the issue.
        let ctx = world.ctx();
        let mut ids = ctx.ids();
        let mut shared = world.store.lock();
        let mut before = first;
        for _ in 0..=MANY_CHANGES_JOB_ABOVE {
            let then = IssueId::generate(&mut ids)?;
            shared.state.relations.push(Relation {
                id: RelationId::generate(&mut ids)?,
                kind: blocks,
                from: before,
                to: then,
                added_by: UserId::generate(&mut ids),
                added_at: Timestamp::UNIX_EPOCH,
            });
            before = then;
        }
    }
    let refused = preview(&world, issue).await;
    assert!(
        matches!(
            refused,
            Err(crate::auth::AppError::Refused(
                bb_core::ErrorCode::InvalidRequest
            ))
        ),
        "{refused:?}"
    );
    Ok(())
}

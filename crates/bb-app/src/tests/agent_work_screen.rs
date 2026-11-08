//! What the screens of agent work read (AI-31, AI-33, ADR-0029): the brief that is told before a
//! hand-over, the agent and the run of an attempt on the work, and the mark of a row of a list.

use bb_core::agent_work::{AgentWorkState, BriefPart};

use super::agent_work::{handed, query};
use super::search_run::hit;
use super::world::{Outcome, World};
use crate::agent_work::{AgentWorkQuery, issue_agent_work};
use crate::issues::IssueStore as _;
use crate::issues::{IssuesQuery, issues};

fn ask(session: &str, issue: &impl ToString) -> AgentWorkQuery {
    AgentWorkQuery {
        session_token: session.to_owned(),
        issue: issue.to_string(),
    }
}

fn listing(session: &str, project: &impl ToString, agent: bool) -> IssuesQuery {
    IssuesQuery {
        session_token: session.to_owned(),
        project: project.to_string(),
        assigned_to: false,
        labels: false,
        agent: agent.into(),
        time: false,
        cursor: None,
        limit: 10,
    }
}

#[tokio::test]
async fn ai_31_the_brief_is_told_before_anything_is_handed_over() -> Outcome {
    let world = World::new().await?;
    let given = handed(&world).await?;
    let work = issue_agent_work(&world.ctx(), ask(&given.team.session_token, &given.issue))
        .await?
        .into_value();
    // The issue has no description, steps or *Done when*: the window can name all three.
    assert_eq!(
        work.brief_missing,
        [
            BriefPart::Description,
            BriefPart::StepsToReproduce,
            BriefPart::DoneWhen
        ]
    );
    Ok(())
}

#[tokio::test]
async fn ai_33_the_work_names_its_agent_the_person_and_who_handed_it_over() -> Outcome {
    let world = World::new().await?;
    let given = handed(&world).await?;
    let work = issue_agent_work(&world.ctx(), ask(&given.team.session_token, &given.issue))
        .await?
        .into_value();
    let piece = &work.work[0];
    assert_eq!(piece.agent_name, "Fixer");
    assert_eq!(piece.agent_person, piece.handed_over_by);
    assert_eq!(piece.budget.attempts, 5);
    assert!(!piece.budget.plan_needs_approval);
    assert_eq!(piece.ends_at, given.hand_over.ends_at);
    Ok(())
}

#[tokio::test]
async fn ai_33_a_list_row_names_the_agent_and_its_state_only_when_asked() -> Outcome {
    let world = World::new().await?;
    let given = handed(&world).await?;
    let found = world
        .store
        .issue(given.issue)
        .await
        .map_err(|error| error.to_string())?
        .ok_or("the issue is there")?;
    world.store.lock().search_hits = vec![hit(found)];
    let project = given.project;
    let session = &given.team.session_token;
    let mut with = issues(&world.ctx(), listing(session, &project, true))
        .await?
        .into_value();
    let mark = with
        .items
        .remove(0)
        .into_value()
        .agent_work
        .ok_or("a mark")?;
    assert_eq!(mark.name, "Fixer");
    assert_eq!(mark.state, AgentWorkState::WaitingForTheAgent);
    // Left out unless asked for, and the issue itself is as it was.
    let mut without = issues(&world.ctx(), listing(session, &project, false))
        .await?
        .into_value();
    assert_eq!(without.items.remove(0).into_value().agent_work, None);
    let _ = query(session, given.issue);
    Ok(())
}

#[tokio::test]
async fn ai_30_a_reader_not_told_who_people_are_sees_someone_behind_an_agent() -> Outcome {
    use bb_core::access::PeopleLevel;
    use bb_core::people_kinds::PeopleKind;
    use bb_policy::{PersonRef, StartingRole};

    use super::scenarios::token;
    use super::workflows_rules::{member, starting_role};
    use crate::roles::{SetPeopleLevel, set_level};

    let world = World::new().await?;
    let given = handed(&world).await?;
    let reader = member(
        &world,
        "reader@example.test",
        StartingRole::Viewer,
        given.project,
    )
    .await?;
    let role = starting_role(&world, StartingRole::Viewer)
        .await
        .map_err(crate::auth::AppError::Internal)?;
    set_level(
        &world.ctx(),
        SetPeopleLevel {
            session_token: token(&world),
            role,
            project: None,
            kind: PeopleKind::ProjectMembers,
            level: Some(PeopleLevel::Hidden),
        },
    )
    .await?;
    let work = issue_agent_work(&world.ctx(), ask(&reader.session_token, &given.issue))
        .await?
        .into_value();
    // Neither the person the agent works for nor the person who handed over is told.
    assert_eq!(work.work[0].agent_person, PersonRef::Someone);
    assert_eq!(work.work[0].handed_over_by, PersonRef::Someone);
    assert_eq!(work.worked_on_by[0].person, PersonRef::Someone);
    Ok(())
}

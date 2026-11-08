//! The limits of an agent (AI-30, AI-37, ADR-0029 decision 9): its person changes them within what the
//! organization allows.

use bb_core::ErrorCode;
use bb_core::agent::AgentLimits;

use super::agents::{an_agent, refused};
use super::scenarios::{Added, a_project, measured, text, token};
use super::world::{Outcome, World};
use crate::agents::{AgentQuery, SetAgentLimits, agent, set_agent_limits};

fn limits(changes: i64, issues: i64, attempts: i64) -> AgentLimits {
    AgentLimits {
        changes_a_minute: changes,
        issues_at_once: issues,
        attempts_on_one_issue: attempts,
    }
}

fn asking(
    session: &str,
    id: bb_core::id::AgentId,
    to: AgentLimits,
    version: i64,
) -> SetAgentLimits {
    SetAgentLimits {
        session_token: session.to_owned(),
        id,
        limits: to,
        version,
    }
}

pub(crate) async fn set_agent_limits_scenario(world: &World) -> Added {
    a_project(world, "WEB").await?;
    let fixer = an_agent(world, &token(world), "Fixer")
        .await
        .map_err(text)?;
    measured(
        &world.store,
        set_agent_limits(
            &world.ctx(),
            asking(&token(world), fixer, limits(10, 2, 3), 1),
        ),
    )
    .await
}

#[tokio::test]
async fn ai_37_a_person_changes_the_limits_of_an_agent_within_the_maximum() -> Outcome {
    let world = World::new().await?;
    a_project(&world, "WEB").await?;
    let session = token(&world);
    let fixer = an_agent(&world, &session, "Fixer").await?;
    let before = agent(
        &world.ctx(),
        AgentQuery {
            session_token: session.clone(),
            id: fixer,
        },
    )
    .await?
    .into_value();
    let most = before.maximums.ok_or("a single agent shows the maximums")?;
    let changed = set_agent_limits(
        &world.ctx(),
        asking(&session, fixer, limits(10, 2, 3), before.version),
    )
    .await?
    .into_value();
    assert_eq!(changed.limits, limits(10, 2, 3));
    assert_eq!(changed.version, before.version + 1);
    // Not below one, and not above what the organization allows.
    for bad in [
        limits(0, 2, 3),
        limits(10, 0, 3),
        limits(10, 2, 0),
        limits(most.changes_a_minute + 1, 2, 3),
        limits(10, most.issues_at_once + 1, 3),
        limits(10, 2, most.attempts_on_one_issue + 1),
    ] {
        let refusal = set_agent_limits(&world.ctx(), asking(&session, fixer, bad, changed.version))
            .await
            .unwrap_err();
        assert!(refused(&refusal, ErrorCode::InvalidRequest), "{bad:?}");
    }
    // A change on an older version is stopped.
    let stale = set_agent_limits(&world.ctx(), asking(&session, fixer, limits(5, 2, 3), 1)).await;
    assert!(stale.is_err());
    Ok(())
}

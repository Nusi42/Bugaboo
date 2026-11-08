//! The list of every agent of the organization (AI-30, M7-F224): working agents first, then paused
//! ones, then stopped ones, by name inside each group, whatever the size of a page.

use bb_core::agent::AgentState;
use bb_core::id::AgentId;

use super::agents::{an_agent, page};
use super::scenarios::{a_project, token};
use super::world::{Outcome, World};
use crate::agents::{AgentAction, AgentsQuery, ChangeAgent, change_agent, organization_agents};

/// Changes the state of an agent that was not changed before.
async fn act(world: &World, id: AgentId, action: AgentAction) -> Outcome {
    change_agent(
        &world.ctx(),
        ChangeAgent {
            session_token: token(world),
            id,
            action,
            version: 1,
        },
    )
    .await?;
    Ok(())
}

/// The names of every page of the list, `size` agents at a time, with `between` run after the
/// first page has been read.
async fn names_by_pages(
    world: &World,
    size: u32,
    between: impl AsyncFnOnce() -> Outcome,
) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
    let mut between = Some(between);
    let mut names = Vec::new();
    let mut cursor = None;
    loop {
        let read = organization_agents(
            &world.ctx(),
            AgentsQuery {
                cursor,
                limit: size,
                ..page(&token(world))
            },
        )
        .await?
        .into_value();
        names.extend(read.items.iter().map(|agent| agent.name.clone()));
        if let Some(run) = between.take() {
            run().await?;
        }
        cursor = read.next_cursor;
        if cursor.is_none() {
            return Ok(names);
        }
    }
}

/// Seven agents in every state, made in an order that is neither the order of names nor of states.
async fn agents_in_all_states(
    world: &World,
) -> Result<Vec<AgentId>, Box<dyn std::error::Error + Send + Sync>> {
    a_project(world, "WEB").await?;
    let admin = token(world);
    let mut ids = Vec::new();
    for name in [
        "Echo", "Alpha", "Delta", "Bravo", "Foxtrot", "Charlie", "Golf",
    ] {
        ids.push(an_agent(world, &admin, name).await?);
    }
    // Echo and Alpha are stopped, Delta and Foxtrot are paused; Bravo, Charlie and Golf work.
    for index in [0, 1] {
        act(world, ids[index], AgentAction::Stop).await?;
    }
    for index in [2, 4] {
        act(world, ids[index], AgentAction::Pause).await?;
    }
    Ok(ids)
}

#[tokio::test]
async fn ai_30_the_list_of_the_organization_puts_working_agents_first_on_every_page_size() -> Outcome
{
    let world = World::new().await?;
    agents_in_all_states(&world).await?;
    let whole = [
        "Bravo", "Charlie", "Golf", "Delta", "Foxtrot", "Alpha", "Echo",
    ];
    for size in [1, 2, 3, 7, 50] {
        let names = names_by_pages(&world, size, async || Ok(())).await?;
        assert_eq!(names, whole, "pages of {size}");
    }
    Ok(())
}

#[tokio::test]
async fn ai_30_the_list_of_the_organization_stays_whole_when_an_agent_changes_between_pages()
-> Outcome {
    let world = World::new().await?;
    let ids = agents_in_all_states(&world).await?;
    // After the first page (Bravo, Charlie) Bravo is stopped: nobody is skipped; Bravo, who moved on
    // to the stopped agents, is read once more there (the page shows the agent once, the last time).
    let names = names_by_pages(&world, 2, async || {
        act(&world, ids[3], AgentAction::Stop).await
    })
    .await?;
    assert_eq!(
        names,
        [
            "Bravo", "Charlie", "Golf", "Delta", "Foxtrot", "Alpha", "Bravo", "Echo"
        ]
    );
    let every = organization_agents(&world.ctx(), page(&token(&world)))
        .await?
        .into_value();
    assert_eq!(
        every
            .items
            .iter()
            .map(|agent| agent.state)
            .collect::<Vec<_>>(),
        [
            AgentState::Active,
            AgentState::Active,
            AgentState::Paused,
            AgentState::Paused,
            AgentState::Stopped,
            AgentState::Stopped,
            AgentState::Stopped
        ]
    );
    Ok(())
}

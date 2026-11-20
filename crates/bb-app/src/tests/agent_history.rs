//! The history of an agent (AI-37, ADR-0029, amendment M9): what it handed over and what it undid,
//! newest first, a page at a time, for its person.

use bb_core::ErrorCode;
use bb_core::agent::AgentSwitches;
use bb_core::event::{Actor, Channel, DomainEvent, EntityKind, EntityRef, RecordKind};
use bb_core::id::EntityId;
use serde_json::json;

use super::agent_tools::used;
use super::agent_undo::{handed_title, retitled_by_agent, undo_input};
use super::agents::{an_agent, refused};
use super::agents_switches::switched;
use super::world::{Outcome, World};
use crate::agents::{AgentDeed, AgentHistoryPage, AgentHistoryQuery, agent_history};
use crate::auth::AppError;

/// The history of the agent as `session` reads it, from `cursor`, at most `limit` a page.
async fn history_of(
    world: &World,
    session: &str,
    agent: bb_core::id::AgentId,
    cursor: Option<String>,
    limit: u32,
) -> Result<AgentHistoryPage, AppError> {
    Ok(agent_history(
        &world.ctx(),
        AgentHistoryQuery {
            session_token: session.to_owned(),
            agent,
            cursor,
            limit,
        },
    )
    .await?
    .into_value())
}

#[tokio::test]
async fn ai_37_the_history_of_an_agent_lists_its_hand_overs_and_undos() -> Outcome {
    let world = World::with_ai_apps_and_agents().await?;
    let given = handed_title(&world).await?;
    let session = given.team.session_token.clone();
    let helper = an_agent(&world, &session, "Helper").await?;
    let both = AgentSwitches {
        may_hand_over: true,
        may_undo: true,
    };
    switched(&world, &session, given.agent, both).await?;
    assert!(
        history_of(&world, &session, given.agent, None, 50)
            .await?
            .items
            .is_empty()
    );
    retitled_by_agent(&world, &given, "A wrong title").await?;
    used(&world, &given.key, "undo_my_run", undo_input(&given)).await?;
    world.clock.advance(60);
    used(
        &world,
        &given.key,
        "hand_over_work",
        json!({ "agent": "Helper", "issues": [given.issue.to_string()] }),
    )
    .await?;

    let page = history_of(&world, &session, given.agent, None, 50).await?;
    assert_eq!(page.items.len(), 2, "{:?}", page.items);
    assert!(page.next_cursor.is_none());
    let AgentDeed::HandedOver { to, issues } = &page.items[0].deed else {
        return Err(format!("the newest is the hand-over: {:?}", page.items[0]).into());
    };
    assert_eq!(
        to.as_ref().map(|to| (to.id, to.name.as_str())),
        Some((helper, "Helper"))
    );
    assert_eq!(
        issues.iter().map(|line| line.id).collect::<Vec<_>>(),
        vec![given.issue]
    );
    let AgentDeed::Undid { issue, went_back } = &page.items[1].deed else {
        return Err(format!("then the undo: {:?}", page.items[1]).into());
    };
    assert_eq!(issue.as_ref().map(|line| line.id), Some(given.issue));
    assert_eq!(*went_back, 1);
    assert!(page.items[0].at >= page.items[1].at);

    // A page at a time: the cursor of the first leads to the second, and nothing is said twice.
    let first = history_of(&world, &session, given.agent, None, 1).await?;
    assert_eq!(first.items, page.items[..1]);
    let second = history_of(&world, &session, given.agent, first.next_cursor, 1).await?;
    assert_eq!(second.items, page.items[1..]);
    assert!(second.next_cursor.is_none());

    // What the helper was handed is not something it did.
    assert!(
        history_of(&world, &session, helper, None, 50)
            .await?
            .items
            .is_empty()
    );
    // An administrator, who sees every agent, does not read it: the runs are the person's.
    let admin = history_of(
        &world,
        &super::scenarios::token(&world),
        given.agent,
        None,
        50,
    )
    .await;
    assert!(refused(&admin.unwrap_err(), ErrorCode::NotFound));
    // The agent itself does not read it: its key is no session. A cursor that is none is refused.
    let by_agent = history_of(&world, &given.key, given.agent, None, 50).await;
    assert!(refused(
        &by_agent.unwrap_err(),
        ErrorCode::AccessKeyNotAllowed
    ));
    let bad = history_of(&world, &session, given.agent, Some("x".to_owned()), 50).await;
    assert!(refused(&bad.unwrap_err(), ErrorCode::InvalidRequest));
    Ok(())
}

/// Hand-overs and undos of one moment, more of each than a page holds: the pages together hold each
/// once, in the one order of both kinds (newest first, then by ID).
#[tokio::test]
async fn ai_37_the_pages_of_the_history_hold_every_entry_of_one_moment_once() -> Outcome {
    let world = World::with_ai_apps_and_agents().await?;
    let given = handed_title(&world).await?;
    let session = given.team.session_token.clone();
    let helper = an_agent(&world, &session, "Helper").await?;
    let person = given.team.person.id;
    let moment = now_of(&world);
    let mut made = Vec::new();
    {
        let mut shared = world.store.lock();
        let template = shared
            .state
            .work
            .hand_overs
            .first()
            .cloned()
            .ok_or("no hand-over")?;
        for number in 1..=3_u8 {
            let mut hand_over = template.clone();
            hand_over.id = bb_core::id::HandOverId::from_random([number; 16]);
            hand_over.agent = helper;
            hand_over.by_agent = Some(given.agent);
            hand_over.made_at = moment;
            made.push(EntityId::from(hand_over.id));
            shared.state.work.hand_overs.push(hand_over);
        }
        for number in 11..=13_u8 {
            let event = DomainEvent {
                id: bb_core::id::EventId::from_parts(moment, [number; 10])?,
                time: moment,
                actor: Actor::Agent {
                    agent: given.agent,
                    person,
                },
                channel: Channel::Api,
                entity: EntityRef {
                    kind: EntityKind::from(RecordKind::ChangeRun),
                    id: EntityId::from(bb_core::id::RunId::from_parts(moment, [number; 10])?),
                },
                changes: vec![bb_core::event::FieldChange {
                    field: crate::runs::UNDONE.to_owned(),
                    before: None,
                    after: Some(json!(1)),
                }],
                reason: Some(bb_core::run::RUN_UNDONE.to_owned()),
                run: None,
            };
            made.push(EntityId::from(event.id));
            shared.events.push(event);
        }
    }
    let mut seen = Vec::new();
    let mut cursor = None;
    loop {
        let page = history_of(&world, &session, given.agent, cursor, 2).await?;
        assert!(page.items.len() <= 2);
        seen.extend(page.items.iter().map(|item| item.id));
        match page.next_cursor {
            Some(next) => cursor = Some(next),
            None => break,
        }
    }
    let mut each_once = seen.clone();
    each_once.sort_by_key(|id| id.to_bytes());
    each_once.dedup();
    assert_eq!(each_once.len(), seen.len(), "an entry came twice: {seen:?}");
    let mut wanted = made;
    wanted.sort_by_key(|id| id.to_bytes());
    assert_eq!(each_once, wanted);
    assert_eq!(seen, wanted, "one moment is in the order of the IDs");
    Ok(())
}

/// The moment of the clock of the test.
fn now_of(world: &World) -> jiff::Timestamp {
    crate::context::Clock::now(world.clock.as_ref())
}

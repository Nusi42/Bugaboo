//! An agent handed to another person on both databases (AI-31, ADR-0029, amendment M9): the
//! switches its first person chose go off with the hand-off, as for a new agent.

use bb_app::auth::AccountStore as _;
use bb_app::ports::ChangeStore;
use bb_core::agent::{Agent, AgentState, AgentSwitches};
use bb_core::id::AgentId;
use bb_testing::{TestResult, each_database};

use super::*;
use crate::test_support::{at, event, fixture, ids, person};

#[tokio::test]
async fn ai_31_the_switches_go_off_when_the_agent_is_handed_to_another_person() -> TestResult {
    each_database("ai_31_switches_off_on_hand_off", async |db| {
        let fixture = fixture(db).await?;
        let storage = fixture.storage.clone();
        let mut source = ids();
        let (ana, ben) = (
            person(&mut source, "ana@example.test"),
            person(&mut source, "ben@example.test"),
        );
        let fixer = Agent {
            id: AgentId::generate(&mut source),
            person: ana.id,
            name: "Fixer".to_owned(),
            state: AgentState::Active,
            paused_for: None,
            changes_a_minute: 60,
            issues_at_once: 5,
            attempts_on_one_issue: 3,
            may_hand_over: true,
            may_undo: true,
            offered_to: None,
            made_by: ana.id,
            made_at: at(1),
            version: 1,
        };
        let (made, people) = (fixer.clone(), [ana.clone(), ben.clone()]);
        ChangeStore::record_change(&storage, &event(&mut source, 1), move |tx| {
            Box::pin(async move {
                for each in &people {
                    AuthStorage::insert_person(tx, each, "hash", at(1)).await?;
                }
                AuthStorage::insert_agent(tx, &made).await
            })
        })
        .await?;
        let (id, to) = (fixer.id, ben.id);
        let handed = ChangeStore::record_change(&storage, &event(&mut source, 2), move |tx| {
            Box::pin(async move { AuthStorage::hand_agent_to(tx, id, to, 1).await })
        })
        .await?;
        assert!(handed, "the agent was handed on its version");
        let now = storage.agent(id).await?.ok_or("the agent is gone")?;
        assert_eq!(now.person, ben.id);
        assert_eq!(
            now.switches(),
            AgentSwitches::default(),
            "both switches are off"
        );
        Ok(())
    })
    .await
}

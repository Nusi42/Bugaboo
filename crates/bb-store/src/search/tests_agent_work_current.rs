//! The current work of agents in the store, on both databases: what waits for a person comes first,
//! the cut never takes it, and a hand-over that ran out gives no work.

use bb_app::agent_work::AgentWorkRecords as _;
use bb_core::agent_work::{AgentWorkState, HandOverPower};
use bb_testing::{TestResult, each_database};

use super::tests_agent_work::{ENDS, Given, Made, given, hand_over};
use crate::AuthStorage;
use crate::test_support::{at, event};
use bb_app::ports::ChangeStore as _;

/// Gives the work of an issue another state.
async fn put_in(
    storage: &AuthStorage,
    made: &mut Made,
    work: &bb_core::agent_records::IssueWork,
    state: AgentWorkState,
) -> Result<(), crate::StoreError> {
    made.step += 1;
    let step = made.step;
    let (id, version) = (work.id, work.version);
    storage
        .record_change(&event(&mut made.source, step), move |transaction| {
            Box::pin(async move {
                AuthStorage::set_work_state(transaction, id, state, None, false, version).await?;
                Ok(())
            })
        })
        .await
}

#[tokio::test]
async fn ai_37_the_current_work_lists_what_waits_for_a_person_first_and_the_cut_never_takes_it()
-> TestResult {
    each_database("ai_37_current_work", async |db| {
        let Given {
            fixture,
            data,
            owner,
            fixer,
        } = given(db).await?;
        let storage = &fixture.storage;
        let mut made = Made::new();
        let [a, b, c, d] = [0, 1, 2, 3].map(|at| data.issues[at].id);
        let mut works = Vec::new();
        for issue in [a, b, c, d] {
            let (_, made_works) = hand_over(
                storage,
                &mut made,
                (owner, fixer),
                &[issue],
                (vec![HandOverPower::ChangeFields], Vec::new()),
            )
            .await?;
            works.extend(made_works);
        }
        // The oldest waits for a person, the next two work or wait for the agent, the newest too.
        put_in(storage, &mut made, &works[0], AgentWorkState::WaitingForYou).await?;
        put_in(storage, &mut made, &works[1], AgentWorkState::AgentWorking).await?;
        let states = |found: &[bb_core::agent_records::IssueWork]| {
            found.iter().map(|work| work.id).collect::<Vec<_>>()
        };
        // The agent's own list: the newest first.
        let own = storage.current_work_of_agent(fixer, at(100), 10).await?;
        assert_eq!(own.len(), 4);
        // The list of the person: what waits for them first, then the newest; the cut takes the
        // rest and never the one that waits.
        let all = storage
            .current_work_of_agents(vec![fixer], owner, at(100), 10)
            .await?;
        assert_eq!(
            states(&all),
            [works[0].id, works[3].id, works[2].id, works[1].id]
        );
        let cut = storage
            .current_work_of_agents(vec![fixer], owner, at(100), 2)
            .await?;
        assert_eq!(states(&cut), [works[0].id, works[3].id]);
        // After the end of the hand-overs they give no work.
        assert!(
            storage
                .current_work_of_agents(vec![fixer], owner, at(ENDS + 1), 10)
                .await?
                .is_empty()
        );
        assert!(
            storage
                .current_work_of_agent(fixer, at(ENDS + 1), 10)
                .await?
                .is_empty()
        );
        Ok(())
    })
    .await
}

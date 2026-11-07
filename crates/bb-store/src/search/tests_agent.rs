//! *Agent* and *Agent state* in the search, on both databases (AI-33): found only among the issues
//! the reader sees, and only for work a review has not closed.

use bb_app::search::{FieldTarget, Searchable};
use bb_core::agent_work::AgentWorkState;
use bb_core::id::{AgentId, AgentWorkId, HandOverId, IssueId, UserId};
use bb_searchlang::{Field, FieldKind};
use bb_testing::{TestResult, each_database};
use sea_query::{Expr, Query};

use super::tests::{Data, request_with, seed};
use super::tests_more::titles_of;
use crate::AuthStorage;
use crate::db::StoredId as _;
use crate::schema::{AgentRow, IssueWorkRow};
use crate::test_support::{at, event, person};
use bb_app::auth::AccountStore as _;
use bb_app::ports::ChangeStore as _;

pub(super) fn agent_fields() -> Vec<Searchable> {
    vec![
        Searchable {
            field: Field::unordered("agent"),
            target: FieldTarget::Agent,
        },
        Searchable {
            field: Field::new("agent state", FieldKind::Choice).with_choices(
                &AgentWorkState::ALL
                    .iter()
                    .map(|state| state.code())
                    .collect::<Vec<_>>(),
            ),
            target: FieldTarget::AgentState,
        },
    ]
}

/// An agent of the person, stored.
pub(super) async fn an_agent(
    storage: &AuthStorage,
    person: UserId,
    name: &str,
    tag: u8,
) -> Result<AgentId, crate::StoreError> {
    let mut bytes = [0_u8; 16];
    bytes[15] = tag;
    let agent = AgentId::from_random(bytes);
    let mut insert = Query::insert();
    insert
        .into_table(AgentRow::Table)
        .columns([
            AgentRow::Id,
            AgentRow::Person,
            AgentRow::Name,
            AgentRow::State,
            AgentRow::ChangesAMinute,
            AgentRow::IssuesAtOnce,
            AgentRow::AttemptsOnOneIssue,
            AgentRow::OfferedTo,
            AgentRow::MadeBy,
            AgentRow::MadeAt,
            AgentRow::Version,
        ])
        .values_panic([
            agent.stored().into(),
            person.stored().into(),
            name.into(),
            "active".into(),
            60_i64.into(),
            5_i64.into(),
            5_i64.into(),
            Expr::val(None::<Vec<u8>>),
            person.stored().into(),
            0_i64.into(),
            1_i64.into(),
        ]);
    storage.database().execute(&insert).await?;
    Ok(agent)
}

/// Work of the agent on an issue in the given state.
/// A person of the tests, stored, whose agents can be stored.
pub(super) async fn a_stored_person(storage: &AuthStorage) -> Result<UserId, crate::StoreError> {
    let mut source = bb_core::id::SequentialIds::default();
    let made = person(&mut source, "owner@example.test");
    let stored = made.clone();
    storage
        .record_change(&event(&mut source, 5), move |tx| {
            Box::pin(async move { AuthStorage::insert_person(tx, &stored, "hash", at(0)).await })
        })
        .await?;
    Ok(made.id)
}

async fn work_on(
    storage: &AuthStorage,
    person: UserId,
    agent: AgentId,
    issue: IssueId,
    state: AgentWorkState,
    closed: bool,
) -> Result<(), crate::StoreError> {
    // Ids that differ for every piece of work: the bytes of the issue, turned.
    let turned = |by: u8| {
        let mut bytes = issue.to_stored();
        bytes[15] ^= by;
        bytes
    };
    let mut work = Query::insert();
    work.into_table(IssueWorkRow::Table)
        .columns([
            IssueWorkRow::Id,
            IssueWorkRow::Issue,
            IssueWorkRow::HandOver,
            IssueWorkRow::Agent,
            IssueWorkRow::Person,
            IssueWorkRow::State,
            IssueWorkRow::GaveUp,
            IssueWorkRow::Closed,
            IssueWorkRow::AcceptedByItself,
            IssueWorkRow::StatusPending,
            IssueWorkRow::MadeAt,
            IssueWorkRow::Version,
        ])
        .values_panic([
            AgentWorkId::from_random(turned(0x22)).stored().into(),
            issue.stored().into(),
            HandOverId::from_random(turned(0x33)).stored().into(),
            agent.stored().into(),
            person.stored().into(),
            state.code().into(),
            Expr::val(None::<String>),
            closed.into(),
            false.into(),
            false.into(),
            0_i64.into(),
            1_i64.into(),
        ]);
    storage.database().execute(&work).await?;
    Ok(())
}

async fn found(storage: &AuthStorage, data: &Data, text: &str) -> Vec<String> {
    titles_of(
        storage,
        &request_with(data, text, None, 100, agent_fields(), Vec::new()),
    )
    .await
    .unwrap_or_default()
}

#[tokio::test]
async fn ai_33_agent_state_waiting_for_you_finds_the_issue_only_for_who_may_see_it() -> TestResult {
    each_database("ai_33_agent_search", async |db| {
        let fixture = crate::test_support::fixture(db).await?;
        let storage = &fixture.storage;
        let data = seed(storage).await;
        let owner = a_stored_person(storage).await?;
        let fixer = an_agent(storage, owner, "Fixer", 1).await?;
        let helper = an_agent(storage, owner, "Helper", 2).await?;
        // Work on two issues the reader sees, one the review closed, and one they do not see.
        work_on(
            storage,
            owner,
            fixer,
            data.issues[0].id,
            AgentWorkState::WaitingForYou,
            false,
        )
        .await?;
        work_on(
            storage,
            owner,
            helper,
            data.issues[1].id,
            AgentWorkState::AgentWorking,
            false,
        )
        .await?;
        work_on(
            storage,
            owner,
            fixer,
            data.issues[2].id,
            AgentWorkState::WaitingForYou,
            true,
        )
        .await?;
        work_on(
            storage,
            owner,
            fixer,
            data.hidden.id,
            AgentWorkState::WaitingForYou,
            false,
        )
        .await?;
        assert_eq!(
            found(storage, &data, "agent state is \"waiting-for-you\"").await,
            ["Printer jams"]
        );
        assert_eq!(
            found(storage, &data, "agent is Fixer").await,
            ["Printer jams"]
        );
        assert_eq!(
            found(storage, &data, "agent is Helper").await,
            ["Login broken"]
        );
        assert_eq!(
            found(storage, &data, "agent is empty sort by title").await,
            ["Crash on save", "Docs typo", "Idea", "Slow page"]
        );
        Ok(())
    })
    .await
}

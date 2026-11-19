//! The requests for agents on both databases (ADR-0029, amendment M9): stored and read back with
//! their agents, answered only from the state they are in, the waiting ones of a key or grant
//! counted, and the old ones deleted.

use bb_app::ports::{BoxFuture, ChangeStore};
use bb_core::id::{AgentId, SequentialIds};
use bb_testing::{TestResult, each_database};

use super::*;
use crate::test_support::{at, event, fixture, ids};

async fn write<T: Send>(
    storage: &AuthStorage,
    source: &mut SequentialIds,
    change: impl for<'t> FnOnce(&'t mut Transaction) -> BoxFuture<'t, Result<T, StoreError>> + Send,
) -> Result<T, StoreError> {
    let event = event(source, 1);
    ChangeStore::record_change(storage, &event, change).await
}

fn request(source: &mut SequentialIds, person: UserId, asker: Asker, second: i64) -> AgentRequest {
    AgentRequest {
        id: AgentRequestId::generate(source).unwrap(),
        person,
        asked_by_key: match asker {
            Asker::Key(key) => Some(key),
            Asker::Grant(_) => None,
        },
        asked_by_grant: match asker {
            Asker::Grant(grant) => Some(grant),
            Asker::Key(_) => None,
        },
        asked_by_name: "Script".to_owned(),
        asked_from: None,
        reason: "A team".to_owned(),
        agents: vec![AskedAgent {
            name: "Planner".to_owned(),
            purpose: "plans".to_owned(),
            key_days: Some(30),
            agent: None,
        }],
        state: RequestState::Waiting,
        asked_at: at(second),
        answered_at: None,
    }
}

async fn insert(
    storage: &AuthStorage,
    source: &mut SequentialIds,
    request: AgentRequest,
) -> Result<(), StoreError> {
    write(storage, source, move |tx| {
        Box::pin(async move { AuthStorage::insert_agent_request(tx, &request).await })
    })
    .await
}

#[tokio::test]
async fn agent_requests_are_stored_and_read_back() -> TestResult {
    each_database("agent_requests_stored", async |db| {
        let fixture = fixture(db).await?;
        let storage = fixture.storage.clone();
        let mut source = ids();
        let person = UserId::generate(&mut source);
        let key = Asker::Key(AccessKeyId::generate(&mut source));
        let grant = Asker::Grant(GrantId::generate(&mut source));
        let first = request(&mut source, person, key, 10);
        let second = request(&mut source, person, grant, 20);
        insert(&storage, &mut source, first.clone()).await?;
        insert(&storage, &mut source, second.clone()).await?;
        assert_eq!(storage.agent_request(first.id).await?, Some(first.clone()));
        let listed = storage.agent_requests_of(person, at(0), 10).await?;
        assert_eq!(listed, vec![second.clone(), first.clone()], "newest first");
        assert_eq!(
            storage.agent_requests_of(person, at(15), 10).await?,
            vec![second.clone()]
        );
        // Approved from waiting, with its agent; not twice.
        let agent = AgentId::generate(&mut source);
        let mut agents = first.agents.clone();
        agents[0].agent = Some(agent);
        let to_store = agents.clone();
        let id = first.id;
        let approved = write(&storage, &mut source, move |tx| {
            Box::pin(async move {
                AuthStorage::answer_agent_request(
                    tx,
                    id,
                    RequestState::Waiting,
                    RequestState::Approved,
                    at(30),
                    &to_store,
                )
                .await
            })
        })
        .await?;
        assert!(approved);
        let to_store = agents.clone();
        let again = write(&storage, &mut source, move |tx| {
            Box::pin(async move {
                AuthStorage::answer_agent_request(
                    tx,
                    id,
                    RequestState::Waiting,
                    RequestState::Declined,
                    at(31),
                    &to_store,
                )
                .await
            })
        })
        .await?;
        assert!(!again);
        // Collected from approved: the moment of the answer stays the moment of the approval.
        let to_store = agents.clone();
        write(&storage, &mut source, move |tx| {
            Box::pin(async move {
                AuthStorage::answer_agent_request(
                    tx,
                    id,
                    RequestState::Approved,
                    RequestState::Collected,
                    at(40),
                    &to_store,
                )
                .await
            })
        })
        .await?;
        let read = storage.agent_request(id).await?.ok_or("gone")?;
        assert_eq!(read.state, RequestState::Collected);
        assert_eq!(read.answered_at, Some(at(30)));
        assert_eq!(read.agents[0].agent, Some(agent));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn the_waiting_requests_of_a_credential_are_counted() -> TestResult {
    each_database("agent_requests_counted", async |db| {
        let fixture = fixture(db).await?;
        let storage = fixture.storage.clone();
        let mut source = ids();
        let person = UserId::generate(&mut source);
        let key = Asker::Key(AccessKeyId::generate(&mut source));
        let other = Asker::Key(AccessKeyId::generate(&mut source));
        for asked in [10, 20, 30] {
            let made = request(&mut source, person, key, asked);
            insert(&storage, &mut source, made).await?;
        }
        let made = request(&mut source, person, other, 30);
        insert(&storage, &mut source, made).await?;
        assert_eq!(storage.waiting_requests_of(key, at(0)).await?, 3);
        assert_eq!(storage.waiting_requests_of(key, at(15)).await?, 2);
        assert_eq!(storage.waiting_requests_of(other, at(0)).await?, 1);
        // The old ones go, a step at a time.
        let deleted = write(&storage, &mut source, move |tx| {
            Box::pin(async move { AuthStorage::delete_old_agent_requests(tx, at(25), 1).await })
        })
        .await?;
        assert_eq!(deleted, 1);
        let deleted = write(&storage, &mut source, move |tx| {
            Box::pin(async move { AuthStorage::delete_old_agent_requests(tx, at(25), 10).await })
        })
        .await?;
        assert_eq!(deleted, 1);
        assert_eq!(storage.waiting_requests_of(key, at(0)).await?, 1);
        Ok(())
    })
    .await
}

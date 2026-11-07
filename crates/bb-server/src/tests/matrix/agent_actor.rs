//! The agent as an actor of the matrix (AI-30, ADR-0029): an agent with no work, whose person is
//! an *Administrator*, is refused on every route the way a person with nothing is, except that it
//! reads its own record. It runs apart from the record of the matrix: it adds nothing to the file,
//! and one more actor in that run would use up the room for fields and issue types that the
//! examples make.

use bb_app::agents::CreateAgent;
use bb_app::auth::CreateAccessKey;
use bb_core::access_key::KeyChoice;
use bb_core::id::ProjectId;
use bb_testing::{Failure, TestResult, each_database};

use super::actors::Actor;
use super::routes::run;
use crate::sign_in_tests::policy_rig::Member;
use crate::sign_in_tests::rig::{PASSWORD, Rig, Tries};

/// The key of a new agent of the person, which has no work: the secret.
pub(super) async fn an_agents_key(rig: &Rig, member: &Member) -> Result<String, Failure> {
    let agent = rig
        .api()
        .create_agent(
            Rig::info(),
            CreateAgent {
                session_token: member.session.clone(),
                name: "Agent of the matrix".to_owned(),
            },
        )
        .await
        .map_err(|error| error.to_string())?
        .into_value()
        .id;
    let made = rig
        .api()
        .create_access_key(
            Rig::info(),
            CreateAccessKey {
                session_token: member.session.clone(),
                password: PASSWORD.to_owned(),
                name: "Key of the agent".to_owned(),
                choice: KeyChoice::SeeAndChange,
                permissions: Vec::new(),
                projects: Vec::new(),
                expires_at: None,
                agent: Some(agent),
                renews: None,
            },
        )
        .await
        .map_err(|error| error.to_string())?
        .into_value();
    Ok(made.secret)
}

/// An agent with no work, whose person is an *Administrator*, is refused on every route the way a
/// person with nothing is, except that it reads its own record (AI-30, ADR-0029). It runs apart
/// from the record: it adds nothing to the file of the matrix, and one more actor in that run would
/// use up the room for fields and issue types that the examples make.
#[tokio::test]
async fn sec_01_an_agent_without_work_is_refused_on_every_route() -> TestResult {
    each_database("sec_matrix_agent_without_work", async |db| {
        let rig = Rig::start_with(db, Tries::Unreached).await?;
        run(&rig, &[Actor::AgentWithoutWork]).await?;
        Ok(())
    })
    .await
}

/// The key an actor comes with: an administrator's key that may only see, one for the sibling
/// project, or the key of an agent of theirs; the secret.
pub(super) async fn a_key(
    rig: &Rig,
    actor: Actor,
    member: &Member,
    sibling: ProjectId,
) -> Result<String, Failure> {
    if actor == Actor::AgentWithoutWork {
        return an_agents_key(rig, member).await;
    }
    let (choice, projects) = if actor == Actor::KeySeeOnly {
        (KeyChoice::SeeOnly, Vec::new())
    } else {
        (KeyChoice::Everything, vec![sibling])
    };
    let made = rig
        .api()
        .create_access_key(
            Rig::info(),
            CreateAccessKey {
                session_token: member.session.clone(),
                password: PASSWORD.to_owned(),
                name: actor.name(),
                choice,
                permissions: Vec::new(),
                projects,
                expires_at: None,
                agent: None,
                renews: None,
            },
        )
        .await
        .map_err(|error| error.to_string())?
        .into_value();
    Ok(made.secret)
}

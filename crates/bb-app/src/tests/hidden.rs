//! A hidden project and a missing one cost the server the same work (PRJ-01, UX §2.1): an answer
//! that took longer for one of them would tell them apart.

use bb_core::ErrorCode;

use super::world::{Outcome, World};
use crate::auth::AppError;
use crate::projects::{MakeProject, ProjectQuery, make_project, project};

/// The reads the store answers while `address` is asked for, and what the answer was.
async fn reads_for(
    world: &World,
    session: &str,
    address: &str,
) -> Result<(usize, Option<AppError>), AppError> {
    let before = world.store.reads();
    let answer = project(
        &world.ctx(),
        ProjectQuery {
            session_token: session.to_owned(),
            address: address.to_owned(),
        },
    )
    .await;
    Ok((world.store.reads() - before, answer.err()))
}

#[tokio::test]
async fn prj_01_a_hidden_project_and_a_missing_one_cost_the_same_reads() -> Outcome {
    let world = World::new().await?;
    let secret = make_project(
        &world.ctx(),
        MakeProject {
            session_token: world.admin.session_token.clone(),
            parent: None,
            short_name: "SECRET".to_owned(),
            name: "Secret".to_owned(),
        },
    )
    .await?
    .into_value();
    let bob = world.person("bob@example.test").await?.session_token;
    let missing_id = "0190f4c2-7a3e-7c1d-9b52-3d6e8f1a2b4c";
    let asked = [
        ("SECRET", "NOSUCH"),
        (&secret.id.to_string(), missing_id),
        ("SECRET", "not an address"),
    ];
    for (hidden, missing) in asked {
        let (hidden_reads, hidden_answer) = reads_for(&world, &bob, hidden).await?;
        let (missing_reads, missing_answer) = reads_for(&world, &bob, missing).await?;
        let not_found = Some(AppError::Refused(ErrorCode::NotFound));
        assert_eq!(hidden_answer, not_found, "{hidden}");
        assert_eq!(missing_answer, not_found, "{missing}");
        assert_eq!(
            hidden_reads, missing_reads,
            "{hidden} and {missing} read the store as often"
        );
    }
    Ok(())
}

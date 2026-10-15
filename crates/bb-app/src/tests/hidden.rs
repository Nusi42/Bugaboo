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

#[tokio::test]
async fn prj_01_a_person_who_may_only_make_sub_projects_in_a_project_they_cannot_see_finds_it_not_there()
-> Outcome {
    use bb_core::access::AppliesTo;
    use bb_policy::Permission;

    use crate::roles::{AssignRole, Given, MakeRole, Subject, assign_role, make_role};

    let world = World::new().await?;
    let admin = world.admin.session_token.clone();
    let secret = make_project(
        &world.ctx(),
        MakeProject {
            session_token: admin.clone(),
            parent: None,
            short_name: "SECRET".to_owned(),
            name: "Secret".to_owned(),
        },
    )
    .await?
    .into_value();
    // A role that only lets them make sub-projects, given in the project: it does not let them
    // see it, so the project is not there for them, as for anybody who does not see it.
    let only = make_role(
        &world.ctx(),
        MakeRole {
            session_token: admin.clone(),
            name: "Only sub-projects".to_owned(),
            permissions: vec![Permission::MakeSubProjects],
            two_step: false,
        },
    )
    .await?
    .into_value();
    let bob = world.person("bob@example.test").await?;
    assign_role(
        &world.ctx(),
        AssignRole {
            session_token: admin,
            subject: Subject::Person(bob.person.id),
            given: Given::Role(only.id),
            project: Some(secret.id),
            applies_to: AppliesTo::ProjectAndSubProjects,
        },
    )
    .await?;
    let make_in = |parent: Option<bb_core::id::ProjectId>, short: &str| MakeProject {
        session_token: bob.session_token.clone(),
        parent,
        short_name: short.to_owned(),
        name: "Mine".to_owned(),
    };
    let hidden = make_project(&world.ctx(), make_in(Some(secret.id), "MINE"))
        .await
        .err();
    let missing = make_project(
        &world.ctx(),
        make_in(
            Some(bb_core::id::ProjectId::from_parts(
                jiff::Timestamp::UNIX_EPOCH,
                [7; 10],
            )?),
            "MINE",
        ),
    )
    .await
    .err();
    let not_found = Some(AppError::Refused(ErrorCode::NotFound));
    assert_eq!(missing, not_found);
    assert_eq!(hidden, not_found, "a project they cannot see is not there");
    Ok(())
}

#[tokio::test]
async fn sec_06_a_hidden_comment_or_relation_and_a_missing_one_cost_the_same_reads() -> Outcome {
    use bb_core::id::{CommentId, RelationId};
    use bb_core::relation::Direction;

    use super::scenarios::a_project;
    use super::scenarios_issues::an_issue;
    use super::scenarios_relations::{a_comment, a_kind};
    use crate::comments::{DeleteComment, delete_comment};
    use crate::relations::{AddRelation, RemoveRelation, add_relation, remove_relation};

    let world = World::new().await?;
    let project = a_project(&world, "SECRET").await?;
    let (first, second) = (
        an_issue(&world, project).await?,
        an_issue(&world, project).await?,
    );
    let comment = a_comment(&world, first).await?;
    let kind = a_kind(&world).await?;
    let relation = add_relation(
        &world.ctx(),
        AddRelation {
            session_token: world.admin.session_token.clone(),
            issue: first.to_string(),
            other: second.to_string(),
            kind,
            direction: Direction::Forward,
        },
    )
    .await?
    .into_value()
    .id;
    // Bob has no role anywhere: for him the comment and the relation are not there.
    let bob = world.person("bob@example.test").await?.session_token;
    let nobody = "0190f4c2-7a3e-7c1d-9b52-3d6e8f1a2b4c";
    let (missing_comment, missing_relation) = (
        nobody.parse::<CommentId>().map_err(|_| "no ID")?,
        nobody.parse::<RelationId>().map_err(|_| "no ID")?,
    );
    let mut counted = Vec::new();
    for (comment, relation) in [(comment, relation), (missing_comment, missing_relation)] {
        let before = world.store.reads();
        let answer = delete_comment(
            &world.ctx(),
            DeleteComment {
                session_token: bob.clone(),
                comment,
            },
        )
        .await
        .err();
        let comment_reads = world.store.reads() - before;
        let before = world.store.reads();
        let removed = remove_relation(
            &world.ctx(),
            RemoveRelation {
                session_token: bob.clone(),
                relation,
            },
        )
        .await
        .err();
        let relation_reads = world.store.reads() - before;
        let not_found = Some(AppError::Refused(ErrorCode::NotFound));
        assert_eq!((answer, removed), (not_found.clone(), not_found));
        counted.push((comment_reads, relation_reads));
    }
    assert_eq!(
        counted[0], counted[1],
        "a hidden and a missing one read the store as often"
    );
    Ok(())
}

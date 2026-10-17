//! An access key narrows what its owner sees of the tree everywhere a use case decides it
//! (ADR-0025 decision 8, M2-F27): the list, the search, the trail, the references to a parent, the
//! answer of a change, the preview of a move and the move itself. The tests ask as the
//! administrator, who sees everything, once with their session and once with a key for one project
//! that they made (AUTH-07); the key gets that project and what is inside it, and nothing above or
//! beside it.

use bb_core::ErrorCode;
use bb_core::access_key::KeyChoice;
use bb_core::id::ProjectId;

use super::world::{Outcome, PASSWORD, World};
use crate::access::ProjectLabel;
use crate::auth::{AppError, CreateAccessKey, create_access_key};
use crate::projects::{
    ChangeProject, CreateProject, MovePreviewQuery, MoveProject, ProjectItem, ProjectQuery,
    ProjectsQuery, change_project, create_project, move_project, preview_move, project, projects,
};

/// ```text
/// ACME ─ WEB ─ API        OTHER
///          └─ SUB
/// ```
struct Tree {
    acme: ProjectId,
    web: ProjectId,
    api: ProjectId,
    sub: ProjectId,
    other: ProjectId,
}

async fn tree(world: &World) -> Result<Tree, Box<dyn std::error::Error + Send + Sync>> {
    let create = async |parent: Option<ProjectId>, short: &str| -> Result<ProjectId, AppError> {
        Ok(create_project(
            &world.ctx(),
            CreateProject {
                session_token: world.admin.session_token.clone(),
                parent,
                short_name: short.to_owned(),
                name: format!("Project {short}"),
            },
        )
        .await?
        .into_value()
        .id)
    };
    let acme = create(None, "ACME").await?;
    let web = create(Some(acme), "WEB").await?;
    let api = create(Some(web), "API").await?;
    let sub = create(Some(web), "SUB").await?;
    let other = create(None, "OTHER").await?;
    Ok(Tree {
        acme,
        web,
        api,
        sub,
        other,
    })
}

/// A key of the administrator for one project and everything inside it: its secret, which is what
/// a script sends in place of a session.
async fn key_for(world: &World, project: ProjectId, choice: KeyChoice) -> Result<String, AppError> {
    Ok(create_access_key(
        &world.ctx(),
        CreateAccessKey {
            session_token: world.admin.session_token.clone(),
            password: PASSWORD.to_owned(),
            name: "For one project".to_owned(),
            choice,
            permissions: Vec::new(),
            projects: vec![project],
            expires_at: None,
        },
    )
    .await?
    .into_value()
    .secret)
}

/// The session of the administrator, and a key of theirs for the project.
async fn owner_and_key(world: &World, project: ProjectId) -> Result<(String, String), AppError> {
    Ok((
        world.admin.session_token.clone(),
        key_for(world, project, KeyChoice::Everything).await?,
    ))
}

fn list(token: &str, inside: Option<ProjectId>, search: Option<&str>) -> ProjectsQuery {
    ProjectsQuery {
        session_token: token.to_owned(),
        inside,
        search: search.map(str::to_owned),
        outside: None,
        movable: None,
        cursor: None,
        limit: 100,
    }
}

fn names(items: &[ProjectItem]) -> Vec<&str> {
    let mut names: Vec<&str> = items.iter().map(|item| item.short_name.as_str()).collect();
    names.sort_unstable();
    names
}

/// What a use case answers when it refuses with the code.
fn refusal(code: ErrorCode) -> AppError {
    AppError::Refused(code)
}

fn query(token: &str, address: &str) -> ProjectQuery {
    ProjectQuery {
        session_token: token.to_owned(),
        address: address.to_owned(),
    }
}

#[tokio::test]
async fn auth_07_a_key_for_a_project_lists_that_project_and_below_and_nothing_else() -> Outcome {
    let world = World::new().await?;
    let tree = tree(&world).await?;
    let (owner, key) = owner_and_key(&world, tree.web).await?;
    let ctx = world.ctx();

    // The owner sees the top of the tree: ACME and OTHER.
    let top = projects(&ctx, list(&owner, None, None)).await?.into_value();
    assert_eq!(names(&top.items), ["ACME", "OTHER"]);
    // The key's top is its project: WEB, whose parent is not named, and nothing beside it.
    let top = projects(&ctx, list(&key, None, None)).await?.into_value();
    assert_eq!(names(&top.items), ["WEB"]);
    assert!(top.items[0].parent.is_none(), "the parent is not named");
    assert!(top.items[0].has_sub_projects);
    // Inside WEB: API and SUB. Inside the project above it, or beside it: not found, like a
    // project that does not exist.
    let inside = projects(&ctx, list(&key, Some(tree.web), None))
        .await?
        .into_value();
    assert_eq!(names(&inside.items), ["API", "SUB"]);
    assert!(inside.items.iter().all(|item| !item.has_sub_projects));
    assert!(inside.items.iter().all(|item| {
        item.parent
            .as_ref()
            .is_some_and(|parent| parent.id == tree.web)
    }));
    for hidden in [tree.acme, tree.other] {
        let error = projects(&ctx, list(&key, Some(hidden), None)).await.err();
        assert_eq!(error, Some(refusal(ErrorCode::NotFound)), "{hidden}");
    }
    // The search finds only what the key sees; a word of a hidden project finds nothing.
    let found = projects(&ctx, list(&key, None, Some("project")))
        .await?
        .into_value();
    assert_eq!(names(&found.items), ["API", "SUB", "WEB"]);
    for word in ["acme", "other"] {
        let found = projects(&ctx, list(&key, None, Some(word)))
            .await?
            .into_value();
        assert!(found.items.is_empty(), "{word}");
    }
    // Moving into knows nothing of what the key does not see.
    let movable = projects(
        &ctx,
        ProjectsQuery {
            movable: Some(tree.api),
            ..list(&key, None, Some(""))
        },
    )
    .await?
    .into_value();
    assert_eq!(names(&movable.items), ["SUB"], "{movable:?}");
    Ok(())
}

#[tokio::test]
async fn auth_07_the_trail_of_a_key_stops_at_the_project_it_is_for() -> Outcome {
    let world = World::new().await?;
    let tree = tree(&world).await?;
    let (owner, key) = owner_and_key(&world, tree.web).await?;
    let ctx = world.ctx();

    let whole = project(&ctx, query(&owner, &tree.api.to_string()))
        .await?
        .into_value();
    let trail: Vec<_> = whole
        .trail
        .iter()
        .map(|step| step.short_name.as_str())
        .collect();
    assert_eq!(trail, ["ACME", "WEB"]);
    let narrowed = project(&ctx, query(&key, &tree.api.to_string()))
        .await?
        .into_value();
    let trail: Vec<_> = narrowed
        .trail
        .iter()
        .map(|step| step.short_name.as_str())
        .collect();
    assert_eq!(trail, ["WEB"], "the trail stops at the project of the key");
    assert_eq!(narrowed.project.parent, Some(tree.web));
    // The project of the key itself has no way down, and no parent to name.
    let top = project(&ctx, query(&key, &tree.web.to_string()))
        .await?
        .into_value();
    assert!(top.trail.is_empty());
    assert_eq!(top.project.parent, None);
    // Above it and beside it, by ID and by short name: not found, like a missing project.
    for address in [
        tree.acme.to_string(),
        tree.other.to_string(),
        "ACME".to_owned(),
        "OTHER".to_owned(),
    ] {
        let error = project(&ctx, query(&key, &address)).await.err();
        assert_eq!(error, Some(refusal(ErrorCode::NotFound)), "{address}");
    }
    Ok(())
}

#[tokio::test]
async fn auth_07_the_answer_of_a_change_does_not_name_a_parent_the_key_cannot_see() -> Outcome {
    let world = World::new().await?;
    let tree = tree(&world).await?;
    let (_, key) = owner_and_key(&world, tree.web).await?;
    let ctx = world.ctx();
    let rename = |token: &str, project: ProjectId, version: i64| ChangeProject {
        session_token: token.to_owned(),
        project,
        version,
        name: Some("Renamed".to_owned()),
        short_name: None,
        fields: std::collections::BTreeMap::new(),
    };
    let answer = change_project(&ctx, rename(&key, tree.web, 1))
        .await?
        .into_value();
    assert_eq!(
        answer.parent, None,
        "the parent of WEB is ACME, which the key cannot see"
    );
    // A project outside the key is not found.
    let error = change_project(&ctx, rename(&key, tree.other, 1))
        .await
        .err();
    assert_eq!(error, Some(refusal(ErrorCode::NotFound)));
    // A key that may only see changes nothing, in the project it is for.
    let see_only = key_for(&world, tree.web, KeyChoice::SeeOnly).await?;
    let error = change_project(&ctx, rename(&see_only, tree.api, 1))
        .await
        .err();
    assert_eq!(error, Some(refusal(ErrorCode::Forbidden)));
    Ok(())
}

#[tokio::test]
async fn auth_07_the_preview_and_the_move_of_a_key_stay_inside_the_key() -> Outcome {
    let world = World::new().await?;
    let tree = tree(&world).await?;
    let (owner, key) = owner_and_key(&world, tree.web).await?;
    let ctx = world.ctx();
    let preview_of = |token: &str, project: ProjectId, to: Option<ProjectId>| MovePreviewQuery {
        session_token: token.to_owned(),
        project,
        to,
    };
    // Into a project the key cannot see, or of a project it cannot see: not found; the owner may.
    for (moved, to) in [(tree.api, Some(tree.acme)), (tree.other, Some(tree.sub))] {
        let error = preview_move(&ctx, preview_of(&key, moved, to)).await.err();
        assert_eq!(error, Some(refusal(ErrorCode::NotFound)), "{moved}");
        assert!(
            preview_move(&ctx, preview_of(&owner, moved, to))
                .await
                .is_ok(),
            "the owner may"
        );
    }
    // Inside the key the preview names only what the key sees, and the move happens.
    let preview = preview_move(&ctx, preview_of(&key, tree.api, Some(tree.sub)))
        .await?
        .into_value();
    assert_eq!(preview.to.as_ref().map(|to| to.id), Some(tree.sub));
    let moved = move_project(
        &ctx,
        MoveProject {
            session_token: key.clone(),
            project: tree.api,
            to: Some(tree.sub),
            preview: preview.preview,
        },
    )
    .await?
    .into_value();
    assert_eq!(moved.parent, Some(tree.sub));
    // A move out of the key is refused when it is done, too: the re-check counts the key.
    let outside = move_project(
        &ctx,
        MoveProject {
            session_token: key.clone(),
            project: tree.sub,
            to: Some(tree.acme),
            preview: "made up".to_owned(),
        },
    )
    .await
    .err();
    assert_eq!(outside, Some(refusal(ErrorCode::NotFound)));
    Ok(())
}

#[tokio::test]
async fn auth_07_a_project_label_of_a_key_is_hidden_outside_the_key() -> Outcome {
    let world = World::new().await?;
    let tree = tree(&world).await?;
    let (owner, key) = owner_and_key(&world, tree.web).await?;
    let ctx = world.ctx();
    let (owner_actor, key_actor) = (ctx.actor(&owner).await?, ctx.actor(&key).await?);
    for (project, visible_to_key) in [
        (tree.web, true),
        (tree.api, true),
        (tree.acme, false),
        (tree.other, false),
    ] {
        let whole = ctx.project_label(&owner_actor, project).await?;
        assert!(matches!(whole, ProjectLabel::Visible { .. }));
        let narrowed = ctx.project_label(&key_actor, project).await?;
        assert_eq!(
            matches!(narrowed, ProjectLabel::Visible { .. }),
            visible_to_key,
            "{project}"
        );
    }
    Ok(())
}

#[tokio::test]
async fn auth_07_a_key_that_may_only_see_is_offered_no_place_to_move_a_project_into() -> Outcome {
    let world = World::new().await?;
    let tree = tree(&world).await?;
    let see_only = key_for(&world, tree.web, KeyChoice::SeeOnly).await?;
    let everything = key_for(&world, tree.web, KeyChoice::Everything).await?;
    let ctx = world.ctx();
    let movable = |token: &str| ProjectsQuery {
        movable: Some(tree.api),
        ..list(token, None, Some(""))
    };
    // The administrator, and a key for everything inside WEB, may move API into SUB; a key that
    // may only see may move nothing, so no place is offered (the check of `Shown::allows`).
    let owner = world.admin.session_token.clone();
    let places = projects(&ctx, movable(&owner)).await?.into_value();
    assert!(names(&places.items).contains(&"SUB"), "{places:?}");
    let places = projects(&ctx, movable(&everything)).await?.into_value();
    assert_eq!(names(&places.items), ["SUB"]);
    let places = projects(&ctx, movable(&see_only)).await?.into_value();
    assert!(places.items.is_empty(), "{places:?}");
    Ok(())
}

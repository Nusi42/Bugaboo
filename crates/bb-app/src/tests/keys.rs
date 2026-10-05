//! An access key narrows what its owner sees of the tree everywhere a use case decides it
//! (ADR-0025 decision 8, M2-F27): the list, the search, the trail, the references to a parent, the
//! answer of a change, the preview of a move and the move itself. The tests ask as the
//! administrator, who sees everything, once as themselves and once as somebody who came with a key
//! for one project (`Ctx::with_key`); the key gets that project and what is inside it, and
//! nothing above or beside it.

use std::collections::BTreeSet;

use bb_core::ErrorCode;
use bb_core::id::ProjectId;
use bb_policy::Narrowing;

use super::world::{Outcome, World};
use crate::access::ProjectLabel;
use crate::auth::{AppError, Ctx};
use crate::projects::{
    ChangeProject, MakeProject, MovePreviewQuery, MoveProject, ProjectItem, ProjectQuery,
    ProjectsQuery, change_project, make_project, move_project, preview_move, project, projects,
};

use super::fake::FakeStore;

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
    let make = async |parent: Option<ProjectId>, short: &str| -> Result<ProjectId, AppError> {
        Ok(make_project(
            &world.ctx(),
            MakeProject {
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
    let acme = make(None, "ACME").await?;
    let web = make(Some(acme), "WEB").await?;
    let api = make(Some(web), "API").await?;
    let sub = make(Some(web), "SUB").await?;
    let other = make(None, "OTHER").await?;
    Ok(Tree {
        acme,
        web,
        api,
        sub,
        other,
    })
}

/// A key for one project and everything inside it, with every kind of permission.
fn key_for(project: ProjectId) -> Narrowing {
    Narrowing::everything().in_projects(BTreeSet::from([project]))
}

fn list(world: &World, inside: Option<ProjectId>, search: Option<&str>) -> ProjectsQuery {
    ProjectsQuery {
        session_token: world.admin.session_token.clone(),
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

fn refused(error: Option<AppError>, code: ErrorCode) -> bool {
    error == Some(AppError::Refused(code))
}

/// The same request as the administrator and as a key for `scope`.
fn asked(world: &World, scope: Narrowing) -> (Ctx<'_, FakeStore>, Ctx<'_, FakeStore>) {
    (world.ctx(), world.ctx().with_key(scope))
}

fn query(world: &World, address: &str) -> ProjectQuery {
    ProjectQuery {
        session_token: world.admin.session_token.clone(),
        address: address.to_owned(),
    }
}

#[tokio::test]
async fn auth_07_a_key_for_a_project_lists_that_project_and_below_and_nothing_else() -> Outcome {
    let world = World::new().await?;
    let tree = tree(&world).await?;
    let (owner, key) = asked(&world, key_for(tree.web));

    // The owner sees the top of the tree: ACME and OTHER.
    let top = projects(&owner, list(&world, None, None))
        .await?
        .into_value();
    assert_eq!(names(&top.items), ["ACME", "OTHER"]);
    // The key's top is its project: WEB, whose parent is not named, and nothing beside it.
    let top = projects(&key, list(&world, None, None)).await?.into_value();
    assert_eq!(names(&top.items), ["WEB"]);
    assert!(top.items[0].parent.is_none(), "the parent is not named");
    assert!(top.items[0].has_sub_projects);
    // Inside WEB: API and SUB. Inside the project above it, or beside it: not found, like a
    // project that does not exist.
    let inside = projects(&key, list(&world, Some(tree.web), None))
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
        let error = projects(&key, list(&world, Some(hidden), None)).await.err();
        assert!(refused(error, ErrorCode::NotFound), "{hidden}");
    }
    // The search finds only what the key sees; a word of a hidden project finds nothing.
    let found = projects(&key, list(&world, None, Some("project")))
        .await?
        .into_value();
    assert_eq!(names(&found.items), ["API", "SUB", "WEB"]);
    for word in ["acme", "other"] {
        let found = projects(&key, list(&world, None, Some(word)))
            .await?
            .into_value();
        assert!(found.items.is_empty(), "{word}");
    }
    // Moving into knows nothing of what the key does not see.
    let movable = projects(
        &key,
        ProjectsQuery {
            movable: Some(tree.api),
            ..list(&world, None, Some(""))
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
    let (owner, key) = asked(&world, key_for(tree.web));

    let whole = project(&owner, query(&world, &tree.api.to_string()))
        .await?
        .into_value();
    let trail: Vec<_> = whole
        .trail
        .iter()
        .map(|step| step.short_name.as_str())
        .collect();
    assert_eq!(trail, ["ACME", "WEB"]);
    let narrowed = project(&key, query(&world, &tree.api.to_string()))
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
    let top = project(&key, query(&world, &tree.web.to_string()))
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
        let error = project(&key, query(&world, &address)).await.err();
        assert!(refused(error, ErrorCode::NotFound), "{address}");
    }
    Ok(())
}

#[tokio::test]
async fn auth_07_the_answer_of_a_change_does_not_name_a_parent_the_key_cannot_see() -> Outcome {
    let world = World::new().await?;
    let tree = tree(&world).await?;
    let (_, key) = asked(&world, key_for(tree.web));
    let rename = |project: ProjectId, version: i64| ChangeProject {
        session_token: world.admin.session_token.clone(),
        project,
        version,
        name: Some("Renamed".to_owned()),
        short_name: None,
    };
    let answer = change_project(&key, rename(tree.web, 1))
        .await?
        .into_value();
    assert_eq!(
        answer.parent, None,
        "the parent of WEB is ACME, which the key cannot see"
    );
    // A project outside the key is not found.
    let error = change_project(&key, rename(tree.other, 1)).await.err();
    assert!(refused(error, ErrorCode::NotFound));
    // A key that may only see changes nothing, in the project it is for.
    let see_only = world
        .ctx()
        .with_key(Narrowing::see_only().in_projects(BTreeSet::from([tree.web])));
    let error = change_project(&see_only, rename(tree.api, 1)).await.err();
    assert!(refused(error, ErrorCode::Forbidden));
    Ok(())
}

#[tokio::test]
async fn auth_07_the_preview_and_the_move_of_a_key_stay_inside_the_key() -> Outcome {
    let world = World::new().await?;
    let tree = tree(&world).await?;
    let (owner, key) = asked(&world, key_for(tree.web));
    let preview_of = |project: ProjectId, to: Option<ProjectId>| MovePreviewQuery {
        session_token: world.admin.session_token.clone(),
        project,
        to,
    };
    // Into a project the key cannot see, or of a project it cannot see: not found; the owner may.
    for (moved, to) in [(tree.api, Some(tree.acme)), (tree.other, Some(tree.sub))] {
        let error = preview_move(&key, preview_of(moved, to)).await.err();
        assert!(refused(error, ErrorCode::NotFound), "{moved}");
        assert!(
            preview_move(&owner, preview_of(moved, to)).await.is_ok(),
            "the owner may"
        );
    }
    // Inside the key the preview names only what the key sees, and the move happens.
    let preview = preview_move(&key, preview_of(tree.api, Some(tree.sub)))
        .await?
        .into_value();
    assert_eq!(preview.to.as_ref().map(|to| to.id), Some(tree.sub));
    let moved = move_project(
        &key,
        MoveProject {
            session_token: world.admin.session_token.clone(),
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
        &key,
        MoveProject {
            session_token: world.admin.session_token.clone(),
            project: tree.sub,
            to: Some(tree.acme),
            preview: "made up".to_owned(),
        },
    )
    .await
    .err();
    assert!(refused(outside, ErrorCode::NotFound));
    Ok(())
}

#[tokio::test]
async fn auth_07_a_project_label_of_a_key_is_hidden_outside_the_key() -> Outcome {
    let world = World::new().await?;
    let tree = tree(&world).await?;
    let (owner, key) = asked(&world, key_for(tree.web));
    let (owner_actor, key_actor) = (
        owner.actor(&world.admin.session_token).await?,
        key.actor(&world.admin.session_token).await?,
    );
    for (project, visible_to_key) in [
        (tree.web, true),
        (tree.api, true),
        (tree.acme, false),
        (tree.other, false),
    ] {
        let whole = owner.project_label(&owner_actor, project).await?;
        assert!(matches!(whole, ProjectLabel::Visible { .. }));
        let narrowed = key.project_label(&key_actor, project).await?;
        assert_eq!(
            matches!(narrowed, ProjectLabel::Visible { .. }),
            visible_to_key,
            "{project}"
        );
    }
    Ok(())
}

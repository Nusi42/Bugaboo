//! What a person who looks after tags learns about projects they do not see (TAG-01, ADR-0027): the
//! list leaves out a tag only those projects know, and a change never drops or adds a limit on a
//! project they cannot see.

use bb_core::ErrorCode;
use bb_core::access::AppliesTo;
use bb_core::id::{ProjectId, TagId};
use bb_policy::Permission;

use super::scenarios::{a_project, token};
use super::scenarios_issues::an_issue;
use super::scenarios_tags::tagged;
use super::world::{Outcome, World};
use crate::auth::AppError;
use crate::roles::{AssignRole, CreateRole, Given, Subject, assign_role, create_role};
use crate::tags::{ChangeTag, TagView, TagsQuery, change_tag, tags};

/// Bob looks after tags and sees every project and its issues but `secret`, which has *No access*
/// for him.
async fn blind_manager(world: &World, secret: ProjectId) -> Result<String, String> {
    let admin = token(world);
    let role = create_role(
        &world.ctx(),
        CreateRole {
            session_token: admin.clone(),
            name: "Tag keeper".to_owned(),
            permissions: vec![
                Permission::SeeProjects,
                Permission::SeeIssues,
                Permission::ManageLabels,
            ],
            two_step: false,
        },
    )
    .await
    .map_err(|error| error.to_string())?
    .into_value();
    let bob = world
        .person("bob@example.test")
        .await
        .map_err(|error| error.to_string())?;
    let give = |project, given, applies_to| AssignRole {
        session_token: admin.clone(),
        subject: Subject::Person(bob.person.id),
        given,
        project,
        applies_to,
    };
    for row in [
        give(None, Given::Role(role.id), AppliesTo::Organization),
        give(
            Some(secret),
            Given::NoAccess,
            AppliesTo::ProjectAndSubProjects,
        ),
    ] {
        assign_role(&world.ctx(), row)
            .await
            .map_err(|error| error.to_string())?;
    }
    Ok(bob.session_token)
}

async fn listed(world: &World, session_token: &str) -> Result<Vec<TagView>, AppError> {
    Ok(tags(
        &world.ctx(),
        TagsQuery {
            session_token: session_token.to_owned(),
            cursor: None,
            limit: 100,
        },
    )
    .await?
    .into_value()
    .items)
}

async fn limit_to(
    world: &World,
    session_token: &str,
    tag: TagId,
    projects: Vec<ProjectId>,
) -> Result<TagView, AppError> {
    let version = listed(world, &token(world))
        .await?
        .iter()
        .find(|view| view.id == tag)
        .map_or(1, |view| view.version);
    Ok(change_tag(
        &world.ctx(),
        ChangeTag {
            session_token: session_token.to_owned(),
            tag,
            version,
            name: None,
            color: None,
            projects: Some(projects),
        },
    )
    .await?
    .into_value())
}

async fn limits_of(world: &World, tag: TagId) -> Result<Vec<ProjectId>, AppError> {
    let mut ids: Vec<ProjectId> = listed(world, &token(world))
        .await?
        .iter()
        .find(|view| view.id == tag)
        .map(|view| view.projects.iter().map(|project| project.id).collect())
        .unwrap_or_default();
    ids.sort();
    Ok(ids)
}

#[tokio::test]
async fn tag_01_the_settings_list_leaves_out_a_tag_only_a_hidden_project_knows() -> Outcome {
    let world = World::new().await?;
    let (open, secret) = (
        a_project(&world, "OPEN").await?,
        a_project(&world, "SECRET").await?,
    );
    let bob = blind_manager(&world, secret).await?;
    let (issue, hidden_issue) = (
        an_issue(&world, open).await?,
        an_issue(&world, secret).await?,
    );
    let everywhere = tagged(&world, issue, "everywhere").await?;
    let hidden_only = tagged(&world, hidden_issue, "hidden-only").await?;
    limit_to(&world, &token(&world), hidden_only, vec![secret]).await?;
    let limited_here = tagged(&world, issue, "limited-here").await?;
    limit_to(&world, &token(&world), limited_here, vec![open]).await?;
    let both = tagged(&world, issue, "both").await?;
    limit_to(&world, &token(&world), both, vec![open, secret]).await?;
    let seen: Vec<String> = listed(&world, &bob)
        .await?
        .into_iter()
        .map(|view| view.name)
        .collect();
    assert!(seen.contains(&"everywhere".to_owned()));
    assert!(seen.contains(&"limited-here".to_owned()));
    assert!(seen.contains(&"both".to_owned()));
    assert!(!seen.contains(&"hidden-only".to_owned()), "{seen:?}");
    let own = listed(&world, &token(&world)).await?;
    assert!(own.iter().any(|view| view.id == hidden_only));
    assert!(own.iter().any(|view| view.id == everywhere));
    Ok(())
}

#[tokio::test]
async fn tag_01_saving_the_projects_keeps_the_limits_on_projects_the_person_cannot_see() -> Outcome
{
    let world = World::new().await?;
    let (open, other, secret) = (
        a_project(&world, "OPEN").await?,
        a_project(&world, "OTHER").await?,
        a_project(&world, "SECRET").await?,
    );
    let bob = blind_manager(&world, secret).await?;
    let issue = an_issue(&world, open).await?;
    let tag = tagged(&world, issue, "billing").await?;
    limit_to(&world, &token(&world), tag, vec![open, secret]).await?;
    let shown = limit_to(&world, &bob, tag, vec![other]).await?;
    assert_eq!(
        shown
            .projects
            .iter()
            .map(|project| project.id)
            .collect::<Vec<_>>(),
        vec![other],
        "the answer names no project the person cannot see"
    );
    let mut wanted = vec![other, secret];
    wanted.sort();
    assert_eq!(limits_of(&world, tag).await?, wanted);
    Ok(())
}

#[tokio::test]
async fn tag_01_an_empty_list_from_such_a_person_does_not_create_the_tag_organization_wide()
-> Outcome {
    let world = World::new().await?;
    let (open, secret) = (
        a_project(&world, "OPEN").await?,
        a_project(&world, "SECRET").await?,
    );
    let bob = blind_manager(&world, secret).await?;
    let issue = an_issue(&world, open).await?;
    let tag = tagged(&world, issue, "billing").await?;
    limit_to(&world, &token(&world), tag, vec![secret]).await?;
    // The tag is used in a project Bob sees, so it is in his list, as for everybody.
    let view = listed(&world, &bob)
        .await?
        .into_iter()
        .find(|view| view.id == tag)
        .ok_or("the tag is listed")?;
    assert!(view.projects.is_empty());
    assert!(
        view.whole_organization,
        "nothing shows that a hidden limit exists"
    );
    let saved = limit_to(&world, &bob, tag, Vec::new()).await?;
    assert!(saved.projects.is_empty());
    assert_eq!(limits_of(&world, tag).await?, vec![secret]);
    // A project he cannot see is not there for him when he asks for it.
    let refused = limit_to(&world, &bob, tag, vec![secret]).await.err();
    assert_eq!(refused, Some(AppError::Refused(ErrorCode::NotFound)));
    Ok(())
}

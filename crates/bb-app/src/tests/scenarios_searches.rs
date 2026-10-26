//! The scenarios of the saved-search use cases (ADR-0007, M1-F18, QRY-06): the preparation, the
//! call, and how many events the call added. `events.rs` names each use case once and runs them;
//! `saved_searches.rs` uses the same preparation for the rules.

use bb_core::access::AppliesTo;
use bb_core::id::{GroupId, ProjectId, SavedSearchId};
use bb_core::saved_search::{Column, Presentation, SearchScope, ShareTarget, Spacing};
use bb_policy::StartingRole;

use super::scenarios::{Added, a_project, measured, text, token};
use super::workflows_rules::starting_role;
use super::world::World;
use crate::auth::{AppError, Started};
use crate::people::{AddGroupMember, CreateGroup, add_group_member, create_group};
use crate::roles::{AssignRole, Given, Subject, assign_role};
use crate::saved_searches::{
    ChangeSearch, DeleteSearch, ProjectList, ProjectListQuery, RestoreSearch, SaveCopy, SaveSearch,
    SavedSearchView, SearchQuery, SearchesQuery, SetProjectList, ShareSearch, StopSharingSearch,
    change_search, delete_search, project_list, restore_search, save_copy, save_search,
    saved_search, saved_searches, set_project_list, share_search, stop_sharing_search,
};

/// A list with two columns, grouped by a field, *Compact*.
pub(crate) fn a_presentation() -> Presentation {
    Presentation {
        columns: vec![
            Column {
                field: "title".to_owned(),
                width: Some(320),
            },
            Column {
                field: "status".to_owned(),
                width: None,
            },
        ],
        spacing: Spacing::of("compact").unwrap_or_default(),
        group_by: Some("statusGroup".to_owned()),
        ..Presentation::default()
    }
}

/// Saves a search for the person with the session. The clock moves on a second, so that the
/// searches are in the order they were made in (an ID starts with the moment it was made).
pub(crate) async fn save(
    world: &World,
    who: &str,
    name: &str,
    search_text: &str,
) -> Result<SavedSearchView, AppError> {
    world.clock.advance(1);
    Ok(save_search(
        &world.ctx(),
        SaveSearch {
            session_token: who.to_owned(),
            name: name.to_owned(),
            text: search_text.to_owned(),
            presentation: a_presentation(),
        },
    )
    .await?
    .into_value())
}

/// Shares a saved search of the person.
pub(crate) async fn share(
    world: &World,
    who: &str,
    search: SavedSearchId,
    target: ShareTarget,
) -> Result<SavedSearchView, AppError> {
    Ok(share_search(
        &world.ctx(),
        ShareSearch {
            session_token: who.to_owned(),
            search,
            target,
        },
    )
    .await?
    .into_value())
}

/// Stops sharing.
pub(crate) async fn unshare(
    world: &World,
    who: &str,
    search: SavedSearchId,
    target: ShareTarget,
) -> Result<SavedSearchView, AppError> {
    Ok(stop_sharing_search(
        &world.ctx(),
        StopSharingSearch {
            session_token: who.to_owned(),
            search,
            target,
        },
    )
    .await?
    .into_value())
}

/// A page of saved searches (the first, of the largest size).
pub(crate) async fn list(
    world: &World,
    who: &str,
    scope: SearchScope,
) -> Result<Vec<SavedSearchView>, AppError> {
    Ok(saved_searches(
        &world.ctx(),
        SearchesQuery {
            session_token: who.to_owned(),
            scope,
            cursor: None,
            limit: 100,
        },
    )
    .await?
    .into_value()
    .items)
}

/// Opens a saved search.
pub(crate) async fn open(
    world: &World,
    who: &str,
    search: SavedSearchId,
) -> Result<SavedSearchView, AppError> {
    Ok(saved_search(
        &world.ctx(),
        SearchQuery {
            session_token: who.to_owned(),
            search,
        },
    )
    .await?
    .into_value())
}

/// The list a project shows.
pub(crate) async fn list_of(
    world: &World,
    who: &str,
    project: &str,
) -> Result<ProjectList, AppError> {
    Ok(project_list(
        &world.ctx(),
        ProjectListQuery {
            session_token: who.to_owned(),
            project: project.to_owned(),
        },
    )
    .await?
    .into_value())
}

/// Chooses the list of a project.
pub(crate) async fn choose_list(
    world: &World,
    who: &str,
    project: &str,
    search: Option<SavedSearchId>,
) -> Result<ProjectList, AppError> {
    Ok(set_project_list(
        &world.ctx(),
        SetProjectList {
            session_token: who.to_owned(),
            project: project.to_owned(),
            search,
        },
    )
    .await?
    .into_value())
}

/// Gives the person a starting role in the project and below it, or in the whole organization.
pub(crate) async fn with_role(
    world: &World,
    person: &Started,
    role: StartingRole,
    project: Option<ProjectId>,
) -> Result<(), String> {
    assign_role(
        &world.ctx(),
        AssignRole {
            session_token: token(world),
            subject: Subject::Person(person.person.id),
            given: Given::Role(starting_role(world, role).await?),
            project,
            applies_to: if project.is_some() {
                AppliesTo::ProjectAndSubProjects
            } else {
                AppliesTo::Organization
            },
        },
    )
    .await
    .map_err(text)?;
    Ok(())
}

/// A group with these people in it.
pub(crate) async fn a_group_of(
    world: &World,
    name: &str,
    members: &[&Started],
) -> Result<GroupId, String> {
    let group = create_group(
        &world.ctx(),
        CreateGroup {
            session_token: token(world),
            name: name.to_owned(),
        },
    )
    .await
    .map_err(text)?
    .into_value()
    .id;
    for member in members {
        add_group_member(
            &world.ctx(),
            AddGroupMember {
                session_token: token(world),
                group,
                person: member.person.id,
            },
        )
        .await
        .map_err(text)?;
    }
    Ok(group)
}

/// A saved search of Ada's.
async fn adas_search(world: &World) -> Result<SavedSearchView, String> {
    save(world, &token(world), "Open bugs", "status is open")
        .await
        .map_err(text)
}

pub(crate) async fn save_search_scenario(world: &World) -> Added {
    measured(&world.store, async {
        save(world, &token(world), "Open bugs", "status is open").await
    })
    .await
}

pub(crate) async fn change_search_scenario(world: &World) -> Added {
    let made = adas_search(world).await?;
    measured(
        &world.store,
        change_search(
            &world.ctx(),
            ChangeSearch {
                session_token: token(world),
                search: made.id,
                version: made.version,
                name: Some("Bugs".to_owned()),
                text: None,
                presentation: None,
            },
        ),
    )
    .await
}

pub(crate) async fn share_search_scenario(world: &World) -> Added {
    let made = adas_search(world).await?;
    let project = a_project(world, "ACME").await?;
    measured(
        &world.store,
        share(world, &token(world), made.id, ShareTarget::Project(project)),
    )
    .await
}

pub(crate) async fn stop_sharing_search_scenario(world: &World) -> Added {
    let made = adas_search(world).await?;
    let project = a_project(world, "ACME").await?;
    share(world, &token(world), made.id, ShareTarget::Project(project))
        .await
        .map_err(text)?;
    measured(
        &world.store,
        unshare(world, &token(world), made.id, ShareTarget::Project(project)),
    )
    .await
}

pub(crate) async fn delete_search_scenario(world: &World) -> Added {
    let made = adas_search(world).await?;
    measured(
        &world.store,
        delete_search(
            &world.ctx(),
            DeleteSearch {
                session_token: token(world),
                search: made.id,
            },
        ),
    )
    .await
}

pub(crate) async fn restore_search_scenario(world: &World) -> Added {
    let made = adas_search(world).await?;
    delete_search(
        &world.ctx(),
        DeleteSearch {
            session_token: token(world),
            search: made.id,
        },
    )
    .await
    .map_err(text)?;
    measured(
        &world.store,
        restore_search(
            &world.ctx(),
            RestoreSearch {
                session_token: token(world),
                search: made.id,
            },
        ),
    )
    .await
}

pub(crate) async fn save_copy_scenario(world: &World) -> Added {
    let made = adas_search(world).await?;
    measured(
        &world.store,
        save_copy(
            &world.ctx(),
            SaveCopy {
                session_token: token(world),
                search: made.id,
                name: None,
            },
        ),
    )
    .await
}

pub(crate) async fn set_project_list_scenario(world: &World) -> Added {
    let made = adas_search(world).await?;
    let project = a_project(world, "ACME").await?;
    share(world, &token(world), made.id, ShareTarget::Project(project))
        .await
        .map_err(text)?;
    measured(
        &world.store,
        choose_list(world, &token(world), "ACME", Some(made.id)),
    )
    .await
}

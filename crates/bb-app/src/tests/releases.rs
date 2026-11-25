//! Releases of a project (ISS-08, ADR-0049, AI-21) on the in-memory store: the record, its states
//! and its permission, the two fields of an issue on every way in, the events, and the tool that
//! lists them. The same rules on both databases are the tests of `bb-store` and `bb-server`.

use bb_core::ErrorCode;
use bb_core::event::{DomainEvent, EntityKind, RecordKind};
use bb_core::given::SetTo;
use bb_core::id::{IssueId, ProjectId, ReleaseId};
use bb_core::project_release::ReleaseState;
use bb_policy::StartingRole;
use serde_json::{Value, json};

use super::ai_tools::call;
use super::scenarios::{Added, a_project, measured, text, token};
use super::scenarios_issues::an_issue;
use super::scenarios_searches::with_role;
use super::world::{Outcome, World};
use crate::auth::AppError;
use crate::issues::{ChangeIssue, IssueQuery, IssueView, MoveIssue, change_issue, move_issue};
use crate::projects::{CreateProject, create_project};
use crate::releases::{
    ChangeRelease, CreateRelease, DeleteRelease, ReleaseView, ReleasesQuery, change_release,
    create_release, delete_release, releases, restore_release,
};

/// A sub-project of `parent`, made by Ada.
pub(super) async fn a_sub_project(
    world: &World,
    parent: ProjectId,
    short_name: &str,
) -> Result<ProjectId, String> {
    Ok(create_project(
        &world.ctx(),
        CreateProject {
            session_token: token(world),
            parent: Some(parent),
            short_name: short_name.to_owned(),
            name: "A part".to_owned(),
        },
    )
    .await
    .map_err(text)?
    .into_value()
    .id)
}

/// The request that makes a release in `project`, as the person of `session`.
pub(super) fn making(session: &str, project: &str, name: &str) -> CreateRelease {
    CreateRelease {
        session_token: session.to_owned(),
        project: project.to_owned(),
        name: name.to_owned(),
        release_date: None,
    }
}

/// A release of `project`, made by Ada.
pub(crate) async fn a_release(
    world: &World,
    project: &str,
    name: &str,
) -> Result<ReleaseView, String> {
    Ok(
        create_release(&world.ctx(), making(&token(world), project, name))
            .await
            .map_err(text)?
            .into_value(),
    )
}

/// A change of the release that names nothing yet.
pub(super) fn changing(session: &str, project: &str, release: &ReleaseView) -> ChangeRelease {
    ChangeRelease {
        session_token: session.to_owned(),
        project: project.to_owned(),
        release: release.id,
        version: release.version,
        name: None,
        release_date: None,
        state: None,
        reason: None,
    }
}

pub(super) fn deleting(session: &str, project: &str, release: ReleaseId) -> DeleteRelease {
    DeleteRelease {
        session_token: session.to_owned(),
        project: project.to_owned(),
        release,
    }
}

/// The issue as Ada sees it.
pub(super) async fn seen(world: &World, issue: IssueId) -> Result<IssueView, String> {
    Ok(crate::issues::issue(
        &world.ctx(),
        IssueQuery {
            session_token: token(world),
            issue: issue.to_string(),
        },
    )
    .await
    .map_err(text)?
    .into_value()
    .issue)
}

/// Sets the releases of an issue as Ada.
pub(super) async fn set_releases(
    world: &World,
    issue: IssueId,
    found: Option<Vec<ReleaseId>>,
    planned: Option<SetTo<ReleaseId>>,
) -> Result<IssueView, AppError> {
    let version = seen(world, issue)
        .await
        .map_err(AppError::Internal)?
        .version;
    change_issue::<_, IssueView>(
        &world.ctx(),
        ChangeIssue {
            found_in_release: found,
            planned_for_release: planned,
            ..ChangeIssue::of(&token(world), issue.to_string(), version)
        },
    )
    .await
    .map(bb_policy::Authorized::into_value)
}

pub(super) fn refused(result: Result<impl std::fmt::Debug, AppError>, code: ErrorCode) {
    match result {
        Err(AppError::Refused(found)) => assert_eq!(found, code),
        Err(AppError::Stale(_)) => assert_eq!(ErrorCode::VersionChanged, code),
        other => panic!("expected {code:?}, got {other:?}"),
    }
}

/// The preview of a tool that changes something, as the assistant asks it.
pub(super) fn previewing(who: &str, tool: &str, input: Value) -> crate::ai::tools::ToolCallQuery {
    crate::ai::tools::ToolCallQuery {
        mode: crate::ai::tools::ToolMode::Preview,
        ..call(who, tool, input)
    }
}

/// The last event of a release.
pub(super) fn last_release_event(world: &World) -> Option<DomainEvent> {
    world
        .store
        .lock()
        .events
        .iter()
        .rev()
        .find(|event| event.entity.kind == EntityKind::from(RecordKind::Release))
        .cloned()
}

// ---- Scenarios of the table of events ----

pub(crate) async fn create_release_scenario(world: &World) -> Added {
    a_project(world, "REL").await?;
    measured(
        &world.store,
        create_release(&world.ctx(), making(&token(world), "REL", "2.4")),
    )
    .await
}

pub(crate) async fn change_release_scenario(world: &World) -> Added {
    a_project(world, "REL").await?;
    let made = a_release(world, "REL", "2.4").await?;
    let asked = ChangeRelease {
        state: Some(ReleaseState::Released),
        ..changing(&token(world), "REL", &made)
    };
    measured(&world.store, change_release(&world.ctx(), asked)).await
}

pub(crate) async fn delete_release_scenario(world: &World) -> Added {
    a_project(world, "REL").await?;
    let made = a_release(world, "REL", "2.4").await?;
    measured(
        &world.store,
        delete_release(&world.ctx(), deleting(&token(world), "REL", made.id)),
    )
    .await
}

pub(crate) async fn restore_release_scenario(world: &World) -> Added {
    a_project(world, "REL").await?;
    let made = a_release(world, "REL", "2.4").await?;
    delete_release(&world.ctx(), deleting(&token(world), "REL", made.id))
        .await
        .map_err(text)?;
    measured(
        &world.store,
        restore_release(&world.ctx(), deleting(&token(world), "REL", made.id)),
    )
    .await
}

// ---- The acceptance tests ----

#[tokio::test]
async fn iss_08_a_release_is_offered_in_its_project_and_sub_projects_only() -> Outcome {
    let world = World::new().await?;
    let top = a_project(&world, "APP").await?;
    a_sub_project(&world, top, "WEB").await?;
    a_project(&world, "OPS").await?;
    let release = a_release(&world, "APP", "2.4").await?;
    let listed = |project: &'static str| {
        let world = &world;
        async move {
            releases(
                &world.ctx(),
                ReleasesQuery {
                    session_token: token(world),
                    project: project.to_owned(),
                    cursor: None,
                    limit: 50,
                },
            )
            .await
            .map_err(text)
            .map(|page| {
                page.into_value()
                    .items
                    .iter()
                    .map(|each| each.id)
                    .collect::<Vec<_>>()
            })
        }
    };
    assert_eq!(listed("APP").await?, [release.id]);
    assert_eq!(listed("WEB").await?, [release.id]);
    assert!(listed("OPS").await?.is_empty());
    let web_issue = an_issue(&world, a_project_id(&world, "WEB").await?).await?;
    set_releases(&world, web_issue, None, Some(SetTo::Value(release.id))).await?;
    let ops_issue = an_issue(&world, a_project_id(&world, "OPS").await?).await?;
    refused(
        set_releases(&world, ops_issue, None, Some(SetTo::Value(release.id))).await,
        ErrorCode::ReleaseNotOffered,
    );
    Ok(())
}

/// The ID of a project by its short name.
pub(super) async fn a_project_id(world: &World, short_name: &str) -> Result<ProjectId, String> {
    world
        .ctx()
        .find_project(short_name)
        .await
        .map_err(text)?
        .map(|found| found.project.id)
        .ok_or_else(|| format!("no project {short_name}"))
}

#[tokio::test]
async fn iss_08_a_release_name_is_unique_within_its_project() -> Outcome {
    let world = World::new().await?;
    let top = a_project(&world, "APP").await?;
    a_sub_project(&world, top, "WEB").await?;
    a_release(&world, "APP", "2.4").await?;
    // The same name in other capitals and spaces is the same name.
    refused(
        create_release(&world.ctx(), making(&token(&world), "APP", " 2.4 ")).await,
        ErrorCode::ReleaseNameTaken,
    );
    // Another project may use it, even one below.
    a_release(&world, "WEB", "2.4").await?;
    let other = a_release(&world, "APP", "2.5").await?;
    refused(
        change_release(
            &world.ctx(),
            ChangeRelease {
                name: Some("2.4".to_owned()),
                ..changing(&token(&world), "APP", &other)
            },
        )
        .await,
        ErrorCode::ReleaseNameTaken,
    );
    Ok(())
}

#[tokio::test]
async fn iss_08_only_manage_releases_makes_changes_marks_and_deletes_a_release() -> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "APP").await?;
    let release = a_release(&world, "APP", "2.4").await?;
    let member = world.person("tea@example.test").await.map_err(text)?;
    with_role(&world, &member, StartingRole::TeamMember, Some(project)).await?;
    let lead = world.person("lea@example.test").await.map_err(text)?;
    with_role(&world, &lead, StartingRole::ProjectLead, Some(project)).await?;
    let session = member.session_token.as_str();
    refused(
        create_release(&world.ctx(), making(session, "APP", "2.5")).await,
        ErrorCode::Forbidden,
    );
    refused(
        change_release(
            &world.ctx(),
            ChangeRelease {
                state: Some(ReleaseState::Released),
                ..changing(session, "APP", &release)
            },
        )
        .await,
        ErrorCode::Forbidden,
    );
    refused(
        delete_release(&world.ctx(), deleting(session, "APP", release.id)).await,
        ErrorCode::Forbidden,
    );
    // Reading needs only seeing the project; the member is not shown as one who manages.
    let page = releases(
        &world.ctx(),
        ReleasesQuery {
            session_token: session.to_owned(),
            project: "APP".to_owned(),
            cursor: None,
            limit: 50,
        },
    )
    .await
    .map_err(text)?
    .into_value();
    assert_eq!(page.items.len(), 1);
    assert!(!page.items[0].may_manage);
    // A project lead holds *Manage releases* at the start.
    let made = create_release(&world.ctx(), making(&lead.session_token, "APP", "2.5"))
        .await
        .map_err(text)?
        .into_value();
    assert!(made.may_manage);
    delete_release(&world.ctx(), deleting(&lead.session_token, "APP", made.id))
        .await
        .map_err(text)?;
    // A person who does not see the project is told it is not there.
    let stranger = world.person("sam@example.test").await.map_err(text)?;
    refused(
        create_release(&world.ctx(), making(&stranger.session_token, "APP", "2.6")).await,
        ErrorCode::NotFound,
    );
    Ok(())
}

#[tokio::test]
async fn iss_08_a_release_moves_planned_released_retired_and_keeps_the_day_released() -> Outcome {
    let world = World::new().await?;
    a_project(&world, "APP").await?;
    let made = a_release(&world, "APP", "2.4").await?;
    assert_eq!(made.state, ReleaseState::Planned);
    assert_eq!(made.released_on, None);
    let released = change_release(
        &world.ctx(),
        ChangeRelease {
            state: Some(ReleaseState::Released),
            ..changing(&token(&world), "APP", &made)
        },
    )
    .await
    .map_err(text)?
    .into_value();
    assert_eq!(released.state, ReleaseState::Released);
    let day = released.released_on.ok_or("no day released")?;
    // Back to *Planned* needs a reason.
    refused(
        change_release(
            &world.ctx(),
            ChangeRelease {
                state: Some(ReleaseState::Planned),
                ..changing(&token(&world), "APP", &released)
            },
        )
        .await,
        ErrorCode::ReleaseReasonNeeded,
    );
    let retired = change_release(
        &world.ctx(),
        ChangeRelease {
            state: Some(ReleaseState::Retired),
            ..changing(&token(&world), "APP", &released)
        },
    )
    .await
    .map_err(text)?
    .into_value();
    assert_eq!(retired.state, ReleaseState::Retired);
    assert_eq!(retired.released_on, Some(day));
    let other = a_release(&world, "APP", "2.5").await?;
    let other = change_release(
        &world.ctx(),
        ChangeRelease {
            state: Some(ReleaseState::Released),
            ..changing(&token(&world), "APP", &other)
        },
    )
    .await
    .map_err(text)?
    .into_value();
    let back = change_release(
        &world.ctx(),
        ChangeRelease {
            state: Some(ReleaseState::Planned),
            reason: Some("The build was pulled".to_owned()),
            ..changing(&token(&world), "APP", &other)
        },
    )
    .await
    .map_err(text)?
    .into_value();
    assert_eq!(back.released_on, None);
    let event = last_release_event(&world).ok_or("no event")?;
    assert!(
        event.changes.iter().any(|change| change.field == "reason"
            && change.after == Some(json!("The build was pulled")))
    );
    // A change on an old version is refused.
    refused(
        change_release(
            &world.ctx(),
            ChangeRelease {
                name: Some("2.5.1".to_owned()),
                ..changing(&token(&world), "APP", &other)
            },
        )
        .await,
        ErrorCode::VersionChanged,
    );
    Ok(())
}

#[tokio::test]
async fn iss_08_a_retired_release_is_not_offered_but_stays_on_its_issues() -> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "APP").await?;
    let release = a_release(&world, "APP", "2.4").await?;
    let named = an_issue(&world, project).await?;
    set_releases(
        &world,
        named,
        Some(vec![release.id]),
        Some(SetTo::Value(release.id)),
    )
    .await?;
    change_release(
        &world.ctx(),
        ChangeRelease {
            state: Some(ReleaseState::Retired),
            ..changing(&token(&world), "APP", &release)
        },
    )
    .await
    .map_err(text)?;
    let other = an_issue(&world, project).await?;
    refused(
        set_releases(&world, other, Some(vec![release.id]), None).await,
        ErrorCode::ReleaseNotOffered,
    );
    let view = seen(&world, named).await?;
    assert_eq!(view.found_in_release.clone().unwrap_or_default().len(), 1);
    assert_eq!(
        view.found_in_release.clone().unwrap_or_default()[0].state,
        ReleaseState::Retired
    );
    assert_eq!(
        view.planned_for_release.map(|each| each.id),
        Some(release.id)
    );
    // The issue keeps it when another field changes, and when the person sets the same list.
    set_releases(&world, named, Some(vec![release.id]), None).await?;
    Ok(())
}

#[tokio::test]
async fn iss_08_a_release_an_issue_names_cannot_be_deleted_and_the_refusal_counts_nothing()
-> Outcome {
    let world = World::new().await?;
    let project = a_project(&world, "APP").await?;
    a_project(&world, "HID").await?;
    let release = a_release(&world, "APP", "2.4").await?;
    let lead = world.person("lea@example.test").await.map_err(text)?;
    with_role(&world, &lead, StartingRole::ProjectLead, Some(project)).await?;
    // An issue the lead cannot see names it: it moved to a project only Ada sees.
    let issue = an_issue(&world, project).await?;
    set_releases(&world, issue, None, Some(SetTo::Value(release.id))).await?;
    let version = seen(&world, issue).await?.version;
    move_issue(
        &world.ctx(),
        MoveIssue {
            session_token: token(&world),
            issue: issue.to_string(),
            version,
            project: "HID".to_owned(),
        },
    )
    .await
    .map_err(text)?;
    refused(
        delete_release(
            &world.ctx(),
            deleting(&lead.session_token, "APP", release.id),
        )
        .await,
        ErrorCode::ReleaseInUse,
    );
    let said = ErrorCode::ReleaseInUse.text().render();
    assert!(!said.chars().any(|each| each.is_ascii_digit()), "{said}");
    assert!(ErrorCode::ReleaseInUse.arguments().is_empty());
    // One that no issue names goes to the trash, and *Undo* takes it back.
    let unused = a_release(&world, "APP", "2.5").await?;
    delete_release(
        &world.ctx(),
        deleting(&lead.session_token, "APP", unused.id),
    )
    .await
    .map_err(text)?;
    let other = an_issue(&world, project).await?;
    refused(
        set_releases(&world, other, Some(vec![unused.id]), None).await,
        ErrorCode::ReleaseNotOffered,
    );
    restore_release(
        &world.ctx(),
        deleting(&lead.session_token, "APP", unused.id),
    )
    .await
    .map_err(text)?;
    set_releases(&world, other, Some(vec![unused.id]), None).await?;
    Ok(())
}

#[tokio::test]
async fn iss_08_manage_releases_is_given_once_at_the_next_start_where_the_roles_were_made()
-> Outcome {
    use bb_core::technical::TechnicalName;
    use bb_policy::Permission;

    use crate::access::{CreateStartingRoles, create_starting_roles};

    let world = World::new().await?;
    let key = TechnicalName::new(Permission::ManageReleases.code()).map_err(text)?;
    let starting = |record: &crate::access::RoleRecord, role: StartingRole| {
        record.role.starting.as_ref().map(TechnicalName::as_str) == Some(role.code())
    };
    // A server of before: no copy holds the permission, and the migration made it due.
    {
        let mut shared = world.store.lock();
        for record in &mut shared.state.roles {
            record.permissions.remove(&Permission::ManageReleases);
        }
        shared.state.starting_permissions_due.push(key.clone());
    }
    let (events, log) = (world.store.event_count(), world.store.lock().log.len());
    create_starting_roles(&world.ctx(), CreateStartingRoles)
        .await
        .map_err(text)?;
    let holders: Vec<bool> = [
        StartingRole::Administrator,
        StartingRole::ProjectLead,
        StartingRole::TeamMember,
    ]
    .iter()
    .map(|role| {
        world.store.read(|state| {
            state.roles.iter().any(|record| {
                starting(record, *role) && record.permissions.contains(&Permission::ManageReleases)
            })
        })
    })
    .collect();
    assert_eq!(holders, [true, true, false]);
    assert_eq!(
        world.store.event_count() - events,
        2,
        "an event of each role"
    );
    assert_eq!(world.store.lock().log.len() - log, 2, "a line of each role");
    assert!(
        world
            .store
            .read(|state| state.starting_permissions_due.is_empty())
    );
    // Another start gives nothing, and a row that comes back finds the copies holding it.
    world.store.lock().state.starting_permissions_due.push(key);
    create_starting_roles(&world.ctx(), CreateStartingRoles)
        .await
        .map_err(text)?;
    assert_eq!(world.store.event_count() - events, 2);
    Ok(())
}

//! The scenarios of groups, roles and who has which role (M1-F18).

use super::*;

pub(crate) async fn create_group_scenario(world: &World) -> Added {
    measured(
        &world.store,
        create_group(
            &world.ctx(),
            CreateGroup {
                session_token: token(world),
                name: "Team".to_owned(),
            },
        ),
    )
    .await
}

pub(crate) async fn rename_group_scenario(world: &World) -> Added {
    let group = a_group(world, "Team").await?;
    measured(
        &world.store,
        rename_group(
            &world.ctx(),
            RenameGroup {
                session_token: token(world),
                group,
                version: FIRST_VERSION,
                name: "Crew".to_owned(),
            },
        ),
    )
    .await
}

pub(crate) async fn delete_group_scenario(world: &World) -> Added {
    let group = a_group(world, "Team").await?;
    measured(
        &world.store,
        delete_group(
            &world.ctx(),
            DeleteGroup {
                session_token: token(world),
                group,
            },
        ),
    )
    .await
}

pub(crate) async fn add_group_member_scenario(world: &World) -> Added {
    let (group, person) = (a_group(world, "Team").await?, a_person(world).await?);
    measured(
        &world.store,
        add_group_member(
            &world.ctx(),
            AddGroupMember {
                session_token: token(world),
                group,
                person,
            },
        ),
    )
    .await
}

pub(crate) async fn remove_group_member_scenario(world: &World) -> Added {
    let (group, person) = (a_group(world, "Team").await?, a_person(world).await?);
    add_group_member(
        &world.ctx(),
        AddGroupMember {
            session_token: token(world),
            group,
            person,
        },
    )
    .await
    .map_err(text)?;
    measured(
        &world.store,
        remove_group_member(
            &world.ctx(),
            RemoveGroupMember {
                session_token: token(world),
                group,
                person,
            },
        ),
    )
    .await
}

pub(crate) async fn create_role_scenario(world: &World) -> Added {
    measured(
        &world.store,
        create_role(
            &world.ctx(),
            CreateRole {
                session_token: token(world),
                name: "Own".to_owned(),
                permissions: vec![Permission::SeeProjects],
                two_step: false,
            },
        ),
    )
    .await
}

pub(crate) async fn change_role_scenario(world: &World) -> Added {
    let role = a_role(world, "Own").await?;
    measured(
        &world.store,
        change_role(
            &world.ctx(),
            ChangeRole {
                session_token: token(world),
                role,
                version: FIRST_VERSION,
                name: Some("Own role".to_owned()),
                permissions: None,
                two_step: Some(true),
            },
        ),
    )
    .await
}

pub(crate) async fn copy_role_scenario(world: &World) -> Added {
    let role = a_role(world, "Own").await?;
    measured(
        &world.store,
        copy_role(
            &world.ctx(),
            CopyRole {
                session_token: token(world),
                role,
                name: "Copy of own".to_owned(),
            },
        ),
    )
    .await
}

pub(crate) async fn delete_role_scenario(world: &World) -> Added {
    let role = a_role(world, "Own").await?;
    measured(
        &world.store,
        delete_role(
            &world.ctx(),
            DeleteRole {
                session_token: token(world),
                role,
                replacement: None,
            },
        ),
    )
    .await
}

pub(crate) async fn reset_role_scenario(world: &World) -> Added {
    let viewer = world
        .store
        .read(|state| {
            state
                .roles
                .iter()
                .find(|record| record.role.name == "Viewer")
                .map(|record| record.role.id)
        })
        .ok_or("no Viewer role")?;
    measured(
        &world.store,
        reset_role(
            &world.ctx(),
            ResetRole {
                session_token: token(world),
                role: viewer,
                version: FIRST_VERSION,
            },
        ),
    )
    .await
}

pub(crate) async fn assign_role_scenario(world: &World) -> Added {
    let (role, person, project) = (
        a_role(world, "Own").await?,
        a_person(world).await?,
        a_project(world, "ACME").await?,
    );
    measured(
        &world.store,
        assign_role(
            &world.ctx(),
            AssignRole {
                session_token: token(world),
                subject: Subject::Person(person),
                given: Given::Role(role),
                project: Some(project),
                applies_to: AppliesTo::ProjectAndSubProjects,
            },
        ),
    )
    .await
}

pub(crate) async fn remove_assignment_scenario(world: &World) -> Added {
    let (person, project) = (a_person(world).await?, a_project(world, "ACME").await?);
    let row = assign_role(
        &world.ctx(),
        AssignRole {
            session_token: token(world),
            subject: Subject::Person(person),
            given: Given::NoAccess,
            project: Some(project),
            applies_to: AppliesTo::ProjectAndSubProjects,
        },
    )
    .await
    .map_err(text)?
    .into_value();
    measured(
        &world.store,
        remove_assignment(
            &world.ctx(),
            RemoveAssignment {
                session_token: token(world),
                assignment: row.id,
            },
        ),
    )
    .await
}

//! The scenarios of access keys and of the lock on a text (M1-F18).

use super::*;

/// Ada makes a key that may only see.
fn see_only_key(world: &World) -> CreateAccessKey {
    CreateAccessKey {
        session_token: token(world),
        password: PASSWORD.to_owned(),
        name: "Nightly report".to_owned(),
        choice: KeyChoice::SeeOnly,
        permissions: Vec::new(),
        projects: Vec::new(),
        expires_at: None,
    }
}

pub(crate) async fn create_access_key_scenario(world: &World) -> Added {
    measured(
        &world.store,
        create_access_key(&world.ctx(), see_only_key(world)),
    )
    .await
}

pub(crate) async fn delete_access_key_scenario(world: &World) -> Added {
    let made = create_access_key(&world.ctx(), see_only_key(world))
        .await
        .map_err(text)?
        .into_value();
    measured(
        &world.store,
        delete_access_key(
            &world.ctx(),
            DeleteAccessKey {
                session_token: token(world),
                id: made.key.id,
            },
        ),
    )
    .await
}

// ---- Editing ----

/// The description of a project, held under an edit lock by Bob, who may change the project.
async fn bob_is_editing(world: &World) -> Result<EditWhat, String> {
    let project = a_project(world, "ACME").await?;
    let bob = world.person("bob@example.test").await.map_err(text)?;
    let role = create_role(
        &world.ctx(),
        CreateRole {
            session_token: token(world),
            name: "Editor".to_owned(),
            permissions: vec![Permission::SeeProjects, Permission::ChangeProjectSettings],
            two_step: false,
        },
    )
    .await
    .map_err(text)?
    .into_value()
    .id;
    assign_role(
        &world.ctx(),
        AssignRole {
            session_token: token(world),
            subject: Subject::Person(bob.person.id),
            given: Given::Role(role),
            project: Some(project),
            applies_to: AppliesTo::ProjectAndSubProjects,
        },
    )
    .await
    .map_err(text)?;
    let what = EditWhat {
        record: EntityRef {
            kind: RecordKind::Project.into(),
            id: EntityId::from(project),
        },
        part: TechnicalName::new("description").map_err(text)?,
    };
    take_lock(
        &world.ctx(),
        TakeLockQuery {
            session_token: bob.session_token,
            what: what.clone(),
            editor: EditorId::from_random([3; 16]),
        },
    )
    .await
    .map_err(text)?;
    Ok(what)
}

pub(crate) async fn ask_to_finish_scenario(world: &World) -> Added {
    let what = bob_is_editing(world).await?;
    measured(
        &world.store,
        ask_to_finish(
            &world.ctx(),
            AskToFinish {
                session_token: token(world),
                what,
            },
        ),
    )
    .await
}

pub(crate) async fn edit_anyway_scenario(world: &World) -> Added {
    let what = bob_is_editing(world).await?;
    measured(
        &world.store,
        edit_anyway(
            &world.ctx(),
            EditAnyway {
                session_token: token(world),
                what,
                editor: EditorId::from_random([4; 16]),
            },
        ),
    )
    .await
}

pub(crate) async fn release_stuck_lock_scenario(world: &World) -> Added {
    let what = bob_is_editing(world).await?;
    measured(
        &world.store,
        release_stuck_lock(
            &world.ctx(),
            ReleaseStuckLock {
                session_token: token(world),
                what,
            },
        ),
    )
    .await
}

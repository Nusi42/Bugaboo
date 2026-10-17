//! The starting roles and a new role (SEC-08).

use super::*;

#[tokio::test]
async fn sec_08_starting_roles_are_made_once_and_can_be_changed_and_reset() -> TestResult {
    each_database("sec_08_starting_roles", async |db| {
        let rig = Rig::start(db).await?;
        let org = Org::start(&rig).await?;
        // The six roles of architecture 7, in the starting order, as editable copies.
        let made = roles(&org).await?;
        let names: Vec<&str> = made.iter().map(|role| role.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "Administrator",
                "Project lead",
                "Team member",
                "Contributor",
                "Viewer",
                "Guest"
            ]
        );
        for (role, starting) in made.iter().zip(StartingRole::ALL) {
            assert_eq!(role.starting.as_deref(), Some(starting.code()));
            let wanted: Vec<Permission> = {
                let mut all = starting.permissions();
                all.sort();
                all
            };
            assert_eq!(role.permissions, wanted, "{}", role.name);
            assert!(!role.two_step);
        }
        // Another start makes nothing new: the roles are made once.
        create_starting_roles(&rig.services.ctx(Rig::info()), CreateStartingRoles).await?;
        assert_eq!(roles(&org).await?.len(), 6);

        // A starting role can be renamed and changed like any role; its mark stays.
        let viewer = role_named(&org, "Viewer").await?;
        let renamed = org
            .rig
            .api()
            .change_role(
                Rig::info(),
                ChangeRole {
                    session_token: org.admin.session.clone(),
                    role: viewer.id,
                    version: viewer.version,
                    name: Some("Looker".to_owned()),
                    permissions: Some(vec![Permission::SeeProjects, Permission::CheckAccess]),
                    two_step: Some(true),
                },
            )
            .await?
            .into_value();
        assert_eq!(renamed.name, "Looker");
        assert_eq!(renamed.starting.as_deref(), Some("viewer"));
        assert!(renamed.two_step);
        assert_eq!(renamed.permissions.len(), 2);

        // Reset gives it the starting permissions back and keeps the new name.
        let reset = org
            .rig
            .api()
            .reset_role(
                Rig::info(),
                ResetRole {
                    session_token: org.admin.session.clone(),
                    role: viewer.id,
                    version: renamed.version,
                },
            )
            .await?
            .into_value();
        assert_eq!(reset.name, "Looker");
        assert_eq!(reset.permissions, StartingRole::Viewer.permissions());
        assert!(!reset.two_step);

        // A role of our own has nothing to be reset to.
        let own = org.role("Own", &[Permission::SeeProjects]).await?;
        let result = org
            .rig
            .api()
            .reset_role(
                Rig::info(),
                ResetRole {
                    session_token: org.admin.session.clone(),
                    role: own.id,
                    version: own.version,
                },
            )
            .await;
        assert_eq!(refused(result), Some(ErrorCode::InvalidRequest));

        // A starting role can be deleted, and the next start does not bring it back.
        let guest = role_named(&org, "Guest").await?;
        org.rig
            .api()
            .delete_role(
                Rig::info(),
                DeleteRole {
                    session_token: org.admin.session.clone(),
                    role: guest.id,
                    replacement: None,
                },
            )
            .await?;
        create_starting_roles(&rig.services.ctx(Rig::info()), CreateStartingRoles).await?;
        assert!(roles(&org).await?.iter().all(|role| role.name != "Guest"));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn sec_08_a_new_role_has_no_levels_everything_is_hidden_until_somebody_decides() -> TestResult
{
    each_database("sec_08_new_role_hidden", async |db| {
        let rig = Rig::start(db).await?;
        let org = Org::start(&rig).await?;
        let own = org.role("Own", &[Permission::SeeProjects]).await?;
        let levels = org
            .rig
            .api()
            .role_levels(
                Rig::info(),
                LevelsQuery {
                    session_token: org.admin.session.clone(),
                    role: own.id,
                    project: None,
                },
            )
            .await?
            .into_value();
        assert!(levels.fields.is_empty() && levels.people.is_empty());
        // The starting roles carry the levels of architecture 7.
        let guest = role_named(&org, "Guest").await?;
        let levels = org
            .rig
            .api()
            .role_levels(
                Rig::info(),
                LevelsQuery {
                    session_token: org.admin.session.clone(),
                    role: guest.id,
                    project: None,
                },
            )
            .await?
            .into_value();
        assert_eq!(levels.fields.len(), Field::ALL.len());
        assert_eq!(levels.people.len(), PeopleKind::ALL.len());
        let level = |field| {
            levels
                .fields
                .iter()
                .find(|row| row.field == field)
                .map(|row| row.level)
        };
        assert_eq!(level(Field::CostCenter.into()), Some(FieldLevel::Hidden));
        assert_eq!(level(Field::Title.into()), Some(FieldLevel::CanChange));
        let people = levels
            .people
            .iter()
            .find(|row| row.kind == PeopleKind::AssignedTo)
            .map(|row| row.level);
        assert_eq!(people, Some(PeopleLevel::RoleOnly));
        Ok(())
    })
    .await
}

//! An M1 database after the upgrade to M2 (SEC-08, ADR-0025 decision 5): the administrator flag
//! is gone, the people who had it have the Administrator role, nobody else does, and they open
//! *Organization settings* as before.

use bb_app::access::{Permission, PolicyStore as _};
use bb_app::auth::StatusQuery;
use bb_auth::PasswordHasher;
use bb_core::id::UserId;
use bb_core::settings::SettingValue;
use bb_core::sign_in::SecurityAction;
use bb_store::{Database, M1Person, M3Setting, Scopes};
use bb_testing::{TestResult, each_database};
use jiff::Timestamp;
use zeroize::Zeroizing;

use super::rig::{PASSWORD, Rig};

#[tokio::test]
async fn sec_08_m1_administrators_become_administrators_and_nobody_else_does() -> TestResult {
    each_database("sec_08_m1_upgrade", async |db| {
        // The database as M1 left it: migrations 1 and 2, two administrators and one more person.
        let old = Database::connect(db.url()).await?;
        old.migrate_up_to(2, Timestamp::UNIX_EPOCH).await?;
        let hash = PasswordHasher::new()?
            .hash(Zeroizing::new(PASSWORD.to_owned()))
            .await?;
        let id = |n: u8| UserId::from_random([n; 16]);
        let (ada, bea, cy) = (id(1), id(2), id(3));
        old.seed_m1_people(
            &[
                M1Person {
                    id: ada,
                    display_name: "Ada Admin",
                    email: "ada@example.test",
                    password_hash: &hash,
                    administrator: true,
                },
                M1Person {
                    id: bea,
                    display_name: "Bea Builder",
                    email: "bea@example.test",
                    password_hash: &hash,
                    administrator: false,
                },
                M1Person {
                    id: cy,
                    display_name: "Cy Chief",
                    email: "cy@example.test",
                    password_hash: &hash,
                    administrator: true,
                },
            ],
            Timestamp::UNIX_EPOCH,
        )
        .await?;
        old.close().await;

        // The first start of M2: the migration, the starting roles, the roles of the administrators.
        let rig = Rig::start_upgraded(db).await?;
        assert_eq!(rig.role_managers().await?, 2);
        assert!(rig.storage.legacy_administrators().await?.is_empty());
        let status = rig
            .api()
            .status(Rig::info(), StatusQuery)
            .await
            .map_err(|error| error.to_string())?;
        assert!(
            !status.setup_needed,
            "setup stays closed: there are administrators"
        );
        assert!(rig.setup_code.is_empty());

        // They sign in as before and open Organization settings: they hold what its screens ask for.
        for email in ["ada@example.test", "cy@example.test"] {
            let started = rig.sign_in(email, PASSWORD).await?;
            let held = rig.permissions(&started.session_token).await?;
            for permission in [
                Permission::ManageInvitations,
                Permission::ChangeOrganizationSettings,
                Permission::SeeStatus,
                Permission::ManageRoles,
            ] {
                assert!(held.contains(&permission), "{email}: {permission}");
            }
        }
        // Nobody else became an administrator.
        let started = rig.sign_in("bea@example.test", PASSWORD).await?;
        assert!(rig.permissions(&started.session_token).await?.is_empty());

        // Each of them was given the role with its own entry in the log.
        let entries = rig.database.security_log_entries(0, 100).await?;
        let made = entries
            .iter()
            .filter(|entry| entry.action.as_str() == SecurityAction::AssignmentMade.code())
            .count();
        assert_eq!(made, 2);
        // Another start gives nobody anything again.
        bb_app::access::make_starting_roles(
            &rig.services.ctx(Rig::info()),
            bb_app::access::MakeStartingRoles,
        )
        .await?;
        let after = rig.database.security_log_entries(0, 100).await?;
        assert_eq!(after.len(), entries.len());
        Ok(())
    })
    .await
}

/// The words a person's choice of theme had before the mode and the two themes replaced it.
#[tokio::test]
async fn cfg_04_the_theme_of_m1_becomes_the_mode_and_the_light_and_the_dark_theme() -> TestResult {
    each_database("cfg_04_theme_carried_over", async |db| {
        let old = Database::connect(db.url()).await?;
        old.migrate_up_to(2, Timestamp::UNIX_EPOCH).await?;
        let hash = PasswordHasher::new()?
            .hash(Zeroizing::new(PASSWORD.to_owned()))
            .await?;
        let id = |n: u8| UserId::from_random([n; 16]);
        let people: Vec<M1Person<'_>> = [(1, "ada"), (2, "bea"), (3, "cy"), (4, "di")]
            .iter()
            .map(|(n, name)| M1Person {
                id: id(*n),
                display_name: name,
                email: [
                    "ada@example.test",
                    "bea@example.test",
                    "cy@example.test",
                    "di@example.test",
                ][usize::from(*n) - 1],
                password_hash: &hash,
                administrator: false,
            })
            .collect();
        old.seed_m1_people(&people, Timestamp::UNIX_EPOCH).await?;
        old.migrate_up_to(3, Timestamp::UNIX_EPOCH).await?;
        let person = |n: u8| id(n).to_bytes().to_vec();
        let row = |key, level, scope, value, locked| M3Setting {
            key,
            level,
            scope,
            value,
            locked,
        };
        old.seed_old_setting_rows(&[
            row("appearance.theme", "person", person(1), "\"paper\"", false),
            row(
                "appearance.theme",
                "person",
                person(2),
                "\"midnight\"",
                false,
            ),
            row("appearance.theme", "person", person(3), "\"device\"", false),
            // A person that is not there any more has nothing to carry over to.
            row(
                "appearance.theme",
                "person",
                id(9).to_bytes().to_vec(),
                "\"dusk\"",
                false,
            ),
            row(
                "region.time-zone",
                "person",
                person(4),
                "\"Europe/Berlin\"",
                false,
            ),
            row(
                "appearance.theme",
                "organization",
                Vec::new(),
                "\"dusk\"",
                true,
            ),
        ])
        .await?;
        old.close().await;

        let rig = Rig::start_upgraded(db).await?;
        let database = &rig.database;
        let layers = async |n: u8| {
            database
                .setting_layers(&Scopes {
                    project: None,
                    person: Some(id(n)),
                })
                .await
        };
        let text = |value: &str| SettingValue::Text(value.to_owned());
        let ada = layers(1).await?;
        assert_eq!(ada.person.len(), 2);
        assert_eq!(ada.person["appearance.mode"], text("light"));
        assert_eq!(ada.person["appearance.light-theme"], text("paper"));
        let bea = layers(2).await?;
        assert_eq!(bea.person.len(), 2);
        assert_eq!(bea.person["appearance.mode"], text("dark"));
        assert_eq!(bea.person["appearance.dark-theme"], text("midnight"));
        let cy = layers(3).await?;
        assert_eq!(cy.person.len(), 1);
        assert_eq!(cy.person["appearance.mode"], text("device"));
        // Everything else a person had stays theirs.
        let di = layers(4).await?;
        assert_eq!(di.person["region.time-zone"], text("Europe/Berlin"));
        // The organization's locked theme became a locked mode and a locked dark theme.
        let organization = &ada.organization;
        assert_eq!(organization.len(), 2);
        assert_eq!(organization["appearance.mode"].value, text("dark"));
        assert!(organization["appearance.mode"].locked);
        assert_eq!(organization["appearance.dark-theme"].value, text("dusk"));
        assert!(organization["appearance.dark-theme"].locked);
        // Nothing of the old setting is left behind, for people or the organization.
        assert_eq!(rig.database.setting_row_count().await?, 2);
        Ok(())
    })
    .await
}

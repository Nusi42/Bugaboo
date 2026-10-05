//! An M1 database after the upgrade to M2 (SEC-08, ADR-0025 decision 5): the administrator flag
//! is gone, the people who had it have the Administrator role, nobody else does, and they open
//! *Organization settings* as before.

use bb_app::access::{Permission, PolicyStore as _};
use bb_app::auth::StatusQuery;
use bb_auth::PasswordHasher;
use bb_core::id::UserId;
use bb_core::sign_in::SecurityAction;
use bb_store::{Database, M1Person};
use bb_testing::each_database;
use jiff::Timestamp;
use zeroize::Zeroizing;

use super::rig::{Outcome, PASSWORD, Rig};

#[tokio::test]
async fn sec_08_m1_administrators_become_administrators_and_nobody_else_does() -> Outcome {
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

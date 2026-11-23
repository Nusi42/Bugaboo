//! The list of what to do again after a restore (ADR-0044, decision 8): removals, switch-offs and
//! keys taken back after the backup are done again, also when their line only waits in the database.

use bb_core::person::Standing;
use bb_core::settings::privacy::RemovedTexts;

use super::*;

#[tokio::test]
async fn ops_07_a_restore_replays_removals_switch_offs_and_taken_back_keys_made_after_the_backup()
-> TestResult {
    each_database("ops_07_replay", async |db| {
        let installation = Installation::new(db)?;
        let database = installation.open().await?;
        let storage = installation.storage(&database);
        let mut source = ids();
        let (ada, key) = a_key(&storage, &mut source).await?;
        let bea_id = bea(&storage, &mut source).await?;
        let (backup, moment) = backed_up(&installation, &database).await?;
        database.close().await;
        // The lines later changes wrote: one of each kind after the backup, and one before it,
        // which the backup holds already.
        let later = |minutes: i64| moment.checked_add(SignedDuration::from_mins(minutes));
        let lines = [
            ReplayLine {
                moment: moment.checked_sub(SignedDuration::from_mins(5))?,
                kind: ReplayKind::AccessKey,
                id: AccessKeyId::generate(&mut source).to_string(),
                choice: None,
            },
            ReplayLine {
                moment: later(1)?,
                kind: ReplayKind::AccessKey,
                id: key.to_string(),
                choice: None,
            },
            ReplayLine {
                moment: later(2)?,
                kind: ReplayKind::SwitchOff,
                id: bea_id.to_string(),
                choice: None,
            },
            ReplayLine {
                moment: later(3)?,
                kind: ReplayKind::Removal,
                id: ada.to_string(),
                choice: Some("keep".to_owned()),
            },
        ];
        // A line whose ID is not a key's: it is counted as unreadable and stops nothing.
        let unreadable = format!(
            "{} access-key not-a-key
",
            later(4)?
        );
        std::fs::write(
            &installation.places.replay_list,
            bb_core::backup::replay::write(&lines) + &unreadable,
        )?;
        let report = installation
            .restore(&backup.path, &[], later(10)?)
            .await
            .map_err(failed)?;
        assert_eq!(report.unreadable_lines, 1);
        assert_eq!(report.replayed, lines[1..].to_vec());
        assert_eq!(report.not_replayed, Vec::<ReplayLine>::new());
        let restored = installation.open().await?;
        assert!(!has_key(&restored, key).await?, "the key stays taken back");
        // The switch-off and the removal after the backup are done again.
        let again = installation.storage(&restored);
        let standing_of = async |person: UserId| {
            bb_app::people::StandingStore::standing(&again, person)
                .await
                .map(|found| found.map(|standing| standing.standing))
        };
        assert_eq!(standing_of(bea_id).await?, Some(Standing::SwitchedOff));
        assert_eq!(standing_of(ada).await?, Some(Standing::Removed));
        let gone = bb_app::auth::AccountStore::account_by_email(&again, "ada@example.test").await?;
        assert!(gone.is_none(), "the removed person's address is cleared");
        restored.close().await;
        Ok(())
    })
    .await
}

#[tokio::test]
async fn priv_04_a_line_that_waits_in_the_database_is_run_by_a_restore() -> TestResult {
    each_database("priv_04_replay_waiting_restore", async |db| {
        let installation = Installation::new(db)?;
        let database = installation.open().await?;
        let storage = installation.storage(&database);
        let mut source = ids();
        let (_, key) = a_key(&storage, &mut source).await?;
        let (backup, moment) = backed_up(&installation, &database).await?;
        // The key is taken back after the backup, and its line never reached the file: the server
        // stopped before it moved.
        let line = ReplayLine {
            moment: moment.checked_add(SignedDuration::from_mins(1))?,
            kind: ReplayKind::AccessKey,
            id: key.to_string(),
            choice: None,
        };
        let waiting = line.clone();
        storage
            .record_change(&event(&mut source, 2), move |tx| {
                Box::pin(async move {
                    <AuthStorage as bb_app::privacy::replay::ReplayStore>::wait_to_replay(
                        tx,
                        &[waiting],
                    )
                    .await
                })
            })
            .await?;
        database.close().await;
        assert!(!installation.places.replay_list.exists());
        let report = installation
            .restore(
                &backup.path,
                &[],
                moment.checked_add(SignedDuration::from_mins(10))?,
            )
            .await
            .map_err(failed)?;
        assert_eq!(report.replayed, std::slice::from_ref(&line));
        let restored = installation.open().await?;
        assert!(!has_key(&restored, key).await?, "the key stays taken back");
        restored.close().await;
        // The list in the data folder holds the line now.
        let kept = std::fs::read_to_string(&installation.places.replay_list)?;
        assert_eq!(bb_core::backup::replay::read(&kept).0, [line]);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn priv_04_a_restore_says_when_its_replay_leaves_nobody_who_decides_roles() -> TestResult {
    use bb_app::access::PolicyStore as _;
    use bb_core::access::{AppliesTo, Assignment, Role};
    use bb_core::id::{AssignmentId, RoleId};
    use bb_core::version::FIRST_VERSION;
    use bb_policy::Permission;
    each_database("priv_04_replay_nobody_decides_roles", async |db| {
        let installation = Installation::new(db)?;
        let database = installation.open().await?;
        let storage = installation.storage(&database);
        let mut source = ids();
        // In the backup, Bea is the only one who decides roles. Later another administrator came
        // and switched her off; the backup does not know that administrator.
        let bea = person(&mut source, "bea@example.test");
        let bea_id = bea.id;
        let role = Role {
            id: RoleId::generate(&mut source)?,
            name: "Keeper".to_owned(),
            starting: None,
            position: 0,
            two_step: false,
            version: FIRST_VERSION,
        };
        let given = Assignment {
            id: AssignmentId::generate(&mut source)?,
            person: Some(bea_id),
            group: None,
            role: Some(role.id),
            project: None,
            applies_to: AppliesTo::Organization,
            given_at: at(1),
        };
        storage
            .record_change(&event(&mut source, 2), move |tx| {
                Box::pin(async move {
                    AuthStorage::insert_person(tx, &bea, "hash", at(0)).await?;
                    AuthStorage::insert_role(
                        tx,
                        &role,
                        &[Permission::ManageRoles, Permission::DecideRoles],
                        &[],
                        &[],
                    )
                    .await?;
                    AuthStorage::insert_assignment(tx, &given).await
                })
            })
            .await?;
        let (backup, moment) = backed_up(&installation, &database).await?;
        database.close().await;
        let line = ReplayLine {
            moment: moment.checked_add(SignedDuration::from_mins(1))?,
            kind: ReplayKind::SwitchOff,
            id: bea_id.to_string(),
            choice: None,
        };
        std::fs::write(
            &installation.places.replay_list,
            bb_core::backup::replay::write(std::slice::from_ref(&line)),
        )?;
        let report = installation
            .restore(
                &backup.path,
                &[],
                moment.checked_add(SignedDuration::from_mins(10))?,
            )
            .await
            .map_err(failed)?;
        assert_eq!(report.replayed, [line]);
        assert!(report.nobody_decides_roles);
        let restored = installation.open().await?;
        let again = installation.storage(&restored);
        // The standing says a restore did it, not an administrator.
        let standing = bb_app::people::StandingStore::standing(&again, bea_id)
            .await?
            .ok_or("Bea has a standing")?;
        assert_eq!(standing.standing, Standing::SwitchedOff);
        assert_eq!(standing.by, None);
        // The security log says so too.
        let entries = restored.security_log_entries(0, 100).await?;
        assert!(
            entries
                .iter()
                .any(|entry| entry.action.as_str() == "backup.restore_left_nobody_for_roles"),
            "{entries:?}"
        );
        restored.close().await;
        Ok(())
    })
    .await
}

/// The removal of `person` that is not finished, its words and the job that does its next step,
/// as the restored database holds them.
async fn left_to_finish(
    storage: &AuthStorage,
    person: UserId,
) -> Result<(bool, bool, bool), Failure> {
    use crate::db::StoredId as _;
    use bb_app::privacy::removal::RemovalStore as _;
    let pending = storage.pending_removal(person).await?;
    let words = storage.removal_words(person).await?;
    let job = match &pending {
        Some(pending) => {
            let mut select = Query::select();
            select
                .column(crate::schema::Job::Id)
                .from(crate::schema::Job::Table)
                .and_where(Expr::col(crate::schema::Job::Id).eq(pending.job.stored()))
                .and_where(Expr::col(crate::schema::Job::State).eq("queued"));
            storage.database().fetch_optional(&select).await?.is_some()
        }
        None => false,
    };
    Ok((pending.is_some(), words.is_some(), job))
}

#[tokio::test]
async fn priv_04_a_backup_made_while_a_removal_goes_on_holds_what_finishes_it() -> TestResult {
    each_database("priv_04_backup_during_removal", async |db| {
        let installation = Installation::new(db)?;
        let database = installation.open().await?;
        let storage = installation.storage(&database);
        let mut source = ids();
        let bea_id = bea(&storage, &mut source).await?;
        crate::test_support::removed(&storage, &mut source, bea_id, RemovedTexts::Keep).await?;
        let (backup, moment) = backed_up(&installation, &database).await?;
        database.close().await;
        installation
            .restore(
                &backup.path,
                &[],
                moment.checked_add(SignedDuration::from_mins(10))?,
            )
            .await
            .map_err(failed)?;
        let restored = installation.open().await?;
        // The row, the words (the same key files open them) and the job that goes on with them.
        let again = installation.storage(&restored);
        assert_eq!(left_to_finish(&again, bea_id).await?, (true, true, true));
        restored.close().await;
        Ok(())
    })
    .await
}

#[tokio::test]
async fn priv_04_a_removal_replayed_into_an_older_backup_starts_its_job_again() -> TestResult {
    each_database("priv_04_replay_removal_job", async |db| {
        let installation = Installation::new(db)?;
        let database = installation.open().await?;
        let storage = installation.storage(&database);
        let mut source = ids();
        let bea_id = bea(&storage, &mut source).await?;
        let (backup, moment) = backed_up(&installation, &database).await?;
        // Bea is removed after the backup; the line of the replay list says so.
        crate::test_support::removed(&storage, &mut source, bea_id, RemovedTexts::Delete).await?;
        database.close().await;
        let line = ReplayLine {
            moment: moment.checked_add(SignedDuration::from_mins(1))?,
            kind: ReplayKind::Removal,
            id: bea_id.to_string(),
            choice: Some("delete".to_owned()),
        };
        std::fs::write(
            &installation.places.replay_list,
            bb_core::backup::replay::write(std::slice::from_ref(&line)),
        )?;
        let report = installation
            .restore(
                &backup.path,
                &[],
                moment.checked_add(SignedDuration::from_mins(10))?,
            )
            .await
            .map_err(failed)?;
        assert_eq!(report.replayed, [line]);
        let restored = installation.open().await?;
        let again = installation.storage(&restored);
        assert_eq!(left_to_finish(&again, bea_id).await?, (true, true, true));
        restored.close().await;
        Ok(())
    })
    .await
}

/// Bea, stored as a person.
async fn bea(storage: &AuthStorage, source: &mut impl IdSource) -> Result<UserId, Failure> {
    let bea = person(source, "bea@example.test");
    let id = bea.id;
    storage
        .record_change(&event(source, 2), move |tx| {
            Box::pin(async move { AuthStorage::insert_person(tx, &bea, "hash", at(0)).await })
        })
        .await?;
    Ok(id)
}

/// The moment of the backups of these tests.
const MOMENT: &str = "2026-11-10T10:00:00Z";

/// A backup of `database` at [`MOMENT`], and the moment.
async fn backed_up(
    installation: &Installation,
    database: &Database,
) -> Result<(CreatedBackup, Timestamp), Failure> {
    let moment: Timestamp = MOMENT.parse()?;
    let backup = database
        .create_backup(&installation.places, Purpose::Regular, moment)
        .await?;
    Ok((backup, moment))
}

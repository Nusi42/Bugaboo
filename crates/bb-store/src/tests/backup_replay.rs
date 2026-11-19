//! The list of what to do again after a restore (ADR-0044, decision 8): removals, switch-offs and
//! keys taken back after the backup are done again, also when their line only waits in the database.

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
        let moment: Timestamp = "2026-11-10T10:00:00Z".parse()?;
        let backup = database
            .create_backup(&installation.places, Purpose::Regular, moment)
            .await?;
        database.close().await;
        // The lines a later change wrote (the writer comes with *Remove a person*): one of each
        // kind after the backup, and one before it, which the backup holds already.
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
                id: ada.to_string(),
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
        assert_eq!(report.replayed, [lines[1].clone()]);
        // A removal and a switch-off are named in the report until the program can run them.
        assert_eq!(report.not_replayed, [lines[2].clone(), lines[3].clone()]);
        let restored = installation.open().await?;
        assert!(!has_key(&restored, key).await?, "the key stays taken back");
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
        let moment: Timestamp = "2026-11-10T10:00:00Z".parse()?;
        let backup = database
            .create_backup(&installation.places, Purpose::Regular, moment)
            .await?;
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

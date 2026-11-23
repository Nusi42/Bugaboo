//! The commands of backups: `bugaboo backup`, `backup list` and `restore` (OPS-07, ADR-0046), and
//! `keys import`.

use bb_testing::TestResult;

use super::super::secrets::tests::installation;
use super::super::secrets::{export_keys, import_keys};
use super::*;

fn backup_args(store: &StoreArgs) -> BackupArgs {
    BackupArgs {
        store: store.clone(),
        to: None,
        command: None,
    }
}

#[tokio::test]
async fn ops_07_the_commands_back_up_list_and_restore_an_installation() -> TestResult {
    let folder = tempfile::tempdir()?;
    let (store, data) = installation(folder.path()).await?;
    backup(&backup_args(&store)).await?;
    let found = backups_in(&data.backups())?;
    assert_eq!(found.len(), 1);
    backup(&BackupArgs {
        command: Some(BackupCommand::List),
        ..backup_args(&store)
    })
    .await?;
    restore(&RestoreArgs {
        store: store.clone(),
        file: found[0].1.clone(),
        replay_list: None,
    })
    .await?;
    // The backup of the present is there beside the one restored, and the log still verifies.
    assert_eq!(backups_in(&data.backups())?.len(), 2);
    super::super::secrets::verify_security_log(&store).await?;
    Ok(())
}

#[tokio::test]
async fn ops_07_restore_refuses_a_running_server_and_another_account() -> TestResult {
    let folder = tempfile::tempdir()?;
    let (store, data) = installation(folder.path()).await?;
    backup(&backup_args(&store)).await?;
    let file = backups_in(&data.backups())?[0].1.clone();
    // A running server holds the lock of the data folder.
    let running = data.lock_server()?.ok_or("the folder was locked already")?;
    let error = restore(&RestoreArgs {
        store: store.clone(),
        file: file.clone(),
        replay_list: None,
    })
    .await
    .err()
    .ok_or("the restore went on beside a running server")?;
    assert!(format!("{error:#}").contains("Stop it"), "{error:#}");
    assert_eq!(backups_in(&data.backups())?.len(), 1, "nothing was changed");
    drop(running);
    // Another account is refused the same way as the other commands that change the data
    // folder: the check is theirs (`require_own_installation`), and on Linux it names the owner.
    #[cfg(target_os = "linux")]
    assert!(crate::data_folder::other_owner(data.root()).is_none());
    Ok(())
}

#[tokio::test]
async fn ops_07_keys_import_is_the_twin_of_keys_export() -> TestResult {
    let folder = tempfile::tempdir()?;
    let (store, data) = installation(folder.path()).await?;
    let exported = folder.path().join("exported");
    export_keys(&store, &exported).await?;
    // The same keys again: nothing to replace.
    import_keys(&store, &exported).await?;
    // Other keys are never put over the keys this installation's data needs.
    let other = tempfile::tempdir()?;
    let (other_store, _) = installation(other.path()).await?;
    let others = other.path().join("exported");
    export_keys(&other_store, &others).await?;
    let before = std::fs::read(data.keys().join("master.key"))?;
    let error = import_keys(&store, &others)
        .await
        .err()
        .ok_or("a key the data needs was replaced")?;
    assert!(
        format!("{error:#}").contains("does not replace"),
        "{error:#}"
    );
    assert_eq!(std::fs::read(data.keys().join("master.key"))?, before);
    // Into a new data folder, the keys go in.
    let fresh = StoreArgs {
        data_dir: Some(folder.path().join("fresh").to_string_lossy().into_owned()),
        ..StoreArgs::default()
    };
    import_keys(&fresh, &exported).await?;
    assert_eq!(
        std::fs::read(folder.path().join("fresh").join("keys").join("master.key"))?,
        before
    );
    Ok(())
}

/// The key files of another installation, exported into a folder of `root`.
async fn foreign_keys(root: &Path) -> Result<PathBuf, bb_testing::Failure> {
    let (other_store, _) = installation(root).await?;
    let exported = root.join("exported");
    export_keys(&other_store, &exported).await?;
    Ok(exported)
}

/// The files of the keys folder, by name and bytes.
fn key_folder(data: &DataFolder) -> Result<Vec<(String, Vec<u8>)>, std::io::Error> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(data.keys())? {
        let entry = entry?;
        files.push((
            entry.file_name().to_string_lossy().into_owned(),
            std::fs::read(entry.path())?,
        ));
    }
    files.sort();
    Ok(files)
}

#[tokio::test]
async fn ops_07_keys_import_replaces_no_key_when_the_database_cannot_be_read() -> TestResult {
    let folder = tempfile::tempdir()?;
    let (store, data) = installation(folder.path()).await?;
    let other = tempfile::tempdir()?;
    let others = foreign_keys(other.path()).await?;
    let before = key_folder(&data)?;
    // A database that cannot be reached: whether the keys are needed cannot be told.
    let gone = folder.path().join("no-such-folder").join("gone.db");
    let down = StoreArgs {
        database_url: Some(format!("sqlite:{}?mode=ro", gone.display())),
        ..store.clone()
    };
    let error = import_keys(&down, &others)
        .await
        .err()
        .ok_or("a key was replaced while the database was down")?;
    assert!(
        format!("{error:#}").contains("could not be read"),
        "{error:#}"
    );
    assert_eq!(key_folder(&data)?, before);
    Ok(())
}

#[tokio::test]
async fn ops_07_keys_import_waits_for_the_server_to_stop() -> TestResult {
    let folder = tempfile::tempdir()?;
    let (store, data) = installation(folder.path()).await?;
    let exported = folder.path().join("exported");
    export_keys(&store, &exported).await?;
    let running = data.lock_server()?.ok_or("the folder was locked already")?;
    let error = import_keys(&store, &exported)
        .await
        .err()
        .ok_or("keys were imported beside a running server")?;
    assert!(format!("{error:#}").contains("Stop it"), "{error:#}");
    drop(running);
    Ok(())
}

#[tokio::test]
async fn ops_07_keys_import_into_a_new_folder_puts_every_key_in_place() -> TestResult {
    let folder = tempfile::tempdir()?;
    let other = tempfile::tempdir()?;
    let others = foreign_keys(other.path()).await?;
    let fresh = StoreArgs {
        data_dir: Some(folder.path().join("fresh").to_string_lossy().into_owned()),
        ..StoreArgs::default()
    };
    import_keys(&fresh, &others).await?;
    let data = DataFolder::existing(&folder.path().join("fresh")).ok_or("no data folder")?;
    let mut expected = Vec::new();
    for name in bb_store::KEY_BACKUP_FILES {
        expected.push((name.to_owned(), std::fs::read(others.join(name))?));
    }
    expected.sort();
    // Every key, and nothing staged left behind.
    assert_eq!(key_folder(&data)?, expected);
    Ok(())
}

#[cfg(windows)]
#[tokio::test]
async fn sec_16_the_master_key_has_its_own_access_list_on_windows() -> TestResult {
    let folder = tempfile::tempdir()?;
    let (_, data) = installation(folder.path()).await?;
    let output = std::process::Command::new(
        crate::system::system_tool("icacls.exe").map_err(|said| said.render())?,
    )
    .arg(data.keys().join("master.key"))
    .output()?;
    let list = String::from_utf8_lossy(&output.stdout);
    // Every entry is the file's own: none is inherited from the folder (`(I)`), and the
    // administrators of the computer (a SID that no language changes) may read it.
    assert!(output.status.success(), "{list}");
    assert!(!list.contains("(I)"), "{list}");
    let administrators = std::process::Command::new(
        crate::system::system_tool("icacls.exe").map_err(|said| said.render())?,
    )
    .arg(data.keys().join("master.key"))
    .arg("/findsid")
    .arg(format!("*{}", crate::data_folder::ADMINISTRATORS_SID))
    .output()?;
    assert!(administrators.status.success());
    assert!(
        String::from_utf8_lossy(&administrators.stdout).contains("master.key"),
        "the administrators are not on the list"
    );
    Ok(())
}

#[tokio::test]
async fn ops_07_a_failed_restore_puts_each_path_and_command_in_its_place() -> TestResult {
    let folder = tempfile::tempdir()?;
    let (store, _data) = installation(folder.path()).await?;
    let file = folder.path().join("chosen.bbak");
    let args = RestoreArgs {
        store,
        file: file.clone(),
        replay_list: None,
    };
    let chosen = file.display().to_string();
    for error in [StoreError::BackupNotSealed, StoreError::KeysDoNotOpen] {
        let failure = RestoreFailure {
            error,
            before: None,
        };
        let text = failed(&args, &failure).to_string();
        assert!(
            text.starts_with(&format!("The backup {chosen} was not restored")),
            "{text}"
        );
        let command = text.rsplit(": ").next().unwrap_or_default();
        assert!(command.contains("keys import"), "{text}");
    }
    let before = folder.path().join("before.bbak");
    let failure = RestoreFailure {
        error: StoreError::BackupNotSealed,
        before: Some(before.clone()),
    };
    let text = failed(&args, &failure).to_string();
    let reason = StoreError::BackupNotSealed.to_string();
    assert!(
        text.starts_with(&format!("The restore stopped half way: {reason}.")),
        "{text}"
    );
    assert!(
        text.contains(&format!("is in the backup {}.", before.display())),
        "{text}"
    );
    assert!(text.contains("run: ") && text.contains("restore"), "{text}");
    Ok(())
}

#[tokio::test]
async fn sec_16_a_restore_writes_the_replay_list_privately() -> TestResult {
    let folder = tempfile::tempdir()?;
    let (store, data) = installation(folder.path()).await?;
    backup(&backup_args(&store)).await?;
    let file = backups_in(&data.backups())?[0].1.clone();
    // A list that somebody opened to the other accounts of the computer.
    let list = data.replay_list();
    std::fs::create_dir_all(list.parent().ok_or("no folder")?)?;
    std::fs::write(
        &list,
        "2026-11-02T10:00:00Z switch-off 0195f7a0-0000-4000-8000-000000000001\n",
    )?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(&list, std::fs::Permissions::from_mode(0o644))?;
    }
    #[cfg(windows)]
    {
        let output = std::process::Command::new("icacls")
            .arg(&list)
            .args(["/grant", "*S-1-5-32-545:R"])
            .output()?;
        assert!(output.status.success());
    }
    restore(&RestoreArgs {
        store: store.clone(),
        file,
        replay_list: None,
    })
    .await?;
    assert!(std::fs::read_to_string(&list)?.contains("switch-off"));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        assert_eq!(std::fs::metadata(&list)?.permissions().mode() & 0o077, 0);
    }
    #[cfg(windows)]
    {
        let sddl = crate::data_folder::rights::access_list(&list).ok_or("no access list")?;
        assert_eq!(
            bb_core::access_list::open_to_many(&sddl),
            Some(false),
            "{sddl}"
        );
    }
    Ok(())
}

#[tokio::test]
async fn ops_07_the_server_makes_no_backup_into_a_share_of_another_computer() -> TestResult {
    use bb_app::backups::BackupFiles as _;
    let folder = tempfile::tempdir()?;
    let (store, data) = installation(folder.path()).await?;
    let settings = load_settings(store.config.as_deref(), &store.settings(), false)?;
    let database = connect(&settings, &data).await?;
    let keys = Arc::new(KeyRing::open(&data.keys(), OpenMode::ExistingOnly)?);
    let backups = crate::wiring::server_backups(database, data, keys);
    for share in ["\\\\fileserver\\backups", "//fileserver/backups"] {
        let refused = backups
            .create(
                Some(share.to_owned()),
                Purpose::Regular,
                Timestamp::now(),
                None,
            )
            .await
            .err()
            .ok_or("a backup went to a share")?;
        assert!(
            refused.to_string().contains("another computer"),
            "{refused}"
        );
    }
    // A folder of this computer is not taken for a share.
    assert!(!crate::system::on_another_computer(folder.path()));
    assert!(!crate::system::on_another_computer(
        &folder.path().join("not-there-yet")
    ));
    Ok(())
}

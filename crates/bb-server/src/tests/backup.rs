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

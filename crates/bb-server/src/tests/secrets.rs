//! The commands that look after the secrets: `security-log verify`, `rotate-key` and
//! `keys export` (M1-F19, M1-F20, M1-F22).

use std::path::Path;

use bb_core::sign_in::SecurityAction;
use bb_core::technical::TechnicalName;
use bb_store::NewSecurityLogEntry;
use bb_testing::{Failure, TestResult};
use jiff::Timestamp;

use super::*;
use crate::config::{Sources, resolve};
use crate::wiring::{Serving, wire};

/// An installation with a built-in database and a security log with two entries.
async fn installation(root: &Path) -> Result<(StoreArgs, DataFolder), Failure> {
    let data = DataFolder::prepare(&root.join("data"))?;
    let given = [
        ("port", "47200".to_owned()),
        ("data_dir", data.root().to_string_lossy().into_owned()),
    ];
    let settings = resolve(&Sources {
        file: None,
        env: &|_| None,
        cli: &given,
        base: root,
    })?;
    wire(&settings, &data, Serving::default()).await?;
    let store = wiring::open_store(&settings, &data).await?;
    for _ in 0..2 {
        store
            .database
            .append_security_log(
                &store.seal,
                &NewSecurityLogEntry {
                    time: Timestamp::now(),
                    person: None,
                    action: TechnicalName::new(SecurityAction::SignInFailed.code())?,
                    subject: None,
                    network: None,
                },
            )
            .await?;
    }
    let args = StoreArgs {
        data_dir: Some(data.root().to_string_lossy().into_owned()),
        ..StoreArgs::default()
    };
    Ok((args, data))
}

#[tokio::test]
async fn m1_f20_verify_passes_on_an_intact_log() -> TestResult {
    let folder = tempfile::tempdir()?;
    let (args, _) = installation(folder.path()).await?;
    verify_security_log(&args).await?;
    Ok(())
}

#[tokio::test]
async fn m1_f20_verify_fails_when_the_anchor_file_was_changed() -> TestResult {
    let folder = tempfile::tempdir()?;
    let (args, data) = installation(folder.path()).await?;
    // A line that does not carry the signature of the log key.
    std::fs::write(
        data.security_log_anchor(),
        "1 00 00
",
    )?;
    let error = verify_security_log(&args)
        .await
        .err()
        .ok_or("a forged anchor file passed")?;
    assert!(format!("{error:#}").contains("anchor file"), "{error:#}");
    Ok(())
}

#[tokio::test]
async fn m1_f20_a_log_without_an_anchor_is_a_warning_and_a_broken_one_is_an_error() -> TestResult {
    let state = LogState::Valid {
        entries: 2,
        head: Some((2, "abababababababab".to_owned())),
        anchored: false,
    };
    assert!(summarize(&state).is_ok(), "a missing anchor only warns");
    let broken = LogState::Broken {
        seq: 5,
        reason: LogBreak::ChangedContent,
    };
    let error = summarize(&broken).err().ok_or("a broken log passed")?;
    let text = format!("{error:#}");
    assert!(
        text.contains("entry 5") && text.contains("changed"),
        "{text}"
    );
    assert!(summarize(&LogState::Unreadable).is_err());
    Ok(())
}

#[tokio::test]
async fn sec_15_verify_names_an_anchor_that_is_ahead_of_the_database_and_says_what_to_check()
-> TestResult {
    let behind = LogState::Broken {
        seq: 5,
        reason: LogBreak::Behind,
    };
    let error = summarize(&behind).err().ok_or("a log that lags passed")?;
    let text = format!("{error:#}");
    assert!(
        text.contains("entry 5") && text.contains("older backup") && text.contains("anchor file"),
        "{text}"
    );
    Ok(())
}

#[tokio::test]
async fn m1_f19_the_commands_refuse_a_folder_that_holds_no_bugaboo_data() -> TestResult {
    let folder = tempfile::tempdir()?;
    let empty = folder.path().join("empty");
    std::fs::create_dir(&empty)?;
    for data_dir in [empty.clone(), folder.path().join("not-there")] {
        let args = StoreArgs {
            data_dir: Some(data_dir.to_string_lossy().into_owned()),
            ..StoreArgs::default()
        };
        let error = verify_security_log(&args)
            .await
            .err()
            .ok_or("the command went on")?;
        assert!(
            format!("{error:#}").contains("no Bugaboo data"),
            "{error:#}"
        );
    }
    assert_eq!(
        std::fs::read_dir(&empty)?.count(),
        0,
        "nothing was made in the folder"
    );
    Ok(())
}

#[tokio::test]
async fn m1_f22_the_key_files_are_copied_to_a_private_folder_and_never_over_a_copy() -> TestResult {
    let folder = tempfile::tempdir()?;
    let (args, data) = installation(folder.path()).await?;
    let target = folder.path().join("backup");
    export_keys(&args, &target).await?;
    for file in ["master.key", "log.key"] {
        assert_eq!(
            std::fs::read(target.join(file))?,
            std::fs::read(data.keys().join(file))?,
            "{file}"
        );
    }
    assert!(
        !target.join("csrf.key").exists(),
        "the CSRF key protects nothing stored"
    );
    // A second export into the same folder is refused, and the first copy stays as it is.
    let before = std::fs::read(target.join("master.key"))?;
    let error = export_keys(&args, &target)
        .await
        .err()
        .ok_or("a copy was overwritten")?;
    assert!(format!("{error:#}").contains("already holds"), "{error:#}");
    assert_eq!(std::fs::read(target.join("master.key"))?, before);
    Ok(())
}

#[tokio::test]
async fn m1_f19_a_new_master_key_still_reads_everything() -> TestResult {
    let folder = tempfile::tempdir()?;
    let (args, data) = installation(folder.path()).await?;
    let before = std::fs::read(data.keys().join("master.key"))?;
    rotate_key(&args).await?;
    assert_ne!(std::fs::read(data.keys().join("master.key"))?, before);
    // The installation still opens and its log still verifies with the new key file.
    verify_security_log(&args).await?;
    Ok(())
}

#[tokio::test]
async fn sec_16_the_commands_refuse_key_files_that_this_account_cannot_change() -> TestResult {
    let folder = tempfile::tempdir()?;
    let (args, data) = installation(folder.path()).await?;
    assert_eq!(unusable_key(&data.keys()), None);
    let key = data.keys().join("master.key");
    let original = std::fs::metadata(&key)?.permissions();
    let mut read_only = original.clone();
    read_only.set_readonly(true);
    std::fs::set_permissions(&key, read_only)?;
    let found = unusable_key(&data.keys());
    let refused = rotate_key(&args).await.err().ok_or("rotate-key went on")?;
    std::fs::set_permissions(&key, original)?;
    assert_eq!(found, Some(key));
    let text = format!("{refused:#}");
    assert!(text.contains("cannot read and change"), "{text}");
    assert!(text.contains("rotate-key"), "{text}");
    Ok(())
}

#[tokio::test]
async fn sec_16_an_export_that_meets_an_old_copy_leaves_no_half_copy() -> TestResult {
    let folder = tempfile::tempdir()?;
    let (args, _) = installation(folder.path()).await?;
    let target = folder.path().join("backup");
    std::fs::create_dir(&target)?;
    // The second file of the export is there already: the first must not stay behind.
    std::fs::write(target.join("log.key"), "an older copy")?;
    let error = export_keys(&args, &target)
        .await
        .err()
        .ok_or("a copy was overwritten")?;
    assert!(format!("{error:#}").contains("already holds"), "{error:#}");
    assert!(!target.join("master.key").exists(), "a half copy stayed");
    assert_eq!(
        std::fs::read_to_string(target.join("log.key"))?,
        "an older copy"
    );
    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn sec_16_exported_keys_are_private_and_a_folder_others_can_open_is_refused() -> TestResult {
    use std::os::unix::fs::PermissionsExt as _;
    let folder = tempfile::tempdir()?;
    let (args, _) = installation(folder.path()).await?;
    let mode = |path: &Path| std::fs::metadata(path).map(|data| data.permissions().mode() & 0o777);
    let target = folder.path().join("backup");
    export_keys(&args, &target).await?;
    assert_eq!(mode(&target)?, 0o700);
    assert_eq!(mode(&target.join("master.key"))?, 0o600);
    let open = folder.path().join("open");
    std::fs::create_dir(&open)?;
    std::fs::set_permissions(&open, std::fs::Permissions::from_mode(0o755))?;
    let error = export_keys(&args, &open)
        .await
        .err()
        .ok_or("an open folder was used")?;
    assert!(
        format!("{error:#}").contains("can be opened by other accounts"),
        "{error:#}"
    );
    assert_eq!(std::fs::read_dir(&open)?.count(), 0);
    Ok(())
}

#[cfg(windows)]
#[tokio::test]
async fn sec_16_exported_keys_are_for_the_owner_and_the_administrators_on_windows() -> TestResult {
    let folder = tempfile::tempdir()?;
    let (args, _) = installation(folder.path()).await?;
    let target = folder.path().join("backup");
    export_keys(&args, &target).await?;
    let output = std::process::Command::new("icacls")
        .arg(target.join("master.key"))
        .output()?;
    let listing = String::from_utf8_lossy(&output.stdout).into_owned();
    assert_eq!(listing.matches(":(").count(), 2, "{listing}");
    assert!(!listing.contains("(I)"), "{listing}");
    Ok(())
}

#[test]
fn sec_16_the_hint_repeats_the_arguments_that_were_given() {
    let none = StoreArgs::default();
    let placeholder = command_line("rotate-key", &none, None);
    assert_eq!(
        placeholder,
        "bugaboo rotate-key --config <the settings file of the service>"
    );
    let config = StoreArgs {
        config: Some(Path::new("/etc/bugaboo/bugaboo.toml").to_path_buf()),
        ..StoreArgs::default()
    };
    assert_eq!(
        command_line("security-log verify", &config, None),
        "bugaboo security-log verify --config /etc/bugaboo/bugaboo.toml"
    );
    // The data folder that was named is named again, and a database address never is: it can
    // hold a password.
    let data = StoreArgs {
        data_dir: Some("/var/lib/bugaboo".to_owned()),
        database_url: Some("postgres://bugaboo:secret@db/bugaboo".to_owned()),
        ..StoreArgs::default()
    };
    let hint = command_line("rotate-key", &data, None);
    assert_eq!(
        hint,
        "bugaboo rotate-key --data-dir /var/lib/bugaboo --database-url <the address of your database>"
    );
    assert!(!hint.contains("secret"), "{hint}");
    // A database address is not repeated; a placeholder says that one was used.
    let postgres = StoreArgs {
        config: Some(Path::new("/etc/bugaboo/bugaboo.toml").to_path_buf()),
        database_url: Some("postgres://bugaboo:secret@db/bugaboo".to_owned()),
        ..StoreArgs::default()
    };
    let hint = command_line("rotate-key", &postgres, None);
    assert_eq!(
        hint,
        "bugaboo rotate-key --config /etc/bugaboo/bugaboo.toml --database-url <the address of your database>"
    );
    assert!(!hint.contains("secret"), "{hint}");
    // The target folder of an export is part of the command, quoted when it has a space.
    assert_eq!(
        command_line("keys export", &config, Some(Path::new("/srv/key backup"))),
        "bugaboo keys export \"/srv/key backup\" --config /etc/bugaboo/bugaboo.toml"
    );
}

#[test]
fn sec_16_an_export_that_fails_on_a_file_leaves_no_copy_behind() -> TestResult {
    let folder = tempfile::tempdir()?;
    let first = folder.path().join("master.key");
    std::fs::write(&first, "0 aa")?;
    // The second key file cannot be read: the copy of the first is taken back, and nothing that
    // was being written stays either.
    let missing = folder.path().join("log.key");
    let target = folder.path().join("backup");
    let error = copy_keys(&[first, missing], &target)
        .err()
        .ok_or("the export went on")?;
    assert!(
        format!("{error:#}").contains("could not be copied"),
        "{error:#}"
    );
    assert_eq!(
        std::fs::read_dir(&target)?.count(),
        0,
        "a copy stayed behind"
    );
    Ok(())
}

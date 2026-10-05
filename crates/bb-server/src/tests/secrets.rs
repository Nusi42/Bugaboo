//! The commands that look after the secrets: `security-log verify`, `rotate-key` and
//! `keys export` (M1-F19, M1-F20, M1-F22).

use std::path::Path;

use bb_core::sign_in::SecurityAction;
use bb_core::technical::TechnicalName;
use bb_store::NewSecurityLogEntry;
use bb_testing::TestResult;
use jiff::Timestamp;

use super::*;
use crate::config::{Sources, resolve};
use crate::wiring::wire;

type Failure = Box<dyn std::error::Error + Send + Sync>;

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
    wire(&settings, &data).await?;
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

//! The commands that look after the secrets of an installation: checking the security log
//! (`security-log verify`, M1-F19, M1-F20), making a new master key (`rotate-key`) and copying
//! the key files for a backup (`keys export`, M1-F22). They work on the stored data and never
//! listen, so they need no port.

use std::path::Path;

use anyhow::{Context as _, anyhow};
use bb_app::status::{LogBreak, LogState};
use bb_core::message::{self, Message, MessageKey};
use jiff::Timestamp;

use super::{StoreArgs, load_settings};
use crate::data_folder::{DataFolder, create_private};
use crate::probe::log_state;
use crate::wiring::{self, Store};
use crate::{config::Settings, output};

/// The installation the command works on, found from the settings, and its opened store.
async fn open(args: &StoreArgs) -> anyhow::Result<(Settings, Store)> {
    let settings = load_settings(args.config.as_deref(), &args.settings(), false)?;
    // A built-in database that is not there means the data folder is the wrong one: opening it
    // would make a new, empty database.
    let data = DataFolder::existing(&settings.data_dir)
        .filter(|data| settings.database_url.is_some() || data.database_file().is_file())
        .ok_or_else(|| anyhow!(message::cli_no_data(settings.data_dir.display()).render()))?;
    let store = wiring::open_store(&settings, &data).await?;
    Ok((settings, store))
}

/// The sentence for why a chain breaks.
const fn reason_key(reason: LogBreak) -> MessageKey {
    match reason {
        LogBreak::Gap => message::SECURITY_LOG_REASON_GAP,
        LogBreak::BrokenLink => message::SECURITY_LOG_REASON_BROKEN_LINK,
        LogBreak::ChangedContent => message::SECURITY_LOG_REASON_CHANGED_CONTENT,
        LogBreak::Checkpoint => message::SECURITY_LOG_REASON_CHECKPOINT,
        LogBreak::Truncated => message::SECURITY_LOG_REASON_TRUNCATED,
        LogBreak::Anchor => message::SECURITY_LOG_REASON_ANCHOR,
    }
}

/// `bugaboo security-log verify`: checks the chain, the checkpoint and the anchor file, and says
/// what the last entry is, so the person can keep a copy of it to compare later. A log with
/// entries and no anchor is a warning (nothing vouches for its end); a broken chain is an error.
pub(super) async fn verify_security_log(args: &StoreArgs) -> anyhow::Result<()> {
    let (_, store) = open(args).await?;
    let result = store.database.verify_security_log(&store.seal).await;
    if let Err(error) = &result {
        return Err(anyhow!("{error}"))
            .context(Message::from(message::SECURITY_LOG_UNREADABLE).render());
    }
    summarize(&log_state(&result))
}

/// Says what checking the log found; an error when the chain is broken.
fn summarize(state: &LogState) -> anyhow::Result<()> {
    match state {
        LogState::Valid {
            entries,
            head,
            anchored,
        } => {
            output::line(
                &message::security_log_valid(i64::try_from(*entries).unwrap_or(i64::MAX)).render(),
            );
            if let Some((seq, hash)) = head {
                output::line(&message::security_log_head(hash, seq).render());
            }
            if *entries > 0 && !anchored {
                output::warning(&Message::from(message::SECURITY_LOG_UNANCHORED).render());
            }
            Ok(())
        }
        LogState::Broken { seq, reason } => Err(anyhow!(
            message::security_log_broken(Message::from(reason_key(*reason)).render(), seq).render()
        )),
        LogState::Unreadable => Err(anyhow!(
            Message::from(message::SECURITY_LOG_UNREADABLE).render()
        )),
    }
}

/// `bugaboo rotate-key`: adds a new master key, wraps every stored secret with it and drops the
/// old keys that nothing uses any more.
pub(super) async fn rotate_key(args: &StoreArgs) -> anyhow::Result<()> {
    let (_, store) = open(args).await?;
    let secrets = store
        .database
        .rotate_master_key(&store.keys, Timestamp::now())
        .await?;
    output::line(&message::keys_rotated(i64::try_from(secrets).unwrap_or(i64::MAX)).render());
    Ok(())
}

/// `bugaboo keys export <folder>`: copies the key files into a folder that only its owner can
/// open, and never over a copy that is already there.
pub(super) async fn export_keys(args: &StoreArgs, folder: &Path) -> anyhow::Result<()> {
    let (_, store) = open(args).await?;
    copy_keys(&store.keys.backup_files(), folder)?;
    output::line(&message::keys_exported(folder.display()).render());
    Ok(())
}

fn copy_keys(files: &[std::path::PathBuf], folder: &Path) -> anyhow::Result<()> {
    let failed =
        |reason: &dyn std::fmt::Display| anyhow!(message::keys_export_failed(reason).render());
    create_private(folder).map_err(|error| failed(&error))?;
    let targets: Vec<_> = files
        .iter()
        .filter_map(|file| file.file_name().map(|name| folder.join(name)))
        .collect();
    // Refused before anything is copied, so a half copy never replaces a good one.
    if targets.iter().any(|target| target.exists()) {
        return Err(anyhow!(
            message::keys_export_exists(folder.display()).render()
        ));
    }
    for (file, target) in files.iter().zip(&targets) {
        std::fs::copy(file, target).map_err(|error| failed(&error))?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "../tests/secrets.rs"]
mod tests;

//! The commands that look after the secrets of an installation: checking the security log
//! (`security-log verify`, M1-F19, M1-F20), making a new master key (`rotate-key`) and copying
//! the key files for a backup (`keys export`, M1-F22). They work on the stored data and never
//! listen, so they need no port.

use std::path::{Path, PathBuf};

use anyhow::{Context as _, anyhow};
use bb_app::status::{LogBreak, LogState};
use bb_core::message::{self, Message, MessageKey};
use jiff::Timestamp;

use super::service::account_name;
use super::{StoreArgs, load_settings};
use crate::data_folder::{
    DataFolder, Existing, create_private, open_to_others, other_owner, write_private,
};
use crate::probe::log_state;
use crate::wiring::{self, Store};
use crate::{config::Settings, output};

/// The first key file in `keys` that this account cannot read and change, if there is one.
/// Opening a store may write there, so a command needs both.
fn unusable_key(keys: &Path) -> Option<PathBuf> {
    std::fs::read_dir(keys)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .find(|path| {
            std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(path)
                .is_err()
        })
}

/// A word of a command line, in quotes when it has a space.
fn word(text: &str) -> String {
    if text.contains(char::is_whitespace) {
        format!("\"{text}\"")
    } else {
        text.to_owned()
    }
}

/// The command a person types to do again what they typed, as the right account: the arguments
/// that were given (the settings file and the data folder, and the `folder` of an export). A
/// database address is left out, because it can hold a password. When nothing names the
/// installation, the words say that the settings file of the service is meant.
fn command_line(command: &str, args: &StoreArgs, folder: Option<&Path>) -> String {
    let mut line = format!("bugaboo {command}");
    if let Some(folder) = folder {
        line.push(' ');
        line.push_str(&word(&folder.display().to_string()));
    }
    let mut named = false;
    if let Some(config) = &args.config {
        line.push_str(" --config ");
        line.push_str(&word(&config.display().to_string()));
        named = true;
    }
    if let Some(data_dir) = &args.data_dir {
        line.push_str(" --data-dir ");
        line.push_str(&word(data_dir));
        named = true;
    }
    if !named {
        line.push(' ');
        line.push_str(&Message::from(message::KEYS_SETTINGS_PLACEHOLDER).render());
    }
    // Only that an address was given, never the address: it can hold a password.
    if args.database_url.is_some() {
        line.push(' ');
        line.push_str(&Message::from(message::KEYS_DATABASE_PLACEHOLDER).render());
    }
    line
}

/// What a person types to run `command` as `owner`, in the way of this system.
fn how_to_run(command: &str, owner: &str) -> Message {
    #[cfg(windows)]
    {
        message::keys_how_windows(command, owner)
    }
    #[cfg(not(windows))]
    {
        message::keys_how_linux(command, owner)
    }
}

/// Refuses to touch an installation that another account looks after, which is what a service
/// install is: a command run from the wrong account would leave files that the service cannot
/// read, or could not read the keys at all, and says how to run it as the right account.
fn require_own_installation(
    data: &DataFolder,
    args: &StoreArgs,
    command: &str,
    folder: Option<&Path>,
) -> anyhow::Result<()> {
    let command = command_line(command, args, folder);
    if let Some(owner) = other_owner(data.root()) {
        let how = how_to_run(&command, &owner).render();
        return Err(anyhow!(
            message::keys_not_owner(how, owner, data.root().display()).render()
        ));
    }
    if unusable_key(&data.keys()).is_some() {
        let how = how_to_run(&command, &account_name()).render();
        return Err(anyhow!(
            message::keys_no_access(how, data.keys().display()).render()
        ));
    }
    Ok(())
}

/// The installation the command works on, found from the settings, and its opened store.
async fn open(
    args: &StoreArgs,
    command: &str,
    folder: Option<&Path>,
) -> anyhow::Result<(Settings, Store)> {
    let settings = load_settings(args.config.as_deref(), &args.settings(), false)?;
    // A built-in database that is not there means the data folder is the wrong one: opening it
    // would make a new, empty database.
    let data = DataFolder::existing(&settings.data_dir)
        .filter(|data| settings.database_url.is_some() || data.database_file().is_file())
        .ok_or_else(|| anyhow!(message::cli_no_data(settings.data_dir.display()).render()))?;
    require_own_installation(&data, args, command, folder)?;
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
    let (_, store) = open(args, "security-log verify", None).await?;
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
    let (_, store) = open(args, "rotate-key", None).await?;
    let secrets = store
        .database
        .rotate_master_key(&store.keys, Timestamp::now())
        .await?;
    output::line(&message::keys_rotated(i64::try_from(secrets).unwrap_or(i64::MAX)).render());
    Ok(())
}

/// `bugaboo keys export <folder>`: copies the key files into a folder that only its owner can
/// open (a folder that others can open is refused). Every copy is a new file that is made
/// private before the key goes in; a copy that is already there is never replaced, and a run that
/// meets one takes back what it made.
pub(super) async fn export_keys(args: &StoreArgs, folder: &Path) -> anyhow::Result<()> {
    let (_, store) = open(args, "keys export", Some(folder)).await?;
    copy_keys(&store.keys.backup_files(), folder)?;
    output::line(&message::keys_exported(folder.display()).render());
    Ok(())
}

fn copy_keys(files: &[PathBuf], folder: &Path) -> anyhow::Result<()> {
    let failed =
        |reason: &dyn std::fmt::Display| anyhow!(message::keys_export_failed(reason).render());
    create_private(folder).map_err(|error| failed(&error))?;
    if open_to_others(folder) {
        return Err(anyhow!(
            message::keys_export_not_private(folder.display()).render()
        ));
    }
    let mut written: Vec<PathBuf> = Vec::new();
    for file in files {
        let Some(name) = file.file_name() else {
            continue;
        };
        let target = folder.join(name);
        let copied =
            std::fs::read(file).and_then(|bytes| write_private(&target, &bytes, Existing::Refuse));
        match copied {
            Ok(()) => written.push(target),
            Err(error) => {
                // A half copy never stays: what this run made is taken away again.
                for done in &written {
                    let _ = std::fs::remove_file(done);
                }
                return Err(if error.kind() == std::io::ErrorKind::AlreadyExists {
                    anyhow!(message::keys_export_exists(folder.display()).render())
                } else {
                    failed(&error)
                });
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "../tests/secrets.rs"]
mod tests;

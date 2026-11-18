//! The commands of backups (OPS-07, ADR-0046): `bugaboo backup` creates one while the server may
//! run, `bugaboo backup list` lists them, and `bugaboo restore <file>` puts one back with the
//! server stopped, as the account that owns the data folder.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::anyhow;
use bb_app::context::SystemIds;
use bb_core::backup::manifest::BackupProblem;
use bb_core::backup::name::Purpose;
use bb_core::backup::run::{BackupEvent, BackupMaker, BackupRun};
use bb_core::event::{Actor, Channel};
use bb_core::id::{BackupId, EventId};
use bb_core::message::{self, Message};
use bb_store::backup::{
    BackupPlaces, RestoreFailure, RestoreInput, RestoreReport, RestoreTarget, backups_in,
};
use bb_store::{KeyRing, LogSeal, OpenMode, StoreError};
use clap::{Args, Subcommand};
use jiff::Timestamp;

use super::secrets::{command_line, open, require_own_installation, summarize};
use super::{StoreArgs, load_settings, text};
use crate::data_folder::{DataFolder, create_private, open_to_others};
use crate::output;
use crate::probe::log_state;

/// What `bugaboo backup` is told.
#[derive(Args, Clone, Default)]
pub(crate) struct BackupArgs {
    #[command(flatten)]
    store: StoreArgs,
    #[arg(long, help = text(message::CLI_HELP_BACKUP_TO))]
    to: Option<PathBuf>,
    #[command(subcommand)]
    command: Option<BackupCommand>,
}

#[derive(Subcommand, Clone)]
enum BackupCommand {
    #[command(about = text(message::CLI_ABOUT_BACKUP_LIST))]
    List,
}

/// What `bugaboo restore` is told.
#[derive(Args, Clone, Default)]
pub(crate) struct RestoreArgs {
    #[command(flatten)]
    store: StoreArgs,
    #[arg(help = text(message::CLI_HELP_RESTORE_FILE))]
    file: PathBuf,
    #[arg(long, help = text(message::CLI_HELP_REPLAY_LIST))]
    replay_list: Option<PathBuf>,
}

/// Where the backups of an installation go and what they copy besides the database.
pub(crate) fn places(data: &DataFolder, folder: PathBuf) -> BackupPlaces {
    BackupPlaces {
        folder,
        files: data.files(),
        replay_list: data.replay_list(),
        file_paths: Arc::new(bb_files::stored_paths),
    }
}

/// The folder of the backups: the one given, made for its owner only and refused when others can
/// open it, or the backups folder of the data folder.
pub(crate) fn backup_folder(data: &DataFolder, to: Option<&Path>) -> anyhow::Result<PathBuf> {
    let Some(folder) = to else {
        return Ok(data.backups());
    };
    create_private(folder).map_err(|error| anyhow!(error.message()))?;
    if open_to_others(folder) {
        return Err(anyhow!(
            message::backups_folder_not_private(folder.display()).render()
        ));
    }
    Ok(folder.to_path_buf())
}

/// `bugaboo backup` and `bugaboo backup list`.
pub(super) async fn backup(args: &BackupArgs) -> anyhow::Result<()> {
    let (settings, store) = open(&args.store, "backup", None).await?;
    let data = DataFolder::existing(&settings.data_dir)
        .ok_or_else(|| anyhow!(message::cli_no_data(settings.data_dir.display()).render()))?;
    let folder = backup_folder(&data, args.to.as_deref())?;
    if matches!(args.command, Some(BackupCommand::List)) {
        let found = backups_in(&folder)?;
        if found.is_empty() {
            output::line(&message::backups_none(folder.display()).render());
        }
        for (name, _, size) in found {
            output::line(&message::backups_listed(name.to_string(), size_of(size)).render());
        }
        return Ok(());
    }
    let now = Timestamp::now();
    let created = store
        .database
        .create_backup(&places(&data, folder), Purpose::Regular, now)
        .await
        .map_err(|error| anyhow!(message::backups_failed(error).render()))?;
    let run = BackupRun {
        id: BackupId::generate(&mut SystemIds)?,
        event: BackupEvent::Made,
        name: created.name.clone(),
        size: size_of(created.size),
        checksum: created.checksum,
        maker: BackupMaker::CommandLine,
        person: None,
        at: now,
        lost_from: None,
        lost_to: None,
    };
    let event = run.domain_event(
        EventId::generate(&mut SystemIds)?,
        Actor::System,
        Channel::System,
    )?;
    store
        .database
        .record_backup(&store.seal, &run, &event)
        .await?;
    output::line(
        &message::backups_created(
            created.name.to_string(),
            created.path.display(),
            size_of(created.size),
        )
        .render(),
    );
    Ok(())
}

fn size_of(bytes: u64) -> i64 {
    i64::try_from(bytes).unwrap_or(i64::MAX)
}

/// The database a restore goes into, from the settings.
fn target(database_url: Option<&str>, data: &DataFolder) -> RestoreTarget {
    match database_url {
        Some(url) if url.starts_with("postgres") => RestoreTarget::Postgres(url.to_owned()),
        Some(url) => RestoreTarget::File(PathBuf::from(
            url.trim_start_matches("sqlite:").trim_start_matches("//"),
        )),
        None => RestoreTarget::File(data.database_file()),
    }
}

/// `bugaboo restore <file>`: refuses a running server and another account, then restores.
pub(super) async fn restore(args: &RestoreArgs) -> anyhow::Result<()> {
    let settings = load_settings(args.store.config.as_deref(), &args.store.settings(), false)?;
    let data = DataFolder::existing(&settings.data_dir)
        .ok_or_else(|| anyhow!(message::cli_no_data(settings.data_dir.display()).render()))?;
    require_own_installation(&data, &args.store, "restore", Some(&args.file))?;
    let Some(_stopped) = data.lock_server()? else {
        return Err(anyhow!(
            message::restore_server_running(data.root().display()).render()
        ));
    };
    let keys = KeyRing::open(&data.keys(), OpenMode::ExistingOnly)?;
    let seal = LogSeal::new(&keys, data.security_log_anchor());
    let more: Vec<PathBuf> = args.replay_list.iter().cloned().collect();
    let result = bb_store::backup::restore(
        RestoreInput {
            archive: &args.file,
            target: &target(settings.database_url.as_deref(), &data),
            places: &places(&data, data.backups()),
            keys: &keys,
            seal: &seal,
            more_replay_lists: &more,
            now: Timestamp::now(),
        },
        &mut SystemIds,
    )
    .await;
    match result {
        Ok(report) => {
            say(&report);
            Ok(())
        }
        Err(failure) => Err(failed(args, &failure)),
    }
}

/// What the person reads after a restore.
fn say(report: &RestoreReport) {
    output::line(&message::restore_done(report.moment).render());
    output::line(&message::restore_before(report.before.path.display()).render());
    if !report.migrations_run.is_empty() {
        output::line(
            &message::restore_migrations(size_of(report.migrations_run.len() as u64)).render(),
        );
    }
    output::line(&message::restore_signed_out(size_of(report.sessions_ended)).render());
    for line in &report.replayed {
        output::line(&message::restore_replayed(line).render());
    }
    for line in &report.not_replayed {
        output::warning(&message::restore_not_replayed(line).render());
    }
    if report.unreadable_lines > 0 {
        output::warning(
            &message::restore_unreadable_lines(size_of(report.unreadable_lines as u64)).render(),
        );
    }
    if let Some((from, to)) = report.lost {
        output::line(&message::restore_lost(from, to).render());
    }
    if let Err(error) = summarize(&log_state(&Ok(report.log))) {
        output::warning(&format!("{error:#}"));
    }
}

/// The words for a backup that was refused.
fn reason(problem: &BackupProblem) -> String {
    match problem {
        BackupProblem::Unreadable => Message::from(message::RESTORE_REASON_UNREADABLE).render(),
        BackupProblem::NewerFormat(_) => {
            Message::from(message::RESTORE_REASON_NEWER_FORMAT).render()
        }
        BackupProblem::BadName(name, _) => message::restore_reason_bad_name(name).render(),
        BackupProblem::Missing(name) => message::restore_reason_missing(name).render(),
        BackupProblem::Extra(name) => message::restore_reason_extra(name).render(),
        BackupProblem::Damaged(name) => message::restore_reason_damaged(name).render(),
        BackupProblem::NewerMigration(version) => {
            message::restore_reason_newer_migration(*version).render()
        }
        BackupProblem::ChangedMigration(version) => {
            message::restore_reason_changed_migration(*version).render()
        }
        BackupProblem::OtherDatabase(kind) => {
            message::restore_reason_other_database(format!("{kind:?}")).render()
        }
    }
}

/// The error of a restore that failed, with what to do next.
fn failed(args: &RestoreArgs, failure: &RestoreFailure) -> anyhow::Error {
    let path = args.file.display();
    if let Some(before) = &failure.before {
        let again = command_line("restore", &args.store, Some(before));
        return anyhow!(message::restore_failed(&failure.error, before.display(), again).render());
    }
    match &failure.error {
        StoreError::BackupRefused(problem) => {
            anyhow!(message::restore_refused(path, reason(problem)).render())
        }
        StoreError::KeysDoNotOpen => {
            let import = command_line(
                "keys import",
                &args.store,
                Some(Path::new(
                    &Message::from(message::KEYS_FOLDER_PLACEHOLDER).render(),
                )),
            );
            anyhow!(message::restore_keys_do_not_open(path, import).render())
        }
        other => anyhow!("{other}"),
    }
}

#[cfg(test)]
#[path = "../tests/backup.rs"]
mod tests;

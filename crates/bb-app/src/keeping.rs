//! Keeping and deleting (PRIV-05, ADR-0045, ADR-0017): every job that deletes what outlived its
//! keep period runs through one shape. A round of such a job is one step: each part deletes at most
//! [`KEEP_STEP_ROWS`] rows in one short write, so a write of people never waits long behind it
//! (M4-F104), and when a part reached its limit the job queues its next step after
//! [`KEEP_STEP_PAUSE`]. Each job is safe to run again. [`JobKind::keeps`] says which rows of
//! `bb_core::keeping::KEEP_KINDS` each job deletes; a row of the group *by an administrator* has no
//! job.
//!
//! - `sessions.purge`: the sessions nobody can use any more, the links of finished sign-ups, the
//!   links to new e-mail addresses that ran out, and the IP addresses of old sessions.
//! - `security-log.prune`: the oldest whole blocks of the security log past its keep setting, and
//!   the log of tool calls with it.
//! - `jobs.purge-dead`: the jobs that gave up, with their payload (M1-F21).
//! - `privacy.delete-old-exports`: the files of *Download my data* past their keep period, each
//!   file first, then its record.
//! - `server-log.rotate`: a new file of the log of a server that runs as a service; it keeps a
//!   number of files, not a period, so it has no row.

mod old_records;
mod rules;

pub use old_records::{
    DeleteOldRecords, DeletedRecords, OLD_PEOPLE_MAX, OldCount, OldOfPerson, OldRecordStore,
    OldRecords, OldRecordsQuery, delete_old_records, old_records,
};
pub use rules::{KeepRuleView, KeepRules, KeepRulesQuery, keep_rules};

use bb_core::event::{Actor as EventActor, RecordKind};
use bb_core::id::JobId;
use bb_core::keeping::{KeepKind, KeptUntil, Step, rule};
use bb_core::settings::{LOGS_SERVER_LOG_FILES, LOGS_SERVER_LOG_MEGABYTES, Layers};
use bb_core::wording::Region;
use jiff::{SignedDuration, Timestamp};
use serde_json::Value;

use crate::auth::{
    AppError, AuthStore, Ctx, Stops, change_failed, changed, fixed_entity, internal,
};
use crate::editing::{PurgeDraftsQuery, purge_drafts};
use crate::jobs::JobKind;
use crate::ports::KeepPeriods;

/// The most rows one part of a step of a deleting job deletes in one write (ADR-0045, decision 5).
/// Kept small, so the write is short: a thousand old notifications of the sample organization held
/// SQLite's write lock for 70 to 250 ms, a hundred for under 10 ms.
pub const KEEP_STEP_ROWS: u64 = 100;

/// How long a deleting job waits before its next step: a large backlog (the first run after an
/// upgrade or an import) is worked off beside the writes of people, not ahead of them.
pub const KEEP_STEP_PAUSE: SignedDuration = SignedDuration::from_secs(3);

/// The bytes in a megabyte, for the size of the server log.
const MEGABYTE: u64 = 1_048_576;

impl JobKind {
    /// The kinds of data the job deletes by itself, each a row of the group *by itself*; empty for
    /// a job that deletes nothing past a keep period. A `match` without a catch-all, so a new kind
    /// has to say (`priv_05_every_kind_of_data_has_one_row_and_one_group`).
    #[must_use]
    pub const fn keeps(self) -> &'static [KeepKind] {
        match self {
            Self::PurgeSessions => &[
                KeepKind::IpAddresses,
                KeepKind::Sessions,
                KeepKind::SignUpLinks,
                KeepKind::EmailChangeLinks,
                KeepKind::AgentRequests,
            ],
            Self::PurgeDrafts => &[KeepKind::Drafts],
            Self::PurgeNotifications => &[KeepKind::Notifications],
            Self::CleanUpFiles => &[KeepKind::Trash, KeepKind::Uploads],
            Self::PurgeDeadJobs => &[KeepKind::DeadJobs],
            Self::CleanOldGrants => &[KeepKind::AppGrants],
            Self::PruneSecurityLog => &[KeepKind::SecurityLog, KeepKind::ToolCalls],
            Self::DeleteOldAwayTimes => &[KeepKind::AwayTimes],
            Self::BackupsDaily => &[KeepKind::Backups],
            Self::DeleteOldExports => &[KeepKind::DataExports],
            Self::CatchUpHistory
            | Self::RefreshStatistics
            | Self::RotateServerLog
            | Self::ManyChangesStep
            | Self::RenewIds
            | Self::MailSend
            | Self::PreviewFile
            | Self::TimersLongRunning
            | Self::HandOverRunsOut
            | Self::RuleRunsOut
            | Self::RuleAboutToRunOut
            | Self::RuleMatches
            | Self::RulesGetAnEnd
            | Self::DataExport
            | Self::FinishRemoval => &[],
        }
    }
}

/// The stored settings answer the port: each kind's period is its row of `KEEP_KINDS`.
impl KeepPeriods for Layers {
    fn kept_until(&self, kind: KeepKind, date: jiff::civil::Date) -> KeptUntil {
        rule(kind).kept_until(self, date)
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The keep periods of the organization, for whoever asks how long a record is kept
    /// ([`KeepPeriods`]).
    ///
    /// # Errors
    /// The error of the store when the settings cannot be read.
    pub async fn keep_periods(&self) -> Result<impl KeepPeriods + use<S>, AppError> {
        self.setting_layers_of(None).await
    }

    /// The moment before which data of `kind` is past its keep period now, with the organization's
    /// settings and zone.
    pub(crate) async fn keep_cutoff(&self, kind: KeepKind) -> Result<Timestamp, AppError> {
        let layers = self.setting_layers_of(None).await?;
        rule(kind)
            .cutoff(&layers, self.now(), Region::of(&layers).zone())
            .ok_or_else(|| AppError::Internal("a keep period that cannot be counted".to_owned()))
    }

    /// One step of the deleting job `kind`: the one shape of deleting by itself. When a part of
    /// the step reached its limit, the next step is queued for after [`KEEP_STEP_PAUSE`]; the
    /// next daily round waits already, so the step adds no second chain of rounds.
    pub(crate) async fn keep_step(&self, kind: JobKind) -> Result<Step, AppError> {
        let rows = |count: u64| Step::of(count, KEEP_STEP_ROWS);
        let step = match kind {
            JobKind::PurgeSessions => self.purge_dead_sessions().await?,
            JobKind::PurgeDrafts => purge_drafts(self, PurgeDraftsQuery).await?,
            JobKind::PurgeNotifications => rows(self.purge_old_notifications().await?),
            JobKind::CleanUpFiles => self.clean_up_files().await?.step(),
            JobKind::PurgeDeadJobs => rows(self.purge_old_dead_jobs().await?),
            JobKind::CleanOldGrants => {
                let cleaned = crate::auth::clean_old_grants(self).await?;
                rows(cleaned.secrets)
                    .and(rows(cleaned.grants))
                    .and(rows(cleaned.clients))
            }
            JobKind::PruneSecurityLog => {
                // The log of tool calls is pruned with the security log (ADR-0013).
                let entries = self.prune_old_security_log().await?;
                rows(entries).and(rows(self.prune_old_tool_calls().await?))
            }
            JobKind::DeleteOldAwayTimes => {
                rows(crate::work_times::delete_old_away_times(self).await?)
            }
            JobKind::DeleteOldExports => rows(self.delete_old_exports().await?),
            // The job of the schedule makes the backup of the day, then deletes through this step.
            // The rotation deletes all it no longer keeps at once, so it never asks for another
            // step: a queued `backups.daily` would make another backup.
            JobKind::BackupsDaily => Step {
                rows: u64::try_from(self.prune_backups().await?).unwrap_or(u64::MAX),
                more: false,
            },
            JobKind::CatchUpHistory
            | JobKind::RefreshStatistics
            | JobKind::RotateServerLog
            | JobKind::ManyChangesStep
            | JobKind::RenewIds
            | JobKind::MailSend
            | JobKind::PreviewFile
            | JobKind::TimersLongRunning
            | JobKind::HandOverRunsOut
            | JobKind::RuleRunsOut
            | JobKind::RuleAboutToRunOut
            | JobKind::RuleMatches
            | JobKind::RulesGetAnEnd
            | JobKind::DataExport
            | JobKind::FinishRemoval => {
                return Err(AppError::Internal(format!(
                    "the job kind {} deletes nothing past a keep period",
                    kind.code()
                )));
            }
        };
        if step.more {
            let id = JobId::generate(&mut self.ids())
                .map_err(|error| AppError::Internal(error.to_string()))?;
            let now = self.now();
            let after_the_pause = now
                .checked_add(KEEP_STEP_PAUSE)
                .map_err(|error| AppError::Internal(error.to_string()))?;
            self.store()
                .queue_job(id, &kind.name()?, &Value::Null, after_the_pause, now)
                .await
                .map_err(|error| internal::<S>(&error))?;
        }
        Ok(step)
    }

    /// Removes the sessions nobody can use any more, the links of sign-ups that were used or ran
    /// out, and the IP addresses of sessions past their keep period: one step. A change of Bugaboo
    /// itself, so it writes an event.
    pub(crate) async fn purge_dead_sessions(&self) -> Result<Step, AppError> {
        let links = self
            .delete_old_sign_up_links()
            .await?
            .and(self.delete_old_e_mail_changes().await?)
            .and(self.delete_old_agent_requests().await?);
        let settings = self.settings().await?;
        let networks_before = self.keep_cutoff(KeepKind::IpAddresses).await?;
        let event = self.event(
            EventActor::System,
            fixed_entity(RecordKind::Session, "purge")?,
            vec![changed("purged")],
        )?;
        let now = self.now();
        let sessions = self
            .store()
            .record_change(&event, move |transaction| {
                Box::pin(async move {
                    let sessions = crate::auth::purge_sessions::<S>(
                        transaction,
                        &settings,
                        now,
                        KEEP_STEP_ROWS,
                    )
                    .await?;
                    let networks =
                        S::forget_old_networks(transaction, networks_before, KEEP_STEP_ROWS)
                            .await?;
                    Ok(Step::of(sessions, KEEP_STEP_ROWS).and(Step::of(networks, KEEP_STEP_ROWS)))
                })
            })
            .await
            .map_err(|error| change_failed::<S>(&error, Stops::NONE))?;
        Ok(sessions.and(links))
    }

    /// Deletes the oldest whole blocks of the security log past its keep setting: one step.
    /// Returns how many entries went.
    pub(crate) async fn prune_old_security_log(&self) -> Result<u64, AppError> {
        let before = self.keep_cutoff(KeepKind::SecurityLog).await?;
        self.store()
            .prune_security_log(before, self.now(), KEEP_STEP_ROWS)
            .await
            .map_err(|error| internal::<S>(&error))
    }

    /// Deletes the jobs that gave up longer ago than their keep period, with their payload, which
    /// can hold the address and the text of a person (M1-F21): one step. Returns how many went.
    pub(crate) async fn purge_old_dead_jobs(&self) -> Result<u64, AppError> {
        let before = self.keep_cutoff(KeepKind::DeadJobs).await?;
        self.store()
            .purge_dead_jobs(before, KEEP_STEP_ROWS)
            .await
            .map_err(|error| internal::<S>(&error))
    }

    /// Starts a new file of the log of a server that runs without a console, when the file is
    /// big. Returns whether it did. Nothing happens on a server that logs to its console.
    pub(crate) async fn rotate_old_server_log(&self) -> Result<bool, AppError> {
        let Some(file) = &self.services.server_log else {
            return Ok(false);
        };
        let layers = self.setting_layers_of(None).await?;
        let megabytes = layers.number(&LOGS_SERVER_LOG_MEGABYTES);
        let files = layers.number(&LOGS_SERVER_LOG_FILES);
        file.rotate(
            u64::try_from(megabytes)
                .unwrap_or(1)
                .saturating_mul(MEGABYTE),
            u32::try_from(files).unwrap_or(1),
        )
        .map_err(|error| AppError::Internal(error.to_string()))
    }
}

//! The part of the canonical dump that is about the people who left, the export and the backups
//! (M9 part 8): counts and the name of the person whose file is ready, nothing random by design (the
//! code of the tombstone, the sizes and checksums of the backups).
//!
//! TEST-SAMPLE-DATA holds these numbers against the plan in `tools/dev/sample.json`
//! (`bb_dev_words::sample::PrivacyPlan`).

use bb_app::backups::BackupStore as _;
use bb_app::people::StandingStore as _;
use bb_app::privacy::export::ExportStore as _;
use bb_app::working_hours::WorkingHoursStore as _;
use bb_core::backup::run::BackupEvent;
use bb_core::event::{Actor, DomainEvent, RecordKind};
use bb_core::person::Standing;
use bb_store::backup::backups_in;
use jiff::civil::Date;
use serde::Serialize;

use crate::error::SampleError;
use crate::generate::Sample;

/// More backup records than the sample makes: they are read in one go.
const RUNS: u64 = 100;

/// A removed person.
#[derive(Debug, Default, Serialize)]
struct DumpRemoved {
    count: u64,
    /// Whether the person has the code of the tombstone (*Former user K7Q2*).
    labelled: bool,
    /// Whether their name is kept, as it was, until a day: their work days are still kept.
    name_held_until_a_day: bool,
    work_days: u64,
    /// The comments they wrote, which stay as *Former user*.
    comments: u64,
}

/// The file of a person's data.
#[derive(Debug, Default, Serialize)]
struct DumpExport {
    person: String,
    state: String,
}

/// The people who left, the export and the backups.
#[derive(Debug, Default, Serialize)]
pub(crate) struct DumpPrivacy {
    switched_off: u64,
    removed: DumpRemoved,
    export: DumpExport,
    backups: u64,
    backups_recorded: u64,
}

impl Sample {
    /// Reads the people who left, the export and the backups.
    pub(crate) async fn privacy_of(
        &self,
        events: &[DomainEvent],
    ) -> Result<DumpPrivacy, SampleError> {
        let mut dump = DumpPrivacy::default();
        for made in &self.people {
            let Some(standing) = self.storage.standing(made.id).await? else {
                continue;
            };
            match standing.standing {
                Standing::SwitchedOff => dump.switched_off += 1,
                Standing::Removed => {
                    let removed = &mut dump.removed;
                    removed.count += 1;
                    removed.labelled = standing.label.is_some();
                    removed.name_held_until_a_day = standing.hold_name.as_deref()
                        == Some(made.person.name.as_str())
                        && standing.hold_until.is_some();
                    removed.work_days = self
                        .storage
                        .work_days(made.id, Date::MIN, Date::MAX)
                        .await?
                        .len() as u64;
                    removed.comments = events
                        .iter()
                        .filter(|event| {
                            event.entity.kind.as_str() == RecordKind::Comment.code()
                                && matches!(event.actor, Actor::Person(person) if person == made.id)
                        })
                        .count() as u64;
                }
                Standing::Active => {}
            }
        }
        let plan = bb_dev_words::sample::Layout::bundled()?.issues.work.privacy;
        if let Some(made) = self
            .people
            .iter()
            .find(|made| made.person.name == plan.export_of)
            && let Some(id) = self.storage.exports_of(made.id).await?.into_iter().last()
            && let Some(export) = self.storage.data_export(id).await?
        {
            dump.export = DumpExport {
                person: made.person.name.clone(),
                state: export.state.code().to_owned(),
            };
        }
        dump.backups = backups_in(&self.backups_folder())?.len() as u64;
        dump.backups_recorded = self
            .storage
            .backup_runs(RUNS)
            .await?
            .iter()
            .filter(|run| run.event == BackupEvent::Made)
            .count() as u64;
        Ok(dump)
    }
}

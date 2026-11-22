//! What TEST-SAMPLE-DATA holds the people who left, the export and the backups of the sample
//! against (M9 part 8, PRIV-03, PRIV-04, OPS-07): the part `issues.work.privacy` of the dump, judged
//! by [`PrivacyPlan::judge`] as part of [`WorkPlan::judge`](crate::sample::WorkPlan). One person is
//! switched off and one removed, whose comments stay and whose work days are kept under the
//! legal-hold name until a day; the file of the person the plan names is ready; the backups are
//! there with their records.

use serde_json::Value;

use crate::sample::PrivacyPlan;
use crate::sample_issues::Judge;

impl PrivacyPlan {
    /// Judges the part `privacy` of the dump.
    pub(crate) fn judge(&self, judge: &mut Judge<'_>, privacy: &Value) {
        let number = |value: &Value| value.as_u64().unwrap_or(0);
        let yes = |value: &Value| u64::from(value.as_bool() == Some(true));
        judge.exactly("people switched off", number(&privacy["switched_off"]), 1);
        let removed = &privacy["removed"];
        judge.exactly("people removed", number(&removed["count"]), 1);
        judge.present(
            "removed people with a tombstone code",
            yes(&removed["labelled"]),
        );
        judge.present(
            "removed people whose name is kept for their work days",
            yes(&removed["name_held_until_a_day"]),
        );
        judge.present(
            "work days of a removed person",
            number(&removed["work_days"]),
        );
        judge.present(
            "comments of a removed person kept as Former user",
            number(&removed["comments"]),
        );
        if privacy["export"]["person"] != self.export_of.as_str()
            || privacy["export"]["state"] != "ready"
        {
            judge.problem(&format!(
                "the file of {}'s data is not ready",
                self.export_of
            ));
        }
        judge.exactly(
            "backups",
            number(&privacy["backups"]),
            u64::from(self.backups),
        );
        judge.exactly(
            "records of backups made",
            number(&privacy["backups_recorded"]),
            u64::from(self.backups),
        );
    }
}
